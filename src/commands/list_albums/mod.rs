use anyhow::{Result, bail};
use std::io::Write;
use syno_api::foto::browse::album::dto::Album;

use crate::commands::api_client::{ApiClient, SessionClient};
use crate::conf::Conf;
use crate::http::HttpClient;
use crate::io::Io;

pub async fn handle<C: HttpClient, I: Io>(conf: &Conf, client: &C, io: &mut I) -> Result<()> {
    if !conf.is_logged_in() {
        bail!("you are not signed in to DSM, use the 'login' command (see '--help' for details)");
    }
    let client = SessionClient::new(conf.session.as_ref().unwrap(), client);
    let owned_albums_count = client.count_owned_albums().await?;
    let owned_albums_list = client.list_owned_albums(owned_albums_count).await?;
    for owned_album in owned_albums_list {
        writeln!(io.stdout(), "{}", owned_album.name)?;
    }
    let shared_albums_list = list_shared_albums(&client).await?;
    for shared_album in shared_albums_list {
        writeln!(io.stdout(), "{}", shared_album.name)?;
    }
    Ok(())
}

/// Search among shared albums. There is no known API method to detect the number of
/// shared albums, we need to query the list in chunks until we find it or there are no
/// more albums returned.
async fn list_shared_albums<C: ApiClient>(client: &SessionClient<'_, C>) -> Result<Vec<Album>> {
    let mut offset = 0;
    const LIMIT: u32 = 50;
    let mut shared_albums = vec![];
    loop {
        let mut shared_album_list = client
            .list_shared_with_me_albums(offset, offset + LIMIT)
            .await?;
        shared_albums.append(&mut shared_album_list);
        if shared_album_list.len() < LIMIT as usize {
            break;
        }
        offset += LIMIT;
    }
    Ok(shared_albums)
}
