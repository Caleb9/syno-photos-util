use crate::commands::api_client::{ApiClient, SessionClient, Space};
use crate::commands::{album_not_found, find_album, Album, DsmError};
use crate::conf::Conf;
use crate::http::HttpClient;
use crate::io::Io;
use anyhow::{bail, Result};
use std::io::Write;
use std::time::Duration;
use syno_api::foto::background_task::file::dto::TaskInfo;
use syno_api::foto::browse::item::dto::Item;
use syno_api::foto::error::PhotoError;
use syno_api::foto::setting::user::dto::UserSettings;
use syno_api::foto_team::browse::folder::Folder;
#[cfg(test)]
use test::fake_sleep as sleep;
#[cfg(not(test))]
use tokio::time::sleep;

mod api_client;

/// * `target_folder_path` - target folder in Personal Space (must exist)
pub async fn handle<C: HttpClient, I: Io>(
    album_name: &str,
    target_folder_path: &str,
    create_folder: bool,
    conf: &Conf,
    client: &C,
    io: &mut I,
) -> Result<()> {
    if !conf.is_logged_in() {
        bail!("you are not signed in to DSM, use the 'login' command (see '--help' for details)");
    }
    let client = SessionClient::new(conf.session.as_ref().unwrap(), client);

    let user_settings = client.get_user_settings().await?;
    if !user_settings.enable_home_service {
        bail!("home service not enabled on DSM, Personal Space not available in Synology Photos")
    }

    let folder_path = format!("/{}", target_folder_path.trim().trim_matches('/'));
    log::info!("target folder: {folder_path}");
    let folder_future = client.get_folder_by_name(folder_path.as_str());

    let find_album_future = find_album(album_name, &user_settings, &client);

    let folder = match folder_future.await {
        Ok(folder) => folder,
        Err(error) => match error.downcast::<DsmError>()? {
            DsmError::Photo(PhotoError::NoAccessOrNotFound) if create_folder => {
                create_folder_path(folder_path.as_str(), &client).await?
            }
            DsmError::Photo(PhotoError::NoAccessOrNotFound) => {
                bail!("folder '{target_folder_path}' does not exist in Personal Space")
            }
            other => bail!(other),
        },
    };

    match find_album_future.await? {
        Some(album) => export((album, folder, user_settings), &client, io).await,
        None => {
            let matching_albums = client.suggest_albums(album_name).await.unwrap_or_else(|e| {
                log::warn!("suggest album search error: {e}");
                vec![]
            });
            album_not_found(album_name, matching_albums, io)
        }
    }
}

async fn create_folder_path<C: ApiClient>(
    folder_path: &str,
    client: &SessionClient<'_, C>,
) -> Result<Folder> {
    let path_segments: Vec<_> = folder_path.split('/').filter(|s| !s.is_empty()).collect();
    if path_segments.iter().any(|s| s.trim().is_empty()) {
        bail!("{folder_path} is not valid folder path");
    }
    let mut result_folder = client.get_folder_by_name("/").await?;
    let mut path_so_far = String::new();
    let mut exists = true;
    for segment in path_segments {
        path_so_far.push('/');
        path_so_far.push_str(segment);
        if exists {
            let folder_result = client.get_folder_by_name(path_so_far.as_str()).await;
            match folder_result {
                Ok(folder) => result_folder = folder,
                Err(error) => match error.downcast::<DsmError>()? {
                    DsmError::Photo(PhotoError::NoAccessOrNotFound) => exists = false,
                    other => bail!(other),
                },
            }
        }
        if !exists {
            result_folder = client.create_folder(segment, result_folder.id).await?;
            log::info!("created {path_so_far} folder")
        }
    }
    Ok(result_folder)
}

async fn export<C: ApiClient, I: Io>(
    (album, target_folder, user_settings): (Album, Folder, UserSettings),
    client: &SessionClient<'_, C>,
    io: &mut I,
) -> Result<()> {
    debug_assert!(user_settings.enable_home_service);
    let photos = client.list_items(&album, album.item_count()).await?;
    writeln!(
        io.stdout(),
        "Copying {} items from album '{}' to folder '{}' in Personal Space",
        photos.len(),
        album.name(),
        target_folder.name
    )?;

    let copy_personal_space_photos_future =
        copy_personal_space_photos(&photos, target_folder.id, client);
    let copy_shared_space_photos_future =
        copy_shared_space_photos(&photos, target_folder.id, &user_settings, client);

    process_task_info(
        vec![
            copy_personal_space_photos_future.await,
            copy_shared_space_photos_future.await,
        ],
        client,
        io,
    )
    .await?;

    Ok(())
}

