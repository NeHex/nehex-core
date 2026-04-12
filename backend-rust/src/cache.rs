use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::{Mutex, RwLock};
use tracing::warn;

#[derive(Clone)]
pub struct RedisCacheSettings {
    pub url: String,
    pub prefix: String,
    pub retry_seconds: u64,
    pub socket_connect_timeout_seconds: f64,
    pub socket_timeout_seconds: f64,
}

#[derive(Clone)]
pub struct RuntimeCache {
    max_entries: usize,
    memory: Arc<RwLock<HashMap<String, CacheEntry>>>,
    redis: Option<Arc<RedisCacheBackend>>,
}

#[derive(Clone)]
struct CacheEntry {
    expires_at: Instant,
    payload: Vec<u8>,
}

struct RedisCacheBackend {
    settings: RedisCacheSettings,
    state: Mutex<RedisState>,
}

struct RedisState {
    client: Option<redis::Client>,
    next_retry_at: Option<Instant>,
}

impl RuntimeCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            memory: Arc::new(RwLock::new(HashMap::new())),
            redis: None,
        }
    }

    pub fn new_with_redis(max_entries: usize, settings: RedisCacheSettings) -> Self {
        Self {
            max_entries: max_entries.max(1),
            memory: Arc::new(RwLock::new(HashMap::new())),
            redis: Some(Arc::new(RedisCacheBackend {
                settings,
                state: Mutex::new(RedisState {
                    client: None,
                    next_retry_at: None,
                }),
            })),
        }
    }

    pub async fn get<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned + Clone + Send + Sync + 'static,
    {
        if let Some(redis) = &self.redis {
            if let Ok(payload) = redis.get(key).await {
                if let Some(payload) = payload {
                    return decode_payload::<T>(&payload);
                }
                return None;
            }
        }

        self.get_from_memory(key).await
    }

    pub async fn set<T>(&self, key: impl Into<String>, value: T, ttl_seconds: u64)
    where
        T: Serialize + Clone + Send + Sync + 'static,
    {
        let Some(payload) = encode_payload(&value) else {
            return;
        };

        let ttl = ttl_seconds.max(1);
        let key = key.into();

        if let Some(redis) = &self.redis {
            if redis.set(&key, &payload, ttl).await.is_ok() {
                self.delete_from_memory(&key).await;
                return;
            }
        }

        self.set_to_memory(key, payload, ttl).await;
    }

    pub async fn delete(&self, key: &str) {
        self.delete_from_memory(key).await;
        if let Some(redis) = &self.redis {
            let _ = redis.delete(key).await;
        }
    }

    pub async fn delete_prefix(&self, prefix: &str) {
        self.delete_prefix_from_memory(prefix).await;
        if let Some(redis) = &self.redis {
            let _ = redis.delete_prefix(prefix).await;
        }
    }

    pub async fn clear(&self) {
        self.clear_memory().await;
        if let Some(redis) = &self.redis {
            let _ = redis.clear().await;
        }
    }

    async fn get_from_memory<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned + Clone + Send + Sync + 'static,
    {
        let now = Instant::now();
        let maybe_payload = {
            let guard = self.memory.read().await;
            let entry = guard.get(key)?;
            if entry.expires_at <= now {
                None
            } else {
                Some(entry.payload.clone())
            }
        };

        let Some(payload) = maybe_payload else {
            self.delete_from_memory(key).await;
            return None;
        };

        decode_payload::<T>(&payload)
    }

    async fn set_to_memory(&self, key: String, payload: Vec<u8>, ttl_seconds: u64) {
        let ttl = Duration::from_secs(ttl_seconds.max(1));
        let expires_at = Instant::now() + ttl;

        let mut guard = self.memory.write().await;
        purge_expired_locked(&mut guard, Instant::now());
        if !guard.contains_key(&key) && guard.len() >= self.max_entries {
            evict_one_locked(&mut guard);
        }

        guard.insert(
            key,
            CacheEntry {
                expires_at,
                payload,
            },
        );
    }

    async fn delete_from_memory(&self, key: &str) {
        let mut guard = self.memory.write().await;
        guard.remove(key);
    }

    async fn delete_prefix_from_memory(&self, prefix: &str) {
        let mut guard = self.memory.write().await;
        guard.retain(|key, _| !key.starts_with(prefix));
    }

    async fn clear_memory(&self) {
        let mut guard = self.memory.write().await;
        guard.clear();
    }
}

impl RedisCacheBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, ()> {
        let full_key = self.redis_key(key);
        let mut conn = self.connection().await.ok_or(())?;
        let result = redis::cmd("GET")
            .arg(full_key)
            .query_async::<Option<Vec<u8>>>(&mut conn)
            .await;

