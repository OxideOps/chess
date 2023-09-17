use tokio_postgres::{Client, NoTls};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::Mutex;

pub static GLOBAL_CLIENT: Lazy<Arc<Mutex<Option<Client>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));

pub async fn initialize_global_client() -> Result<(), Box<dyn std::error::Error>> {
    *GLOBAL_CLIENT.lock().await = Some(connect().await?);
    Ok(())
}


pub async fn connect() -> Result<Client, Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}
