pub use crate::{cli::Cli, fs::FsImpl, io::IoImpl};
use crate::{
    cli::Command,
    commands::{export, list, login, logout, status},
    conf::Conf,
    fs::Fs,
    http::HttpClient,
    io::Io,
};
use anyhow::Result;
pub use reqwest::ClientBuilder;

mod cli;
mod commands;
mod conf;
mod fs;
mod http;
mod io;

#[cfg(test)]
mod test;

pub async fn run<I: Io, C: HttpClient, F: Fs>(
    cli: Cli,
    io: &mut I,
    client: &C,
    fs: &F,
) -> Result<()> {
    let mut conf = Conf::try_load(fs).unwrap_or_else(Conf::new);
    match cli.command {
        Command::Login {
            dsm_url,
            user,
            password,
            remember,
        } => {
            login::handle(
                dsm_url,
                (user, password, remember),
                &mut conf,
                client,
                io,
                fs,
            )
            .await
        }
        Command::Status => status::handle(&conf, io),
        Command::List { album_name } => list::handle(album_name.as_str(), &conf, client, io).await,
        Command::Export {
            album_name,
            folder_path,
        } => export::handle(album_name.as_str(), folder_path.as_str(), &conf, client, io).await,
        Command::Logout { forget } => logout::handle(conf, forget, fs),
    }
}
