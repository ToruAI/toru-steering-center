use crate::db::DbPool;
use toru_plugin_api::{PluginError, PluginResult};

/// Sqlite-backed key-value store for plugins
///
/// Each plugin gets its own isolated namespace in the plugin_kv table.
/// This implements the PluginKvStore trait from toru-plugin-api.
#[derive(Debug, Clone)]
pub struct SqliteKvStore {
    pool: DbPool,
    plugin_id: String,
}

impl SqliteKvStore {
    /// Create a new SqliteKvStore for a specific plugin
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `plugin_id` - Plugin ID for namespace isolation
    pub fn new(pool: DbPool, plugin_id: String) -> Self {
        Self { pool, plugin_id }
    }

    /// Get the plugin ID
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}

#[async_trait::async_trait]
impl toru_plugin_api::PluginKvStore for SqliteKvStore {
    /// Get a value from the plugin's KV namespace
    ///
    /// # Arguments
    /// * `key` - Key to retrieve
    ///
    /// # Returns
    /// Ok(Some(value)) if key exists, Ok(None) if key doesn't exist
    async fn get(&self, key: &str) -> PluginResult<Option<String>> {
        crate::db::plugin_kv_get(&self.pool, &self.plugin_id, key)
            .await
            .map_err(|e| PluginError::Internal(format!("Failed to get value: {}", e)))
    }

    /// Set a value in the plugin's KV namespace
    ///
    /// # Arguments
    /// * `key` - Key to set
    /// * `value` - Value to store
    async fn set(&self, key: &str, value: &str) -> PluginResult<()> {
        crate::db::plugin_kv_set(&self.pool, &self.plugin_id, key, value)
            .await
            .map_err(|e| PluginError::Internal(format!("Failed to set value: {}", e)))
    }

    /// Delete a value from the plugin's KV namespace
    ///
    /// # Arguments
    /// * `key` - Key to delete
    async fn delete(&self, key: &str) -> PluginResult<()> {
        crate::db::plugin_kv_delete(&self.pool, &self.plugin_id, key)
            .await
            .map_err(|e| PluginError::Internal(format!("Failed to delete value: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toru_plugin_api::PluginKvStore;

    #[tokio::test]
    async fn test_kv_store_basic_operations() {
        let pool = crate::db::init_db().unwrap();
        let kv = SqliteKvStore::new(pool, "test-plugin".to_string());

        // Test set and get
        kv.set("test_key", "test_value").await.unwrap();
        assert_eq!(
            kv.get("test_key").await.unwrap(),
            Some("test_value".to_string())
        );

        // Test update
        kv.set("test_key", "updated_value").await.unwrap();
        assert_eq!(
            kv.get("test_key").await.unwrap(),
            Some("updated_value".to_string())
        );

        // Test delete
        kv.delete("test_key").await.unwrap();
        assert_eq!(kv.get("test_key").await.unwrap(), None);

        // Test get non-existent key
        assert_eq!(kv.get("nonexistent").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_kv_store_isolation() {
        let pool = crate::db::init_db().unwrap();
        let kv1 = SqliteKvStore::new(pool.clone(), "plugin-a".to_string());
        let kv2 = SqliteKvStore::new(pool, "plugin-b".to_string());

        // Set the same key in both plugins
        kv1.set("shared_key", "value-a").await.unwrap();
        kv2.set("shared_key", "value-b").await.unwrap();

        // Verify isolation
        assert_eq!(
            kv1.get("shared_key").await.unwrap(),
            Some("value-a".to_string())
        );
        assert_eq!(
            kv2.get("shared_key").await.unwrap(),
            Some("value-b".to_string())
        );
    }
}
