diesel::table! {
    users (user_id) {
        user_id -> Int8,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 320]
        email -> Varchar,
    }
}
