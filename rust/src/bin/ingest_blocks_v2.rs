use env_logger::Builder;
use log::info;

#[tokio::main]
async fn main() {
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();

    info!("Hello world");
}
