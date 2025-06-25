use crate::commands::api_client::{ApiClient, SessionClient, Space};
use crate::io::Io;
use anyhow::Result;
use std::io::Write;
pub use syno_api::error::Error as DsmError;
use syno_api::foto::browse::album::dto::Album as AlbumDto;
use syno_api::foto::browse::person::dto::Person as PersonDto;
use syno_api::foto::search::dto::Search;
use syno_api::foto::setting::team_space::dto::TeamSpaceSettings;
use syno_api::foto::setting::user::dto::UserSettings;

mod api_client;
pub mod check_update;
mod error;
pub mod export;
pub mod list;
pub mod login;
pub mod logout;
pub mod status;

// TODO add support for places album
#[derive(Debug)]
pub enum Album {
    Normal(AlbumDto),
    Person(PersonDto, Space),
}

impl Album {
    pub fn item_count(&self) -> u32 {
        match self {
            Album::Normal(a) => a.item_count,
            Album::Person(p, _) => p.item_count,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Album::Normal(a) => a.name.as_str(),
            Album::Person(p, _) => p.name.as_str(),
        }
    }

    pub fn id_param(&self) -> (&'static str, String) {
        match self {
            Album::Normal(a) => {
                if !a.passphrase.is_empty() {
                    ("passphrase", a.passphrase.to_owned())
                } else {
                    ("album_id", a.id.to_string())
                }
            }
            Album::Person(p, _) => ("person_id", p.id.to_string()),
        }
    }
}

/// Search the API for album or person named `album_name` (case-insensitive)
async fn find_album<C: ApiClient>(
    album_name: &str,
    user_settings: &UserSettings,
    team_space_settings: &TeamSpaceSettings,
    client: &SessionClient<'_, C>,
) -> Result<Option<Album>> {
    /// Search among shared albums. There is no known API method to detect the number of
    /// shared albums, we need to query the list in chunks until we find it or there are no
    /// more albums returned.
    async fn find_shared_album<C: ApiClient>(
        album_name: &str,
        client: &SessionClient<'_, C>,
    ) -> Result<Option<AlbumDto>> {
        let mut offset = 0;
        const LIMIT: u32 = 50;
        loop {
            let shared_album_list = client
                .list_shared_with_me_albums(offset, offset + LIMIT)
                .await?;
            if shared_album_list.is_empty() {
                return Ok(None);
            }
            let shared_album = shared_album_list
                .into_iter()
                .find(|a| a.name.eq_ignore_ascii_case(album_name));
            match shared_album {
                Some(a) => return Ok(Some(a)),
                None => offset += LIMIT,
            }
        }
    }

    async fn find_person_album<C: ApiClient>(
        album_name: &str,
        space: Space,
        client: &SessionClient<'_, C>,
    ) -> Result<Option<PersonDto>> {
        let limit = client.count_people(space).await?;
        let people = client.list_people(space, limit).await?;
        let person = people
            .into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(album_name));
        Ok(person)
    }

    let owned_albums_count = client.count_owned_albums().await?;
    let owned_album = if owned_albums_count > 0 {
        let owned_album_list = client.list_owned_albums(owned_albums_count).await?;
        owned_album_list
            .into_iter()
            .find(|a| a.name.eq_ignore_ascii_case(album_name))
    } else {
        None
    };
    if let Some(album) = owned_album {
        return Ok(Some(Album::Normal(album)));
    }
    let shared_album = find_shared_album(album_name, client).await?;
    if let Some(album) = shared_album {
        return Ok(Some(Album::Normal(album)));
    }
    if user_settings.enable_person {
        let private_space_person_album = find_person_album(album_name, Space::Personal, client)
            .await?
            .map(|p| Album::Person(p, Space::Personal));
        if let Some(person_album) = private_space_person_album {
            return Ok(Some(person_album));
        }
    }
    if let Some(true) = team_space_settings.enable_person {
        let shared_space_person_album = find_person_album(album_name, Space::Shared, client)
            .await?
            .map(|p| Album::Person(p, Space::Shared));
        return Ok(shared_space_person_album);
    }
    Ok(None)
}

/// Print album-not-found information and suggest albums containing `album_name` in their name.
fn album_not_found<I: Io>(
    album_name: &str,
    matching_albums: Vec<Search>,
    io: &mut I,
) -> Result<()> {
    writeln!(io.stdout(), "Album '{album_name}' not found.")?;
    if !matching_albums.is_empty() {
        writeln!(io.stdout(), "Other album names containing '{album_name}':")?;
        for a in matching_albums {
            writeln!(io.stdout(), "- \"{}\" ({})", a.name, a.r#type)?;
        }
    }
    Ok(())
}
