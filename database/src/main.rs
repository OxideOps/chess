fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        sqlx::migrate!()
            .run(
                &sqlx::PgPool::connect(
                    &std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
                )
                .await
                .expect("Failed to connect to the database"),
            )
            .await
            .expect("Failed to run migrations");
    })
}
