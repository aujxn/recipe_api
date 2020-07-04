use anyhow::Result;
use tokio_postgres::{Client, Config, NoTls};
use tracing::error;

pub async fn connect_db() -> Result<Client> {
    let (client, connection) = Config::new()
        .user("austen")
        .dbname("recipe_api")
        .host("localhost")
        .connect(NoTls)
        .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("connection error: {}", e);
        }
    });

    Ok(client)
}
