use dioxus::fullstack::Lazy;
use secrecy::ExposeSecret;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use types::Result;

use crate::CONFIG;
pub use provision_link::ProvisionLink;
pub use session::Session;

mod provision_link;
mod session;

static POOL: Lazy<SqlitePool> = Lazy::new(|| async {
    let db_path = CONFIG.data_dir.join("db.sqlite");

    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .pragma("key", CONFIG.db_secret.expose_secret())
        .create_if_missing(true);

    SqlitePool::connect_with(options).await
});

pub async fn migrate() -> Result<()> {
    Ok(sqlx::migrate!("../migrations").run(&*POOL).await?)
}
