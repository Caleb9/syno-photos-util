use super::{Album, DsmError};
use crate::commands::error::HttpError;
use crate::commands::login::creds::SessionId;
use crate::conf::Session;
use crate::http::{HttpClient, HttpResponse};
use anyhow::{bail, Result};
use reqwest::IntoUrl;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::future::Future;
use syno_api::dto::{ApiResponse, List};
use syno_api::foto;
use syno_api::foto::browse::album::dto::Album as AlbumDto;
use syno_api::foto::browse::item::dto::Item;
use syno_api::foto::browse::person::dto::Person;
use syno_api::foto::search::dto::Search;
use syno_api::foto::setting::user::dto::UserSettings;

/// Trait to add `get` and `post` methods to `HttpClient` which take parameters required by Synology
/// Photos API
pub trait ApiClient {
    fn get<'a, U, S, R>(
        &self,
        url: U,
        common_params: ApiQueryParams<'a, S>,
        other_params: &[(&str, &str)],
    ) -> impl Future<Output = Result<R>>
    where
        U: IntoUrl,
        S: Into<Option<&'a SessionId>>,
        R: DeserializeOwned + 'static;

    fn post<'a, U, S, R>(
        &self,
        url: U,
        common_params: ApiQueryParams<'a, S>,
        other_params: &[(&str, &str)],
    ) -> impl Future<Output = Result<R>>
    where
        U: IntoUrl,
        S: Into<Option<&'a SessionId>>,
        R: DeserializeOwned + 'static;
}

impl<C: HttpClient> ApiClient for C {
    async fn get<'a, U, S, R>(
        &self,
        url: U,
        ApiQueryParams {
            api,
            method,
            version,
            sid,
        }: ApiQueryParams<'a, S>,
        params: &[(&str, &str)],
    ) -> Result<R>
    where
        U: IntoUrl,
        S: Into<Option<&'a SessionId>>,
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
        if let Some(sid) = sid.into() {
            query.push_str(format!("&_sid={sid}").as_str());
        }
        for (key, value) in params {
            query.push_str(format!("&{key}={value}").as_str());
        }
        url.set_query(query.as_str().into());
        let response = C::get(self, url).await?;
        let data_dto = try_deserialize_response_content(response).await?;
        Ok(data_dto)
    }

    async fn post<'a, U, S, R>(
        &self,
        url: U,
        ApiQueryParams {
            api,
            method,
            version,
            sid,
        }: ApiQueryParams<'a, S>,
        params: &[(&str, &str)],
    ) -> Result<R>
    where
        U: IntoUrl,
        S: Into<Option<&'a SessionId>>,
        R: DeserializeOwned + 'static,
    {
        let mut url = url.into_url()?;
        let path = url.path().trim_end_matches('/');
        url.set_path(format!("{path}/webapi/entry.cgi").as_str());
        let version = version.to_string();
        let mut form = vec![
            ("api", api),
            ("method", method),
            ("version", version.as_str()),
        ];
        if let Some(sid) = sid.into() {
            form.push(("_sid", sid.as_str()));
        }
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
pub struct ApiQueryParams<'a, S> {
    api: &'a str,
    method: &'a str,
    version: u8,
    sid: S,
}

impl<'a, S> ApiQueryParams<'a, S>
where
    S: Into<Option<&'a SessionId>>,
{
    pub fn new(api: &'a str, method: &'a str, version: u8, sid: S) -> Self {
        Self {
            api,
            method,
            version,
            sid,
        }
    }
}

/// Provides methods to query Synology Photos API when logged-in. Used by multiple commands.
pub struct SessionClient<'a, C> {
    pub(crate) session: &'a Session,
    pub(crate) client: &'a C,
}

impl<'a, C: ApiClient> SessionClient<'a, C> {
    pub fn new(session: &'a Session, client: &'a C) -> Self {
        SessionClient { session, client }
    }

    pub async fn get_user_settings(&self) -> Result<UserSettings> {
        let Session { url, id } = self.session;
        self.client
            .get(
                url.clone(),
                ApiQueryParams::new(foto::setting::user::API, "get", 1, id),
                &[("id", id.to_string().as_str())],
            )
            .await
    }

    pub async fn count_owned_albums(&self) -> Result<u32> {
        #[derive(Debug, Deserialize)]
        struct CountContainer {
            count: u32,
        }

        let Session { url, id } = self.session;
        let data: CountContainer = self
            .client
            .get(
                url.clone(),
                ApiQueryParams::new(foto::browse::album::API, "count", 2, id),
                &[],
            )
            .await?;
        Ok(data.count)
    }

    pub async fn list_owned_albums(&self, limit: u32) -> Result<Vec<AlbumDto>> {
        let Session { url, id } = self.session;
        let data: List<AlbumDto> = self
            .client
            .get(
                url.clone(),
                ApiQueryParams::new(foto::browse::album::API, "list", 2, id),
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
        let Session { url, id } = self.session;
        let data: List<AlbumDto> = self
            .client
            .get(
                url.clone(),
                ApiQueryParams::new(foto::sharing::misc::API, "list_shared_with_me_album", 2, id),
                &[
                    ("offset", offset.to_string().as_str()),
                    ("limit", limit.to_string().as_str()),
                ],
            )
            .await?;
        Ok(data.list)
    }

    pub async fn count_people(&self) -> Result<u32> {
        #[derive(Debug, Deserialize)]
        struct CountContainer {
            count: u32,
        }

        let Session { url, id } = self.session;
        let data: CountContainer = self
            .client
            .get(
                url.clone(),
                ApiQueryParams::new(foto::browse::person::API, "count", 2, id),
                &[("show_more", true.to_string().as_str())],
            )
            .await?;
        Ok(data.count)
    }

    pub async fn list_people(&self, limit: u32) -> Result<Vec<Person>> {
        let Session { url, id } = self.session;
        let data: List<Person> = self
            .client
            .get(
                url.clone(),
                ApiQueryParams::new(foto::browse::person::API, "list", 1, id),
                &[("offset", "0"), ("limit", limit.to_string().as_str())],
            )
            .await?;
        Ok(data.list)
    }

    pub async fn list_items(&self, album: &Album, limit: u32) -> Result<Vec<Item>> {
        let Session {
            url,
            id: session_id,
        } = self.session;
        let (key, value) = album.id_param();
        let items: List<Item> = self
            .client
            .get(
                url.clone(),
                ApiQueryParams::new(foto::browse::item::API, "list", 1, session_id),
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
        let Session { url, id } = self.session;
        let data: List<Search> = self
            .client
            .get(
                url.clone(),
                ApiQueryParams::new(foto::search::API, "suggest", 6, id),
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

#[derive(Debug)]
pub enum Space {
    Personal,
    Shared,
}
