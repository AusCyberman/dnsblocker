use std::{net::SocketAddr, str::FromStr, sync::Arc};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::{Duration, Utc};

use diesel::prelude::*;
use diesel::ExpressionMethods;
use diesel_async::pooled_connection::deadpool::{self, Pool};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use models::{Domain, EndDuration, Session, User};
use schema::sessions;
use serde::{Deserialize, Serialize};
use tokio::{net::UdpSocket, select};
use tokio_stream::StreamExt;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;
use trust_dns_resolver::error::ResolveErrorKind;
use trust_dns_resolver::{
    config::{NameServerConfig, ResolverConfig, ResolverOpts},
    name_server::{GenericConnector, TokioRuntimeProvider},
    proto::{
        op::{Header, MessageType, OpCode},
        rr::LowerName,
    },
    AsyncResolver, Name,
};
use trust_dns_server::{
    authority::MessageResponseBuilder,
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
    ServerFuture,
};
mod models;
mod schema;

pub struct Handler {
    resolver: AsyncResolver<GenericConnector<TokioRuntimeProvider>>,
    state: Arc<ServerState>,
}

impl Handler {
    async fn resolve<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> Result<(), ServerError> {
        let query = request.query();
        let name = query.name();
        let res = match self.resolver.lookup(name, query.query_type()).await {
            Err(e) => match e.kind() {
                ResolveErrorKind::NoRecordsFound { .. } => vec![],
                _ => return Err(ServerError::TrustDnsResolve(e)),
            },
            e => e?.records().to_vec(),
        };
        let mut header = Header::response_from_request(request.header());
        header.set_authoritative(false);
        let response = MessageResponseBuilder::from_message_request(request).build(
            header,
            &res,
            std::iter::empty(),
            std::iter::empty(),
            std::iter::empty(),
        );
        response_handle.send_response(response).await?;
        Ok(())
    }
    async fn get_domains(&self, ip: String) -> Result<Vec<LowerName>, ServerError> {
        let mut db = self.state.pool.get().await?;
        let now = chrono::Utc::now().naive_utc();
        let user: Vec<User> = schema::clients::table
            .inner_join(schema::users::table.inner_join(sessions::table))
            .filter(
                schema::clients::ip
                    .eq(ip)
                    .and(schema::sessions::end_timestamp.gt(now)),
            )
            .select(User::as_select())
            .load(&mut db)
            .await?;
        if user.len() == 0 {
            return Ok(vec![]);
        }
        Domain::belonging_to(&user)
            .select(Domain::as_select())
            .load_stream::<Domain>(&mut (*db))
            .await?
            .map(|x| Ok(LowerName::new(&Name::from_str(&x?.domain_name).unwrap())))
            .collect::<Result<Vec<LowerName>, ServerError>>()
            .await
    }

    async fn run_domains<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
        name: &LowerName,
    ) -> Result<(), ServerError> {
        let blocked_domains = self.get_domains(request.src().ip().to_string()).await;
        match blocked_domains {
            Ok(blocked_domains) if blocked_domains.iter().any(|x| x.zone_of(name)) => {
                let builder = MessageResponseBuilder::from_message_request(request);
                let mut header = Header::response_from_request(request.header());
                header.set_authoritative(false);
                response_handle
                    .send_response(builder.build(header, &[], &[], &[], &[]))
                    .await?;
                Ok(())
            }
            Err(e) => {
                log::error!("Error checking domains: {}", e);
                self.resolve(request, response_handle).await?;
                Ok(())
            }
            _ => self.resolve(request, response_handle).await,
        }
    }
}