        match result {
            Ok(payload) => Ok(payload),
            Err(error) => {
                self.mark_failed().await;
                warn!("[cache] redis GET failed, fallback to memory cache: {error}");
                Err(())
            }
        }
    }

    async fn set(&self, key: &str, payload: &[u8], ttl_seconds: u64) -> Result<(), ()> {
        let full_key = self.redis_key(key);
        let mut conn = self.connection().await.ok_or(())?;
        let result = redis::cmd("SETEX")
            .arg(full_key)
            .arg(ttl_seconds.max(1))
            .arg(payload)
            .query_async::<()>(&mut conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(error) => {
                self.mark_failed().await;
                warn!("[cache] redis SETEX failed, fallback to memory cache: {error}");
                Err(())
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<(), ()> {
        let full_key = self.redis_key(key);
        let mut conn = self.connection().await.ok_or(())?;
        let result = redis::cmd("DEL")
            .arg(full_key)
            .query_async::<i64>(&mut conn)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(error) => {
                self.mark_failed().await;
                warn!("[cache] redis DEL failed: {error}");
                Err(())
            }
        }
    }

    async fn delete_prefix(&self, prefix: &str) -> Result<(), ()> {
        self.delete_by_pattern(&format!("{}*", self.redis_key(prefix)))
            .await
    }

    async fn clear(&self) -> Result<(), ()> {
        self.delete_by_pattern(&format!("{}*", self.settings.prefix))
            .await
    }

    async fn delete_by_pattern(&self, pattern: &str) -> Result<(), ()> {
        let mut conn = self.connection().await.ok_or(())?;
        let mut cursor: u64 = 0;

        loop {
            let query = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(200)
                .query_async::<(u64, Vec<String>)>(&mut conn)
                .await;

            let (next_cursor, keys) = match query {
                Ok(value) => value,
                Err(error) => {
                    self.mark_failed().await;
                    warn!("[cache] redis SCAN failed: {error}");
                    return Err(());
                }
            };

            if !keys.is_empty() {
                let del_result = redis::cmd("DEL")
                    .arg(keys)
                    .query_async::<i64>(&mut conn)
                    .await;
                if let Err(error) = del_result {
                    self.mark_failed().await;
                    warn!("[cache] redis DEL by pattern failed: {error}");
                    return Err(());
                }
            }

            if next_cursor == 0 {
                break;
            }
            cursor = next_cursor;
        }

        Ok(())
    }

    async fn connection(&self) -> Option<redis::aio::MultiplexedConnection> {
        let client = self.client().await?;
        let connect_timeout =
            Duration::from_secs_f64(self.settings.socket_connect_timeout_seconds.max(0.1_f64));
        let response_timeout =
            Duration::from_secs_f64(self.settings.socket_timeout_seconds.max(0.1_f64));
        let config = redis::AsyncConnectionConfig::new()
            .set_connection_timeout(connect_timeout)
            .set_response_timeout(response_timeout);
        let connection = client
            .get_multiplexed_async_connection_with_config(&config)
            .await;
        match connection {
            Ok(conn) => Some(conn),
            Err(error) => {
                self.mark_failed().await;
                warn!("[cache] redis connection failed, fallback to memory cache: {error}");
                None
            }
        }
    }

    async fn client(&self) -> Option<redis::Client> {
        let now = Instant::now();
        let mut guard = self.state.lock().await;

        if let Some(client) = guard.client.clone() {
            return Some(client);
        }
        if guard.next_retry_at.is_some_and(|retry_at| now < retry_at) {
            return None;
        }

        match redis::Client::open(self.settings.url.clone()) {
            Ok(client) => {
                guard.client = Some(client.clone());
                guard.next_retry_at = None;
                Some(client)
            }
            Err(error) => {
                guard.client = None;
                guard.next_retry_at =
                    Some(now + Duration::from_secs(self.settings.retry_seconds.max(1)));
                warn!("[cache] redis unavailable, fallback to memory cache: {error}");
                None
            }
        }
    }

    async fn mark_failed(&self) {
        let mut guard = self.state.lock().await;
        guard.client = None;
        guard.next_retry_at =
            Some(Instant::now() + Duration::from_secs(self.settings.retry_seconds.max(1)));
    }

    fn redis_key(&self, key: &str) -> String {
        format!("{}{}", self.settings.prefix, key)
    }
}

fn encode_payload<T>(value: &T) -> Option<Vec<u8>>
where
    T: Serialize + Send + Sync + 'static,
{
    serde_json::to_vec(value).ok()
}

fn decode_payload<T>(payload: &[u8]) -> Option<T>
where
    T: DeserializeOwned + Clone + Send + Sync + 'static,
{
    serde_json::from_slice(payload).ok()
}

fn purge_expired_locked(store: &mut HashMap<String, CacheEntry>, now: Instant) {
    let expired = store
        .iter()
        .filter(|(_, entry)| entry.expires_at <= now)
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();

    for key in expired {
        store.remove(&key);
    }
}

fn evict_one_locked(store: &mut HashMap<String, CacheEntry>) {
    let Some(key) = store
        .iter()
        .min_by_key(|(_, entry)| entry.expires_at)
        .map(|(key, _)| key.clone())
    else {
        return;
    };
    store.remove(&key);
}

#[cfg(test)]
mod tests {
    use super::RuntimeCache;

    #[tokio::test]
    async fn memory_cache_set_get_delete_prefix() {
        let cache = RuntimeCache::new(16);
        cache.set("a:1", "v1".to_string(), 30).await;
        cache.set("a:2", "v2".to_string(), 30).await;
        cache.set("b:1", "v3".to_string(), 30).await;

        let v1 = cache.get::<String>("a:1").await;
        let v2 = cache.get::<String>("a:2").await;
        let v3 = cache.get::<String>("b:1").await;
        assert_eq!(v1.as_deref(), Some("v1"));
        assert_eq!(v2.as_deref(), Some("v2"));
        assert_eq!(v3.as_deref(), Some("v3"));

        cache.delete_prefix("a:").await;
        assert!(cache.get::<String>("a:1").await.is_none());
        assert!(cache.get::<String>("a:2").await.is_none());
        assert_eq!(cache.get::<String>("b:1").await.as_deref(), Some("v3"));
    }

    #[tokio::test]
    async fn memory_cache_ttl_expire() {
        let cache = RuntimeCache::new(8);
        cache.set("ttl:key", 42_i64, 1).await;
        assert_eq!(cache.get::<i64>("ttl:key").await, Some(42));
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        assert!(cache.get::<i64>("ttl:key").await.is_none());
    }
}