async fn copy_personal_space_photos<C: ApiClient>(
    photos: &[Item],
    target_folder_id: u32,
    client: &SessionClient<'_, C>,
) -> Result<Option<TaskInfo>> {
    let personal_space_photo_ids: Vec<_> = photos
        .iter()
        .filter(|p| p.owner_user_id != 0)
        .map(|p| p.id)
        .collect();
    if personal_space_photo_ids.is_empty() {
        return Ok(None);
    }
    let task_info = client
        .copy_photos(&personal_space_photo_ids, Space::Personal, target_folder_id)
        .await?;
    Ok(Some(task_info))
}

async fn copy_shared_space_photos<C: ApiClient>(
    photos: &[Item],
    target_folder_id: u32,
    user_settings: &UserSettings,
    client: &SessionClient<'_, C>,
) -> Result<Option<TaskInfo>> {
    let shared_space_access = user_settings.team_space_permission != "none";
    let shared_space_photo_ids: Vec<_> = photos
        .iter()
        .filter(|p| p.owner_user_id == 0)
        .map(|p| p.id)
        .collect();
    if !shared_space_access && !shared_space_photo_ids.is_empty() {
        log::warn!(
            "album contains items from Shared Space, but you don't have access to it; \
             skipping {} item(s)",
            shared_space_photo_ids.len()
        );
    }
    if !shared_space_access || shared_space_photo_ids.is_empty() {
        return Ok(None);
    }

    let task_info = client
        .copy_photos(&shared_space_photo_ids, Space::Shared, target_folder_id)
        .await?;
    Ok(Some(task_info))
}

/// Wait for copy tasks to finish, reporting results. This requires polling the API.
async fn process_task_info<C: ApiClient, I: Io>(
    task_info_results: Vec<Result<Option<TaskInfo>>>,
    client: &SessionClient<'_, C>,
    io: &mut I,
) -> Result<()> {
    let (tasks, errs): (Vec<_>, Vec<_>) = task_info_results.into_iter().partition(|r| r.is_ok());
    for error in errs.into_iter().map(Result::unwrap_err) {
        writeln!(io.stdout(), "Error: {error}")?;
    }
    let mut task_ids: Vec<_> = tasks
        .into_iter()
        .filter_map(Result::unwrap)
        .map(|t| t.id)
        .collect();
    let (mut copied, mut skipped, mut failed, mut aborted) = (0, 0, 0, 0);
    let mut dot_print_counter = 0;
    loop {
        if task_ids.is_empty() {
            writeln!(io.stdout())?;
            break;
        }
        sleep(Duration::from_secs(1)).await;

        if dot_print_counter == 3 {
            write!(io.stdout(), "\r   \r")?;
            dot_print_counter = 0;
        } else {
            write!(io.stdout(), ".")?;
            dot_print_counter += 1;
        }
        io.stdout().flush()?;

        let updated_task_infos = client.get_task_status(&task_ids).await?;
        let (done, processing): (Vec<_>, Vec<_>) = updated_task_infos.iter().partition(|t| {
            t.status != "waiting" && t.status != "processing" && t.status != "aborting"
        });
        task_ids.clear();
        task_ids.append(&mut processing.into_iter().map(|t| t.id).collect::<Vec<u32>>());

        for t in done {
            copied += t.completion - t.skip - t.error;
            skipped += t.skip;
            failed += t.error;
            aborted += t.total - (t.completion + t.skip + t.error)
        }
    }
    if failed != 0 {
        log::warn!(
            "export failed ({failed} item(s) not copied); \
            inspect Synology Photos web interface for details"
        );
    }
    writeln!(
        io.stdout(),
        "Export summary: {copied} copied, {skipped} skipped, {failed} failed, {aborted} canceled"
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    pub(super) async fn fake_sleep(_: Duration) {}
}
