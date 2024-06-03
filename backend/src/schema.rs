// @generated automatically by Diesel CLI.

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

diesel::table! {
    videos (id) {
        id -> Int4,
        user_id -> Nullable<Int4>,
        #[max_length = 36]
        uuid -> Bpchar,
        rec_start -> Timestamp,
        rec_end -> Timestamp,
        #[max_length = 128]
        status -> Varchar,
        channel -> Int4,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    users,
    videos,
);
