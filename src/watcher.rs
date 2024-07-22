use crate::context::Context;
use crate::store::DataStore;
use crate::AsyncResult;

use futures_util::StreamExt;
use redis::{from_redis_value, AsyncCommands, Client, FromRedisValue, RedisResult, Value};
use tokio::time::Duration;
use tracing::{error, info};

use std::sync::Arc;

const KEY_SPACE: &str = "__keyspace@0__:ads:*";

struct PersistentRedisPubSub {
    client: Arc<Client>,
    retry: Duration,
    pattern: String,
}

impl PersistentRedisPubSub {
    pub fn new(client: Arc<Client>, retry: Duration, pattern: String) -> Self {
        Self {
            client,
            retry,
            pattern,
        }
    }

    async fn wait(&self) {
        tokio::time::sleep(self.retry).await;
    }

    pub async fn on_message(&self, context: Context) {
        loop {
            // Get a connection with automatic recovery
            let mut subscriber = 'ps: loop {
                #[allow(deprecated)]
                let conn = self.client.get_async_connection().await;
                match conn {
                    Ok(conn) => {
                        let mut ps = conn.into_pubsub();
                        match ps.psubscribe(&self.pattern).await {
                            Ok(_) => {
                                break 'ps ps;
                            }
                            Err(e) => {
                                error!("psubscribe failed, error: {:?}", e);
                                self.wait().await;
                                continue 'ps;
                            }
                        }
                    }
                    Err(e) => {
                        error!("connection to Redis failed, error: {:?}", e);
                        self.wait().await;
                        continue 'ps;
                    }
                }
            };

            // Subscribe
            let mut stream = subscriber.on_message();
            'sub: loop {
                if let Some(ev) = stream.next().await {
                    let key = ev.get_channel::<String>().unwrap_or_default();
                    if key != self.pattern {
                        return;
                    }
                    if ev.get_payload::<String>().unwrap_or_default().as_str() == "set" {
                        extract(context.clone()).await;
                    }
                } else {
                    info!("subscriber's been dropped. A new one will be established");
                    self.wait().await;
                    break 'sub;
                }
            }
        }
    }
}

impl FromRedisValue for crate::store::Ads {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let json: String = from_redis_value(v)?;
        let ads = serde_json::from_str(&json)?;
        Ok(ads)
    }
}

async fn extract(context: Context) {
    if let Ok(mut conn) = context
        .redis_client
        .get_multiplexed_async_connection()
        .await
    {
        match conn.get::<&str, crate::store::Ads>("ads:all").await {
            Ok(value) => {
                let ads = value
                    .0
                    .iter()
                    .map(|x| Arc::new(x.clone()))
                    .collect::<Vec<Arc<crate::store::Ad>>>();
                DataStore::update(context, ads).await;
            }
            Err(e) => error!("failed to parse `ads::all`, error: {}", e),
        }
        return;
    }

    error!("failed to get async connection");
}

fn watch(context: Context) {
    let pub_sub = PersistentRedisPubSub::new(
        context.redis_client.clone(),
        Duration::from_secs(5),
        KEY_SPACE.to_owned(),
    );

    tokio::spawn(async move {
        pub_sub.on_message(context.clone()).await;
    });
    info!("watcher initialized");
}

pub(crate) async fn run(context: Context) -> AsyncResult {
    extract(context.clone()).await;
    watch(context.clone());
    Ok(())
}
