// @generated automatically by Diesel CLI.

diesel::table! {
    recordings (id) {
        id -> Int4,
        user_id -> Nullable<Int4>,
        #[max_length = 36]
        uuid -> Bpchar,
        rec_start -> Timestamp,
        rec_end -> Timestamp,
        status -> Text,
        #[max_length = 32]
        short_status -> Varchar,
        stage -> Int4,
        #[max_length = 32]
        channel -> Varchar,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 32]
        username -> Varchar,
        max_length_seconds -> Int4,
        can_upload -> Bool,
        can_delete -> Bool,
        superuser -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    recordings,
    users,
);
