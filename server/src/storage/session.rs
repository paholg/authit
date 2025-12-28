use types::{Result, UserData};
use uuid::Uuid;

use crate::{storage::POOL, uuid_v7::UuidV7Ext};

struct SessionRow {
    id: Uuid,
    user_data: String,
}

#[derive(Debug)]
pub struct Session {
    id: Uuid,
    user_data: UserData,
}

impl Session {
    pub fn new(user_data: UserData) -> Self {
        let id = Uuid::now_v7();

        Self { id, user_data }
    }

    pub async fn create(user_data: UserData) -> Result<Self> {
        let session = Self::new(user_data);
        session.insert().await?;
        Ok(session)
    }

    pub async fn find(id: Uuid) -> Result<Self> {
        let id_bytes = id.as_bytes().as_slice();

        let row = sqlx::query_as!(
            SessionRow,
            r#"
            SELECT
                id as "id: _",
                user_data
            FROM sessions
            WHERE id = ?
            "#,
            id_bytes,
        )
        .fetch_one(&*POOL)
        .await?;

        Ok(Self {
            id: row.id,
            user_data: serde_json::from_str(&row.user_data)?,
        })
    }

    /// Find session by signed token (cookie value).
    pub async fn find_token(token: &str) -> Result<Self> {
        let uuid = Uuid::from_token(token)?;
        Self::find(uuid).await
    }

    pub fn user_data(&self) -> &UserData {
        &self.user_data
    }

    pub fn as_token(&self) -> Result<String> {
        self.id.as_token()
    }

    pub async fn insert(&self) -> Result<()> {
        let id = self.id.as_bytes().as_slice();
        let user_data = serde_json::to_string(&self.user_data)?;

        sqlx::query!(
            r#"
            INSERT INTO sessions (id, user_data)
            VALUES (?, ?)
            "#,
            id,
            user_data
        )
        .execute(&*POOL)
        .await?;

        Ok(())
    }

    pub async fn delete(&self) -> Result<()> {
        let id = self.id.as_bytes().as_slice();

        sqlx::query!(
            r#"
            DELETE FROM sessions
            WHERE id = ?
            "#,
            id,
        )
        .execute(&*POOL)
        .await?;

        Ok(())
    }

    pub async fn delete_token(token: &str) -> Result<()> {
        if let Ok(session) = Self::find_token(token).await {
            session.delete().await?;
        }
        Ok(())
    }
}
