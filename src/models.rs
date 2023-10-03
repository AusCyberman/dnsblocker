use chrono::Utc;
use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    prelude::{Associations, Identifiable, Insertable, Queryable},
    query_builder::AsChangeset,
    serialize::ToSql,
    sql_types,
    sqlite::Sqlite,
    Selectable,
};
use serde::{Serialize, Serializer};

#[derive(Selectable, Queryable, Debug, PartialEq, Identifiable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub id: i32,
    pub name: String,
    pub username: String,
}

#[derive(Selectable, Queryable)]
#[diesel(table_name = crate::schema::clients)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(User))]
pub struct Client {
    pub id: i32,
    pub ip: String,
    pub user_id: i32,
}

#[derive(Identifiable, Queryable, Selectable, Associations, Debug, PartialEq)]
#[diesel(table_name = crate::schema::domain)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(User))]
pub struct Domain {
    pub id: i32,
    pub domain_name: String,
    pub user_id: i32,
}

#[derive(Debug, FromSqlRow, AsExpression, Clone)]
#[diesel(sql_type = sql_types::Integer)]
pub struct EndDuration(pub chrono::Duration);

impl<DB> ToSql<sql_types::Integer, DB> for EndDuration
where
    DB: Backend,
    i32: ToSql<sql_types::Integer, DB>,
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, DB>,
    ) -> diesel::serialize::Result {
        let milliseconds = self.0.num_milliseconds() as i32;
        let borrowed = Box::leak(Box::new(milliseconds));
        borrowed.to_sql(out)
    }
}

impl FromSql<sql_types::Integer, Sqlite> for EndDuration
where
    i32: ToSql<sql_types::Integer, Sqlite>,
{
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        Ok(EndDuration(chrono::Duration::milliseconds(
            i32::from_sql(bytes)? as i64,
        )))
    }
}

impl Serialize for EndDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(self.0.num_milliseconds())
    }
}

#[derive(
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    Insertable,
    Debug,
    AsChangeset,
    Serialize,
    Clone,
)]
#[diesel(table_name = crate::schema::sessions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(treat_none_as_null = true)]
#[diesel(belongs_to(User))]
pub struct Session {
    pub id: i32,
    pub user_id: i32,
    #[serde(with = "json_time")]
    pub end_timestamp: Option<chrono::NaiveDateTime>,
    pub time_left: Option<EndDuration>,
}

mod json_time {
    use super::*;
    use chrono::{DateTime, NaiveDateTime};
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn time_to_json(t: NaiveDateTime) -> String {
        DateTime::<Utc>::from_utc(t, Utc).to_rfc3339()
    }
    pub fn serialize<S: Serializer>(
        time: &Option<NaiveDateTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        if let Some(time) = time {
            time_to_json(time.clone()).serialize(serializer)
        } else {
            serializer.serialize_none()
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<NaiveDateTime, D::Error> {
        let time: String = Deserialize::deserialize(deserializer)?;
        Ok(DateTime::parse_from_rfc3339(&time)
            .map_err(D::Error::custom)?
            .naive_utc())
    }
}
