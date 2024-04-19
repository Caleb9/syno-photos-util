use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use syno_photos_util::{Cli, ClientBuilder, FsImpl, IoImpl};

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Warn)
        .without_timestamps()
        .env()
        .init()?;
    let cli = Cli::parse();
    let mut io = IoImpl::new();
    let client = ClientBuilder::default()
        .timeout(cli.timeout_seconds)
        .build()?;
    syno_photos_util::run(cli, &mut io, &client, &FsImpl).await
}