#[async_trait::async_trait]
impl RequestHandler for Handler {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handle: R,
    ) -> ResponseInfo {
        match (request.message_type(), request.op_code()) {
            (MessageType::Query, OpCode::Query) => {
                let query = request.query();
                let name = query.name();

                if let Err(e) = self
                    .run_domains(request, response_handle.clone(), name)
                    .await
                {
                    log::error!("Error sending response: {e}");
                    let builder = MessageResponseBuilder::from_message_request(request);
                    let mut header = Header::new();
                    header.set_response_code(trust_dns_resolver::proto::op::ResponseCode::ServFail);
                    response_handle
                        .send_response(builder.build(header, &[], &[], &[], &[]))
                        .await
                        .unwrap();
                };
            }
            _ => {}
        }
        return Header::new().into();
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("Diesel result error: {0}")]
    DieselError(#[from] diesel::result::Error),
    #[error("Diesel connection error: {0}")]
    DieselConnection(#[from] diesel::ConnectionError),
    #[error("Trust DNS Proto Error: {0}")]
    TrustDnsProto(#[from] trust_dns_resolver::proto::error::ProtoError),
    #[error("Trust DNS Resolve Error: {0}")]
    TrustDnsResolve(#[from] trust_dns_resolver::error::ResolveError),
    #[error("Deadpool pool error: {0}")]
    Deadpool(#[from] deadpool::PoolError),
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::EXPECTATION_FAILED, self.to_string()).into_response()
    }
}

struct ServerState {
    pool: Pool<AsyncPgConnection>,
}

async fn pause(
    State(state): State<Arc<ServerState>>,
    Path(session_id): Path<u32>,
) -> Result<Json<Session>, ServerError> {
    let mut db = state.pool.get().await?;
    let mut session: Session = sessions::table
        .filter(sessions::id.eq(session_id as i32))
        .select(Session::as_select())
        .first(&mut (*db))
        .await?;
    if let Some(timestamp) = session.end_timestamp {
        session.end_timestamp = None;
        session.time_left = Some(models::EndDuration(timestamp - Utc::now().naive_utc()));
        diesel::update(sessions::table)
            .filter(sessions::id.eq(session_id as i32))
            .set(session.clone())
            .execute(&mut (*db))
            .await?;
    }
    Ok(Json(session))
}
async fn session(
    State(state): State<Arc<ServerState>>,
    Path(session_id): Path<u32>,
) -> Result<Json<Session>, ServerError> {
    let mut db = state.pool.get().await?;
    let session: Session = sessions::table
        .filter(sessions::id.eq(session_id as i32))
        .select(Session::as_select())
        .first(&mut db)
        .await?;
    Ok(Json(session))
}

async fn unpause(
    State(state): State<Arc<ServerState>>,
    Path(session_id): Path<u32>,
) -> Result<Json<Session>, ServerError> {
    let mut db = state.pool.get().await?;
    let mut session: Session = sessions::table
        .filter(sessions::id.eq(session_id as i32))
        .select(Session::as_select())
        .first(&mut (*db))
        .await?;
    if let Some(EndDuration(diff)) = session.time_left {
        session.time_left = None;
        session.end_timestamp = Some(Utc::now().naive_utc() + diff);

        diesel::update(sessions::table)
            .filter(sessions::id.eq(session_id as i32))
            .set(session.clone())
            .execute(&mut (*db))
            .await?;
    }
    Ok(Json(session))
}
async fn timers(State(state): State<Arc<ServerState>>) -> Result<Json<Vec<Session>>, ServerError> {
    let mut db = state.pool.get().await?;
    let now = Utc::now().naive_utc();
    Ok(Json(
        sessions::table
            .filter(
                schema::sessions::end_timestamp
                    .gt(now)
                    .or(schema::sessions::time_left
                        .is_not_null()
                        .and(schema::sessions::time_left.gt(0))),
            )
            .select(Session::as_select())
            .load(&mut db)
            .await?,
    ))
}

#[derive(Serialize, Deserialize)]
struct SessionRequest {
    minutes: Option<i64>,
    seconds: Option<i64>,
}

async fn new_session(
    State(state): State<Arc<ServerState>>,
    Path(user_id): Path<i32>,
    Json(req): Json<SessionRequest>,
) -> Result<Json<Session>, ServerError> {
    let mut db = state.pool.get().await?;
    let now = Utc::now();
    let new_duration = now
        + Duration::minutes(req.minutes.unwrap_or(0))
        + Duration::seconds(req.seconds.unwrap_or(0));
    Ok(Json(
        diesel::insert_into(sessions::table)
            .values((
                sessions::user_id.eq(user_id),
                sessions::end_timestamp.eq(new_duration.naive_utc()),
            ))
            .returning(Session::as_select())
            .get_result(&mut db)
            .await?,
    ))
}

#[tokio::main]
pub async fn main() -> Result<(), ServerError> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let mut config = ResolverConfig::new();
    config.add_name_server(NameServerConfig::new(
        "192.168.1.26:53".parse().unwrap(),
        trust_dns_resolver::config::Protocol::Udp,
    ));
    config.add_name_server(NameServerConfig::new(
        "1.1.1.1:53".parse().unwrap(),
        trust_dns_resolver::config::Protocol::Udp,
    ));
    let resolver = AsyncResolver::tokio(config, ResolverOpts::default());
    // create a new connection pool with the default config
    let config = AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(
        std::env::var("DATABASE_URL").unwrap(),
    );
    let pool = Pool::builder(config).build().unwrap();
    let state = Arc::new(ServerState { pool });
    let handler = Handler {
        resolver,
        state: state.clone(),
    };

    let mut dns_server = ServerFuture::new(handler);

    dns_server.register_socket(
        UdpSocket::bind("0.0.0.0:1053".parse::<SocketAddr>().unwrap())
            .await
            .unwrap(),
    );
    let app = axum::Router::new()
        .route(
            "/sessions/:session_id",
            axum::routing::get(session).post(new_session),
        )
        .route("/sessions/:session_id/pause", axum::routing::get(pause))
        .route("/sessions/:session_id/unpause", axum::routing::get(unpause))
        .route("/sessions/active", axum::routing::get(timers))
        .route("/", axum::routing::get("Woop"))
        .layer(TraceLayer::new_for_http())
        .with_state(state);
    let http_server =
        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap()).serve(app.into_make_service());
    select! {
        res = dns_server.block_until_done() => {
            res?;
        },
        res = http_server => {
            res.unwrap();
        }
    }
    Ok(())
}
