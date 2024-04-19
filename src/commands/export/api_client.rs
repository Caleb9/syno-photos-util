//! Extra methods for SessionClient used by export command

use crate::commands::api_client::{ApiClient, ApiQueryParams, SessionClient, Space};
use crate::conf::Session;
use anyhow::Result;
use serde::Deserialize;
use syno_api::dto::List;
use syno_api::foto::background_task::file::dto::TaskInfo;
use syno_api::foto::{self, browse::folder::dto::Folder};
use syno_api::foto_team;

impl<'a, C: ApiClient> SessionClient<'a, C> {
    pub async fn get_folder_by_name(&self, name: &str) -> Result<Folder> {
        #[derive(Debug, Deserialize)]
        struct FolderContainer {
            folder: Folder,
        }

        let Session {
            url,
            id: session_id,
        } = self.session;
        let folder: FolderContainer = self
            .client
            .get(
                url.clone(),
                ApiQueryParams::new(foto::browse::folder::API, "get", 1, session_id),
                &[("name", name)],
            )
            .await?;
        Ok(folder.folder)
    }

    pub async fn copy_photos(
        &self,
        photo_ids: &[u32],
        photos_space: Space,
        target_folder_id: u32,
    ) -> Result<TaskInfo> {
        #[derive(Debug, Deserialize)]
        struct TaskContainer {
            task_info: TaskInfo,
        }

        let api = match photos_space {
            Space::Personal => foto::background_task::file::API,
            Space::Shared => foto_team::background_task::file::API,
        };
        let Session {
            url,
            id: session_id,
        } = self.session;
        let ids = photo_ids
            .iter()
            .map(u32::to_string)
            .reduce(|acc, id| format!("{acc},{id}"))
            .expect("photo_ids should not be empty");
        let task: TaskContainer = self
            .client
            .post(
                url.clone(),
                ApiQueryParams::new(api, "copy", 1, session_id),
                &[
                    ("target_folder_id", target_folder_id.to_string().as_str()),
                    ("item_id", format!("[{ids}]").as_str()),
                    ("action", "skip"),
                    ("folder_id", "[]"),
                ],
            )
            .await?;
        Ok(task.task_info)
    }

    pub async fn get_task_status(&self, task_ids: &[u32]) -> Result<Vec<TaskInfo>> {
        let Session {
            url,
            id: session_id,
        } = self.session;
        let ids = task_ids
            .iter()
            .map(u32::to_string)
            .reduce(|acc, id| format!("{acc},{id}"))
            .expect("task_ids should not be empty");
        let task_infos: List<TaskInfo> = self
            .client
            .get(
                url.clone(),
                ApiQueryParams::new(
                    foto::background_task::info::API,
                    "get_status",
                    1,
                    session_id,
                ),
                &[("id", format!("[{ids}]").as_str())],
            )
            .await?;
        Ok(task_infos.list)
    }
}
