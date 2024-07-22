mod context;
mod engine;
mod logger;
mod store;
mod watcher;

mod proto {
    tonic::include_proto!("filler");
}

pub(crate) type AsyncResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

const REDIS_CONN: &str = "redis://127.0.0.1:6379/0";
const MYSQL_CONN: &str = "mysql://root:123456@localhost:8306/mydb";

#[tokio::main]
async fn main() -> AsyncResult {
    logger::init().unwrap();

    let context = context::Context::new(REDIS_CONN, MYSQL_CONN);

    // Peridic tasks
    context.ticker();
    // Extract data from data-stores, and watch the changes
    watcher::run(context.clone()).await?;
    // gRPC Handler
    engine::run(context).await?;

    Ok(())
}
