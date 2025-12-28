use dioxus::fullstack::Lazy;
use sqlx::SqlitePool;
#[cfg(debug_assertions)]
use sqlx::sqlite::SqliteConnectOptions;
use types::Result;

use crate::CONFIG;
pub use provision_link::ProvisionLink;

mod provision_link;

static POOL: Lazy<SqlitePool> = Lazy::new(|| async {
    let db_path = CONFIG.data_dir.join("db.sqlite");

    #[cfg(debug_assertions)]
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);

    #[cfg(not(debug_assertions))]
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .pragma("key", CONFIG.db_secret.expose_secret().to_owned())
        .create_if_missing(true);

    SqlitePool::connect_with(options).await
});

pub async fn migrate() -> Result<()> {
    Ok(sqlx::migrate!("../migrations").run(&*POOL).await?)
}
