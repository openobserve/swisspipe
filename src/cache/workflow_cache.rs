use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

/// Cached workflow metadata for fast validation
#[derive(Clone, Debug)]
pub struct WorkflowCacheEntry {
    pub workflow_id: String,
    pub start_node_id: String,
    pub cached_at: DateTime<Utc>,
    pub ttl_seconds: i64,
}

impl WorkflowCacheEntry {
    pub fn new(workflow_id: String, start_node_id: String, ttl_seconds: i64) -> Self {
        Self {
            workflow_id,
            start_node_id,
            cached_at: Utc::now(),
            ttl_seconds,
        }
    }

    pub fn is_expired(&self) -> bool {
        let expires_at = self.cached_at + Duration::seconds(self.ttl_seconds);
        Utc::now() > expires_at
    }
}

/// In-memory cache for workflow existence and metadata
#[derive(Clone)]
pub struct WorkflowCache {
    cache: Arc<RwLock<HashMap<String, WorkflowCacheEntry>>>,
    default_ttl_seconds: i64,
}

impl WorkflowCache {
    pub fn new(default_ttl_seconds: Option<i64>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl_seconds: default_ttl_seconds.unwrap_or(300), // 5 minutes default
        }
    }

    /// Get workflow from cache if exists and not expired
    pub async fn get(&self, workflow_id: &str) -> Option<WorkflowCacheEntry> {
        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.get(workflow_id) {
            if !entry.is_expired() {
                tracing::debug!("Cache hit for workflow: {}", workflow_id);
                return Some(entry.clone());
            } else {
                tracing::debug!("Cache entry expired for workflow: {}", workflow_id);
            }
        }
        
        tracing::debug!("Cache miss for workflow: {}", workflow_id);
        None
    }

    /// Put workflow into cache
    pub async fn put(&self, workflow_id: String, start_node_id: String) {
        let entry = WorkflowCacheEntry::new(workflow_id.clone(), start_node_id, self.default_ttl_seconds);
        let mut cache = self.cache.write().await;
        cache.insert(workflow_id.clone(), entry);
        tracing::debug!("Cached workflow: {}", workflow_id);
    }

    /// Put workflow into cache with custom TTL
    pub async fn put_with_ttl(&self, workflow_id: String, start_node_id: String, ttl_seconds: i64) {
        let entry = WorkflowCacheEntry::new(workflow_id.clone(), start_node_id, ttl_seconds);
        let mut cache = self.cache.write().await;
        cache.insert(workflow_id.clone(), entry);
        tracing::debug!("Cached workflow: {} with TTL: {}s", workflow_id, ttl_seconds);
    }

    /// Remove workflow from cache (used when workflow is updated/deleted)
    pub async fn invalidate(&self, workflow_id: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(workflow_id);
        tracing::info!("Invalidated cache for workflow: {}", workflow_id);
    }

    /// Clear all entries from cache
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        let count = cache.len();
        cache.clear();
        tracing::info!("Cleared workflow cache ({} entries)", count);
    }

    /// Remove expired entries from cache
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        let initial_count = cache.len();
        
        cache.retain(|workflow_id, entry| {
            let keep = !entry.is_expired();
            if !keep {
                tracing::debug!("Removing expired cache entry for workflow: {}", workflow_id);
            }
            keep
        });
        
        let removed_count = initial_count - cache.len();
        if removed_count > 0 {
            tracing::info!("Cleaned up {} expired workflow cache entries", removed_count);
        }
        
        removed_count
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let expired_entries = cache.values().filter(|entry| entry.is_expired()).count();
        
        CacheStats {
            total_entries,
            valid_entries: total_entries - expired_entries,
            expired_entries,
        }
    }

    /// Check if workflow exists in cache (ignoring expiration)
    pub async fn contains(&self, workflow_id: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_key(workflow_id)
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

impl Default for WorkflowCache {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration as TokioDuration};

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = WorkflowCache::new(Some(1)); // 1 second TTL
        let workflow_id = "test-workflow-123".to_string();
        let start_node = "trigger".to_string();

        // Cache should be empty initially
        assert!(cache.get(&workflow_id).await.is_none());

        // Put entry in cache
        cache.put(workflow_id.clone(), start_node.clone()).await;

        // Should be able to retrieve it
        let entry = cache.get(&workflow_id).await;
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.workflow_id, workflow_id);
        assert_eq!(entry.start_node_id, start_node);

        // Wait for expiration
        sleep(TokioDuration::from_secs(2)).await;

        // Should be expired now
        assert!(cache.get(&workflow_id).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let cache = WorkflowCache::default();
        let workflow_id = "test-workflow-456".to_string();
        let start_node = "trigger".to_string();

        // Put entry in cache
        cache.put(workflow_id.clone(), start_node.clone()).await;
        assert!(cache.get(&workflow_id).await.is_some());

        // Invalidate it
        cache.invalidate(&workflow_id).await;
        assert!(cache.get(&workflow_id).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let cache = WorkflowCache::new(Some(1)); // 1 second TTL
        
        // Add some entries
        cache.put("workflow-1".to_string(), "trigger".to_string()).await;
        cache.put("workflow-2".to_string(), "trigger".to_string()).await;

        // Wait for expiration
        sleep(TokioDuration::from_secs(2)).await;

        // Clean up expired entries
        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 2);

        let stats = cache.stats().await;
        assert_eq!(stats.total_entries, 0);
    }
}