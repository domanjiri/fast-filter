use crate::store::DataStore;

use arc_swap::ArcSwap;
use chrono::{offset::Local, Timelike};
use redis::Client;
use tokio::time::Duration;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

/// Used for peridic tasks
const TICK_INTERVAL_SECS: u64 = 5;

#[derive(Debug, Clone)]
pub(crate) struct Context {
    // Cached time with appropriate precision to reduce number of syscall
    pub cached_time: Arc<CachedTime>,

    // A lock-free container to store ads and filters
    pub inventory: Arc<ArcSwap<DataStore>>,

    // Persistant data stores
    pub redis_client: Arc<Client>,
    pub mysql_pool: mysql_async::Pool,
}

impl Context {
    pub fn new(redis_conn: &str, mysql_conn: &str) -> Self {
        let cached_time = Arc::new(Default::default());
        let inventory = Arc::new(ArcSwap::new(Arc::new(DataStore::new())));
        let redis_client = Arc::new(redis::Client::open(redis_conn).unwrap());
        let mysql_pool = mysql_async::Pool::new(mysql_conn);

        Self {
            cached_time,
            inventory,
            redis_client,
            mysql_pool,
        }
    }

    pub fn ticker(&self) {
        let context = self.clone();

        tokio::spawn(async move {
            loop {
                let context = context.clone();

                // Updating cached time
                let now = Local::now();
                context
                    .cached_time
                    .hour
                    .store(now.hour() as usize, Ordering::Relaxed);
                drop(context);

                // Put the task into the queue and yield
                tokio::time::sleep(Duration::from_secs(TICK_INTERVAL_SECS)).await;
            }
        });
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        std::mem::drop(mysql_async::Pool::disconnect(self.mysql_pool.clone()));
    }
}

#[derive(Default, Debug)]
pub(crate) struct CachedTime {
    pub hour: AtomicUsize,
}

impl CachedTime {
    pub fn hour(&self) -> usize {
        self.hour.load(Ordering::Relaxed)
    }
}
