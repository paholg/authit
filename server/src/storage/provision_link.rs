use std::time::Duration;

use jiff::Timestamp;
use jiff_sqlx::{Timestamp as SqlxTimestamp, ToSqlx};
use types::{Result, err, provision::ProvisionToken};
use uuid::Uuid;

use crate::{storage::POOL, uuid_v7::UuidV7Ext};

struct ProvisionLinkRow {
    id: Uuid,
    expires_at: SqlxTimestamp,
    max_uses: Option<i32>,
    use_count: i32,
    groups: String,
}

#[derive(Debug)]
pub struct ProvisionLink {
    id: Uuid,
    expires_at: Timestamp,
    max_uses: Option<i32>,
    use_count: i32,
    groups: Vec<String>,
}

impl ProvisionLink {
    pub fn new(duration: Duration, max_uses: Option<u8>, groups: Vec<String>) -> Self {
        let id = Uuid::now_v7();

        Self {
            id,
            expires_at: id.jiff_timestamp() + duration,
            max_uses: max_uses.map(Into::into),
            use_count: 0,
            groups,
        }
    }

    pub async fn create(
        duration: Duration,
        max_uses: Option<u8>,
        groups: Vec<String>,
    ) -> Result<Self> {
        let this = Self::new(duration, max_uses, groups);
        this.insert().await?;
        Ok(this)
    }

    pub async fn find(id: Uuid) -> Result<Self> {
        let id_bytes = id.as_bytes().as_slice();

        let row = sqlx::query_as!(
            ProvisionLinkRow,
            r#"
            SELECT
                id as "id: _",
                expires_at as "expires_at: _",
                max_uses as "max_uses: _",
                use_count as "use_count: _",
                groups
            FROM provision_links
            WHERE id = ?
            "#,
            id_bytes,
        )
        .fetch_one(&*POOL)
        .await?;

        Ok(Self {
            id: row.id,
            expires_at: row.expires_at.to_jiff(),
            max_uses: row.max_uses,
            use_count: row.use_count,
            groups: serde_json::from_str(&row.groups)?,
        })
    }

    pub async fn find_token(token: String) -> Result<Self> {
        let uuid = Uuid::from_token(&token)?;
        Self::find(uuid).await
    }

    pub async fn consume(token: String) -> Result<Self> {
        let record = Self::find_token(token).await?;
        record.verify()?;
        record.try_increment().await?;
        Ok(record)
    }

    pub async fn decrement(&self) -> Result<()> {
        let id = self.id.as_bytes().as_slice();

        sqlx::query!(
            r#"
            UPDATE provision_links
            SET use_count = use_count - 1
            WHERE id = ? AND use_count > 0
            "#,
            id,
        )
        .execute(&*POOL)
        .await?;

        Ok(())
    }

    pub fn verify(&self) -> Result<()> {
        if self.is_expired() {
            return Err(err!("provision link has expired"));
        }

        if self.is_exhausted() {
            return Err(err!("provision link has already been used"));
        }

        Ok(())
    }

    fn is_expired(&self) -> bool {
        Timestamp::now() >= self.expires_at
    }

    fn is_exhausted(&self) -> bool {
        self.max_uses.is_some_and(|max| self.use_count >= max)
    }

    pub fn as_token(&self) -> Result<ProvisionToken> {
        let signed_uuid = self.id.as_token()?;

        Ok(ProvisionToken::new(signed_uuid))
    }

    pub fn groups(&self) -> &[String] {
        &self.groups
    }

    pub async fn insert(&self) -> Result<()> {
        let expires_at = self.expires_at.to_sqlx();
        let groups = serde_json::to_string(&self.groups)?;

        sqlx::query!(
            r#"
            INSERT INTO provision_links (id, expires_at, max_uses, use_count, groups)
            VALUES (?, ?, ?, ?, ?)
            "#,
            self.id,
            expires_at,
            self.max_uses,
            self.use_count,
            groups,
        )
        .execute(&*POOL)
        .await?;

        Ok(())
    }

    async fn try_increment(&self) -> Result<()> {
        let id = self.id.as_bytes().as_slice();

        let result = sqlx::query!(
            r#"
            UPDATE provision_links
            SET use_count = use_count + 1
            WHERE id = ? AND (max_uses IS NULL OR use_count < max_uses)
            "#,
            id,
        )
        .execute(&*POOL)
        .await?;

        if result.rows_affected() == 0 {
            return Err(err!("link already used up"));
        }

        Ok(())
    }

    pub async fn delete(&self) -> Result<()> {
        let id = self.id.as_bytes().as_slice();

        sqlx::query!(
            r#"
            DELETE FROM provision_links
            WHERE id = ?
            "#,
            id,
        )
        .execute(&*POOL)
        .await?;

        Ok(())
    }
}
