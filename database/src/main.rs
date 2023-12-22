fn main() {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        database::connect().await.unwrap();
        database::run_migrations().await.unwrap();
    })
}
