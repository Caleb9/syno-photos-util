use crate::commands::api_client::{ApiClient, SessionClient, Space};
use crate::commands::{DsmError, album_not_found, find_album};
use crate::conf::Conf;
use crate::http::HttpClient;
use crate::io::Io;
use anyhow::{Context, Result, anyhow, bail};
use futures::stream::{self, StreamExt};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use syno_api::foto::error::PhotoError;
use syno_api::foto::{
    browse::{folder::dto::Folder, item::dto::Item},
    setting::user::dto::UserSettings,
    user_info::dto::UserInfo,
};

mod api_client;

pub async fn handle<C: HttpClient, I: Io>(
    album_name: &str,
    conf: &Conf,
    client: &C,
    io: &mut I,
) -> Result<()> {
    if !conf.is_logged_in() {
        bail!("you are not signed in to DSM, use the 'login' command (see '--help' for details)");
    }
    let client = SessionClient::new(conf.session.as_ref().unwrap(), client);
    let user_settings = client.get_user_settings().await?;
    let team_space_settings = client.get_team_space_settings().await?;

    let album = find_album(album_name, &user_settings, &team_space_settings, &client).await?;
    match album {
        Some(album) => {
            let photos = client
                .list_items(&album, album.item_count())
                .await
                .with_context(|| "listing album contents failed")?;

            let folder_ids: HashSet<u32> = photos.iter().map(|p| p.folder_id).collect();
            let folders_future = get_folder_results(folder_ids, &user_settings, &client);

            let owner_ids: HashSet<u32> = photos.iter().map(|p| p.owner_user_id).collect();
            let users = client.get_users(&owner_ids).await?;
            let user_map: HashMap<u32, UserInfo> = users.into_iter().map(|u| (u.id, u)).collect();

            let folder_results = folders_future.await;
            let photo_to_folder_result_map = map_photo_to_folder_result(photos, &folder_results);
            print_results(photo_to_folder_result_map, user_map, io)
        }
        None => {
            let matching_albums = client.suggest_albums(album_name).await.unwrap_or_else(|e| {
                log::warn!("suggest album search error: {e}");
                vec![]
            });
            album_not_found(album_name, matching_albums, io)
        }
    }
}

async fn get_folder_results<C: ApiClient>(
    folder_ids: HashSet<u32>,
    UserSettings {
        enable_home_service,
        team_space_permission,
        ..
    }: &UserSettings,
    client: &SessionClient<'_, C>,
) -> HashMap<u32, Result<Folder>> {
    let get_folder_result = |folder_id| async move {
        let folder_result = match (enable_home_service, team_space_permission.as_str()) {
            (false, "none") => {
                /* user does not have access to neither Personal nor Shared Space */
                Err(anyhow!("no access"))
            }
            (true, "none") => {
                /* user only has access to Personal Space */
                client.get_folder_by_id((folder_id, Space::Personal)).await
            }
            (false, _some) => {
                /* user only has access to Shared Space */
                client.get_folder_by_id((folder_id, Space::Shared)).await
            }
            (true, _some) => {
                /* user has access to both spaces, try them in sequence */
                match client.get_folder_by_id((folder_id, Space::Personal)).await {
                    Ok(folder) => Ok(folder),
                    _ => client.get_folder_by_id((folder_id, Space::Shared)).await,
                }
            }
        };
        (folder_id, folder_result)
    };

    const CONCURRENT_REQUESTS: usize = 8;
    stream::iter(folder_ids)
        .map(get_folder_result)
        .buffer_unordered(CONCURRENT_REQUESTS)
        .collect()
        .await
}

fn map_photo_to_folder_result(
    photos: Vec<Item>,
    folder_results: &HashMap<u32, Result<Folder>>,
) -> HashMap<Item, &Result<Folder>> {
    photos
        .into_iter()
        .map(|photo| {
            let folder_id = photo.folder_id;
            (
                photo,
                folder_results.get(&folder_id).expect("folder should exist"),
            )
        })
        .collect()
}

fn print_results<I: Io>(
    photo_to_folder_result_map: HashMap<Item, &Result<Folder>>,
    user_map: HashMap<u32, UserInfo>,
    io: &mut I,
) -> Result<()> {
    for (
        Item {
            filename,
            owner_user_id,
            ..
        },
        folder_result,
    ) in photo_to_folder_result_map
    {
        const SHARED_SPACE: &str = "Shared Space";
        let mut owner = user_map
            .get(&owner_user_id)
            .expect("user should be fetched")
            .name
            .as_str();
        /* Photos in Shared Space have owner name set to "/volume1/photo" (or similar) */
        if owner.starts_with("/volume") && owner.ends_with("/photo") {
            owner = SHARED_SPACE;
        }
        match folder_result {
            Ok(folder) => {
                /* The following assumes standard locations, not sure if it's possible to have the
                 * service folder links in different locations on DSM 7. */
                let prefix = match owner {
                    SHARED_SPACE => "/var/services/photo".to_string(),
                    _ => format!("/var/services/homes/{owner}/Photos"),
                };
                let sub_folder = folder.name.trim_end_matches('/');
                writeln!(io.stdout(), "{prefix}{sub_folder}/{filename}")?;
            }
            Err(e) => {
                let e_str = match e.downcast_ref::<DsmError>() {
                    Some(DsmError::Photo(PhotoError::NoAccessOrNotFound)) => {
                        format!("no access (owned by {owner})")
                    }
                    _ => e.to_string(),
                };
                writeln!(io.stdout(), "Error: {e_str} '{filename}'")?;
            }
        };
    }
    Ok(())
}
