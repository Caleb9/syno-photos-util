use anyhow::Result;
use clap::Parser;
use log::LevelFilter;
use reqwest::cookie::Jar;
use reqwest::ClientBuilder;
use simple_logger::SimpleLogger;
use std::sync::Arc;
use syno_photos_util::{Cli, CookieClient, FsImpl, IoImpl};

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Warn)
        .without_timestamps()
        .env()
        .init()?;

    let cli = Cli::parse();

    let mut io = IoImpl::new();

    let cookie_store = Arc::new(Jar::default());
    let mut client = CookieClient {
        client: ClientBuilder::default()
            .cookie_provider(cookie_store.clone())
            .timeout(cli.timeout_seconds)
            .build()?,
        cookie_store,
    };

    /* This crate version */
    let installed_version = env!("CARGO_PKG_VERSION");

    syno_photos_util::run(cli, &mut io, &mut client, &FsImpl, installed_version).await
}
