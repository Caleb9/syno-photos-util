use crate::http::HeaderValue;
pub use crate::{cli::Cli, fs::FsImpl, http::CookieClient, io::IoImpl};
use crate::{
    cli::Command,
    commands::{export, list, login, logout, status},
    conf::Conf,
    fs::Fs,
    http::{CookieStore, HttpClient},
    io::Io,
};
use anyhow::Result;

mod cli;
mod commands;
mod conf;
mod fs;
mod http;
mod io;

#[cfg(test)]
mod test;

pub async fn run<I: Io, C: HttpClient, S: CookieStore, F: Fs>(
    cli: Cli,
    io: &mut I,
    client: &mut CookieClient<C, S>,
    fs: &F,
) -> Result<()> {
    let mut conf = Conf::try_load(fs).unwrap_or_else(Conf::new);
    if let Some(session) = &conf.session {
        let cookie = HeaderValue::from_str(session.cookie.as_str())?;
        client
            .cookie_store
            .set_cookies(&mut [cookie].iter(), &session.url);
    }
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
        Command::List { album_name } => {
            list::handle(album_name.as_str(), &conf, &client.client, io).await
        }
        Command::Export {
            album_name,
            folder_path,
        } => {
            export::handle(
                album_name.as_str(),
                folder_path.as_str(),
                &conf,
                &client.client,
                io,
            )
            .await
        }
        Command::Logout { forget } => logout::handle(conf, forget, fs),
    }
}
