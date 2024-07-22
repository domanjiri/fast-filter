use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

const FILE_NAME: &str = "/tmp/engine.log"; // TODO: From config

pub(crate) fn init() -> anyhow::Result<()> {
    let writer = std::fs::File::create(FILE_NAME)?;

    let subscriber = FmtSubscriber::builder()
        .with_ansi(false)
        .with_writer(writer)
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("logger initialized");
    Ok(())
}
