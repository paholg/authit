use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::{path::Path, sync::LazyLock};
use types::{ProvisionLinkInfo, ProvisionRecord, Result, err};
use uuid::Uuid;

use crate::CONFIG;

/// Table definition for provision links.
/// Key: UUID bytes (16 bytes)
/// Value: postcard-serialized ProvisionRecord
const PROVISION_LINKS: TableDefinition<&[u8; 16], &[u8]> = TableDefinition::new("provision_links");

pub static STORAGE: LazyLock<ProvisionStorage> = LazyLock::new(|| {
    let path = CONFIG.data_dir.join("provision.redb");

    ProvisionStorage::open(&path).unwrap()
});

/// Provision link storage backed by redb.
pub struct ProvisionStorage {
    db: Database,
}

impl ProvisionStorage {
    /// Open or create the database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Database::create(path)?;

        // Ensure table exists
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(PROVISION_LINKS)?;
        }
        write_txn.commit()?;

        Ok(Self { db })
    }

    /// Create a new provision link and persist it.
    pub fn create_link(
        &self,
        duration_seconds: u64,
        max_uses: Option<u32>,
    ) -> Result<ProvisionRecord> {
        let id = Uuid::now_v7();
        let record = ProvisionRecord::new(id, duration_seconds, max_uses);

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(PROVISION_LINKS)?;
            let value = postcard::to_allocvec(&record)?;
            table.insert(id.as_bytes(), value.as_slice())?;
        }
        write_txn.commit()?;

        if rand_cleanup() {
            let _ = self.cleanup_expired();
        }

        Ok(record)
    }

    /// Verify a provision link exists and is valid.
    /// Does NOT increment use count.
    pub fn verify_link(&self, id: Uuid) -> Result<ProvisionLinkInfo> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(PROVISION_LINKS)?;

        let value = table
            .get(id.as_bytes())?
            .ok_or_else(|| err!("provision link not found or has been revoked"))?;

        let record: ProvisionRecord = postcard::from_bytes(value.value())?;

        if record.is_expired() {
            return Err(err!("provision link has expired"));
        }

        if record.is_exhausted() {
            return Err(err!("provision link has reached maximum uses"));
        }

        Ok(ProvisionLinkInfo::from(&record))
    }

    /// Consume one use of a provision link.
    /// Returns the updated record (for potential rollback) or error if expired/exhausted.
    /// Deletes the link if use_count reaches max_uses.
    pub fn consume_link(&self, id: Uuid) -> Result<ProvisionRecord> {
        let write_txn = self.db.begin_write()?;
        let record = {
            let mut table = write_txn.open_table(PROVISION_LINKS)?;

            // Read and parse the record, dropping the borrow before mutations
            let mut record: ProvisionRecord = {
                let value = table
                    .get(id.as_bytes())?
                    .ok_or_else(|| err!("provision link not found or has been revoked"))?;
                postcard::from_bytes(value.value())?
            };

            if record.is_expired() {
                // Clean up expired link
                table.remove(id.as_bytes())?;
                return Err(err!("provision link has expired"));
            }

            if record.is_exhausted() {
                // Clean up exhausted link
                table.remove(id.as_bytes())?;
                return Err(err!("provision link has reached maximum uses"));
            }

            // Increment use count
            record.use_count += 1;

            // Delete if now exhausted, otherwise update
            if record.is_exhausted() {
                table.remove(id.as_bytes())?;
            } else {
                let serialized = postcard::to_allocvec(&record)?;
                table.insert(id.as_bytes(), serialized.as_slice())?;
            }

            record
        };
        write_txn.commit()?;

        Ok(record)
    }

    /// Restore a previously consumed link (e.g., if user creation failed).
    /// Decrements use_count, or re-creates the link if it was deleted.
    pub fn unconsume_link(&self, record: ProvisionRecord) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(PROVISION_LINKS)?;

            // Restore with decremented use count
            let mut restored = record;
            restored.use_count = restored.use_count.saturating_sub(1);

            let serialized = postcard::to_allocvec(&restored)?;
            table.insert(restored.id.as_bytes(), serialized.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Delete a specific provision link.
    pub fn delete_link(&self, id: Uuid) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(PROVISION_LINKS)?;
            table.remove(id.as_bytes())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Remove all expired and maxed-out links.
    /// Returns the number of links purged.
    pub fn cleanup_expired(&self) -> Result<usize> {
        let write_txn = self.db.begin_write()?;
        let count = {
            let mut table = write_txn.open_table(PROVISION_LINKS)?;
            let mut to_delete = Vec::new();

            for result in table.iter()? {
                let (key, value) = result?;
                let record: ProvisionRecord = match postcard::from_bytes(value.value()) {
                    Ok(r) => r,
                    Err(_) => {
                        // Corrupted record, delete it
                        to_delete.push(*key.value());
                        continue;
                    }
                };

                if record.is_expired() || record.is_exhausted() {
                    to_delete.push(*key.value());
                }
            }

            for key in &to_delete {
                table.remove(key)?;
            }

            to_delete.len()
        };
        write_txn.commit()?;

        if count > 0 {
            tracing::debug!("cleaned up {} expired/exhausted provision links", count);
        }

        Ok(count)
    }
}

/// Returns true ~10% of the time for opportunistic cleanup.
fn rand_cleanup() -> bool {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Use current time nanoseconds as a simple random source
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() % 10 == 0)
        .unwrap_or(false)
}
