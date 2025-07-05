use super::{Album, DsmError};
use crate::commands::error::HttpError;
use crate::conf::Session;
use crate::http::{HttpClient, HttpResponse, Url};
use anyhow::{Result, bail};
use reqwest::IntoUrl;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::future::Future;
use syno_api::dto::{ApiResponse, List};
use syno_api::foto::browse::album::dto::Album as AlbumDto;
use syno_api::foto::browse::item::dto::Item;
use syno_api::foto::browse::person::dto::Person;
use syno_api::foto::search::dto::Search;
use syno_api::foto::setting::{team_space::dto::TeamSpaceSettings, user::dto::UserSettings};
use syno_api::{foto, foto_team};

/// Trait to add `get` and `post` methods to `HttpClient` which take parameters required by Synology
/// Photos API
pub trait ApiClient {
    fn get<U, R>(
        &self,
        url: U,
        common_query_params: ApiParams,
        query_params: &[(&str, &str)],
    ) -> impl Future<Output = Result<R>>
    where
        U: IntoUrl,
        R: DeserializeOwned + 'static;

    fn post<U, R>(
        &self,
        url: U,
        common_params: ApiParams,
        form_params: &[(&str, &str)],
    ) -> impl Future<Output = Result<R>>
    where
        U: IntoUrl,
        R: DeserializeOwned + 'static;
}

impl<C: HttpClient> ApiClient for C {
    async fn get<U, R>(
        &self,
        url: U,
        ApiParams {
            api,
            method,
            version,
        }: ApiParams<'_>,
        params: &[(&str, &str)],
    ) -> Result<R>
    where
        U: IntoUrl,
        R: DeserializeOwned + 'static,
    {
        let mut url = url.into_url()?;
        let path = url.path().trim_end_matches('/');
        url.set_path(format!("{path}/webapi/entry.cgi").as_str());
        let mut query = format!(
            "api={}&\
            method={}&\
            version={}",
            api, method, version
        );
        for (key, value) in params {
            query.push_str(format!("&{key}={value}").as_str());
        }
        url.set_query(query.as_str().into());
        let response = C::get(self, url).await?;
        let data_dto = try_deserialize_response_content(response).await?;
        Ok(data_dto)
    }

    async fn post<U, R>(
        &self,
        url: U,
        ApiParams {
            api,
            method,
            version,
        }: ApiParams<'_>,
        params: &[(&str, &str)],
    ) -> Result<R>
    where
        U: IntoUrl,
        R: DeserializeOwned + 'static,
    {
        let mut url = url.into_url()?;
        let path = url.path().trim_end_matches('/');
        url.set_path(format!("{path}/webapi/entry.cgi").as_str());
        url.set_query(Some(format!("api={api}").as_str()));
        let version = version.to_string();
        let mut form = vec![("method", method), ("version", version.as_str())];
        for param in params {
            form.push(*param);
        }
        let response = C::post(self, url, &form).await?;
        let data_dto = try_deserialize_response_content(response).await?;
        Ok(data_dto)
    }
}

async fn try_deserialize_response_content<R, D>(response: R) -> Result<D>
where
    R: HttpResponse,
    D: DeserializeOwned + 'static,
{
    if !response.status().is_success() {
        bail!(HttpError(response.status()));
    }
    let dto = if cfg!(debug_assertions) {
        /* Print response body as text in debug builds */
        let response_str = response.text().await?;
        // dbg!(response_str.as_str());
        serde_json::from_str::<ApiResponse<D>>(response_str.as_str())?
    } else {
        response.json::<ApiResponse<D>>().await?
    };
    if let Some(syno_api::dto::Error { code }) = dto.error {
        bail!(DsmError::from(code));
    }
    assert!(dto.success);
    Ok(dto
        .data
        .expect("data should be populated on successful response"))
}

/// Request-parameters required by Synology Photos API
#[derive(Debug, Copy, Clone)]
pub struct ApiParams<'a> {
    api: &'a str,
    method: &'a str,
    version: u8,
}

impl<'a> ApiParams<'a> {
    pub fn new(api: &'a str, method: &'a str, version: u8) -> Self {
        Self {
            api,
            method,
            version,
        }
    }
}

