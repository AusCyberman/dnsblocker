// @generated automatically by Diesel CLI.

diesel::table! {
    clients (id) {
        id -> Int4,
        block_all -> Bool,
        ip -> Text,
        user_id -> Int4,
    }
}

diesel::table! {
    domain (id) {
        id -> Int4,
        domain_name -> Text,
        user_id -> Int4,
    }
}

diesel::table! {
    sessions (id) {
        id -> Int4,
        user_id -> Int4,
        time_left -> Nullable<Int4>,
        end_timestamp -> Nullable<Timestamp>,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
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
