//! Extra methods for SessionClient used by list command

use crate::commands::api_client::{ApiClient, ApiParams, SessionClient, Space};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use syno_api::dto::List;
use syno_api::foto::{self, browse::folder::dto::Folder, user_info::dto::UserInfo};
use syno_api::foto_team;

impl<'a, C: ApiClient> SessionClient<'a, C> {
    pub async fn get_folder_by_id(&self, (id, space): (u32, Space)) -> Result<Folder> {
        #[derive(Debug, Deserialize, Serialize)]
        struct FolderContainer {
            folder: Folder,
        }

        let api = match space {
            Space::Personal => foto::browse::folder::API,
            Space::Shared => foto_team::browse::folder::API,
        };
        let folder: FolderContainer = self
            .client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(api, "get", 1),
                &[("id", id.to_string().as_str())],
            )
            .await?;
        Ok(folder.folder)
    }

    pub async fn get_users(&self, user_ids: &HashSet<u32>) -> Result<Vec<UserInfo>> {
        let ids = user_ids
            .iter()
            .map(u32::to_string)
            .reduce(|acc, id| format!("{acc},{id}"))
            .expect("user ids should not be empty");
        let users: List<UserInfo> = self
            .client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(foto::user_info::API, "get", 1),
                &[("id", format!("[{ids}]").as_str())],
            )
            .await?;
        Ok(users.list)
    }
}