/// Provides methods to query Synology Photos API when logged-in. Used by multiple commands.
pub struct SessionClient<'a, C> {
    pub(crate) dsm_url: &'a Url,
    pub(crate) client: &'a C,
}

impl<'a, C: ApiClient> SessionClient<'a, C> {
    pub fn new(session: &'a Session, client: &'a C) -> Self {
        SessionClient {
            dsm_url: &session.url,
            client,
        }
    }

    pub async fn get_user_settings(&self) -> Result<UserSettings> {
        self.client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(foto::setting::user::API, "get", 1),
                &[],
            )
            .await
    }

    pub async fn get_team_space_settings(&self) -> Result<TeamSpaceSettings> {
        self.client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(foto::setting::team_space::API, "get", 1),
                &[],
            )
            .await
    }

    pub async fn count_owned_albums(&self) -> Result<u32> {
        #[derive(Debug, Deserialize)]
        struct CountContainer {
            count: u32,
        }

        let data: CountContainer = self
            .client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(foto::browse::album::API, "count", 2),
                &[],
            )
            .await?;
        Ok(data.count)
    }

    pub async fn list_owned_albums(&self, limit: u32) -> Result<Vec<AlbumDto>> {
        let data: List<AlbumDto> = self
            .client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(foto::browse::album::API, "list", 2),
                &[("offset", "0"), ("limit", limit.to_string().as_str())],
            )
            .await?;
        Ok(data.list)
    }

    pub async fn list_shared_with_me_albums(
        &self,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<AlbumDto>> {
        let data: List<AlbumDto> = self
            .client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(foto::sharing::misc::API, "list_shared_with_me_album", 2),
                &[
                    ("offset", offset.to_string().as_str()),
                    ("limit", limit.to_string().as_str()),
                ],
            )
            .await?;
        Ok(data.list)
    }

    pub async fn count_people(&self, space: Space) -> Result<u32> {
        #[derive(Debug, Deserialize)]
        struct CountContainer {
            count: u32,
        }

        let data: CountContainer = self
            .client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(space.browse_person_api(), "count", 2),
                &[("show_more", true.to_string().as_str())],
            )
            .await?;
        Ok(data.count)
    }

    pub async fn list_people(&self, space: Space, limit: u32) -> Result<Vec<Person>> {
        let data: List<Person> = self
            .client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(space.browse_person_api(), "list", 1),
                &[("offset", "0"), ("limit", limit.to_string().as_str())],
            )
            .await?;
        Ok(data.list)
    }

    pub async fn list_items(&self, album: &Album, limit: u32) -> Result<Vec<Item>> {
        let (key, value) = album.id_param();
        let api = match album {
            Album::Normal(_) => foto::browse::item::API,
            Album::Person(_, space) => space.browse_item_api(),
        };
        let items: List<Item> = self
            .client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(api, "list", 1),
                &[
                    (key, value.as_str()),
                    ("offset", "0"),
                    ("limit", limit.to_string().as_str()),
                ],
            )
            .await?;
        Ok(items.list)
    }

    /// This is unreliable on the API side (returns errors e.g. when keyword starts with numbers,
    /// or just doesn't return anything in other scenarios...). Use only for informational purposes.
    pub async fn suggest_albums(&self, album_name: &str) -> Result<Vec<Search>> {
        let data: List<Search> = self
            .client
            .get(
                self.dsm_url.clone(),
                ApiParams::new(foto::search::API, "suggest", 6),
                &[("keyword", album_name)],
            )
            .await?;
        // TODO add support for "places" albums
        const SUPPORTED_ALBUM_TYPES: [&str; 3] = ["album", "person", "shared_with_me"];
        let albums = data
            .list
            .into_iter()
            .filter(|s| SUPPORTED_ALBUM_TYPES.contains(&s.r#type.as_str()))
            .collect();
        Ok(albums)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Space {
    Personal,
    Shared,
}

impl Space {
    pub fn browse_person_api(&self) -> &'static str {
        match self {
            Space::Personal => foto::browse::person::API,
            Space::Shared => foto_team::browse::person::API,
        }
    }

    pub fn browse_item_api(&self) -> &'static str {
        match self {
            Space::Personal => foto::browse::item::API,
            Space::Shared => foto_team::browse::item::API,
        }
    }
}
