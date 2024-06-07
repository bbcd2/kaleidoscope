// @generated automatically by Diesel CLI.

diesel::table! {
    recordings (id) {
        id -> Int4,
        user_id -> Nullable<Int4>,
        #[max_length = 36]
        uuid -> Bpchar,
        rec_start -> Timestamp,
        rec_end -> Timestamp,
        #[max_length = 128]
        status -> Varchar,
        stage -> Int4,
        #[max_length = 32]
        channel -> Varchar,
    }
}
