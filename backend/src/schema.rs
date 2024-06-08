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
