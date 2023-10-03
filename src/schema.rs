// @generated automatically by Diesel CLI.

diesel::table! {
    clients (id) {
        id -> Integer,
        block_all -> Bool,
        ip -> Text,
        user_id -> Integer,
    }
}

diesel::table! {
    domain (id) {
        id -> Integer,
        domain_name -> Text,
        user_id -> Integer,
    }
}

diesel::table! {
    sessions (id) {
        id -> Integer,
        user_id -> Integer,
        time_left -> Nullable<Integer>,
        end_timestamp -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        name -> Text,
        username -> Text,
    }
}

diesel::joinable!(clients -> users (user_id));
diesel::joinable!(domain -> users (user_id));
diesel::joinable!(sessions -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    clients,
    domain,
    sessions,
    users,
);
