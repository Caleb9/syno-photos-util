//! Isolates [reqwest::Client] for testing

use anyhow::Result;
use reqwest::{Client as ReqwestClient, IntoUrl, Response as ReqwestResponse, StatusCode};
pub use reqwest::{Url, cookie::CookieStore, header::HeaderValue};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::future::Future;
use std::sync::Arc;

pub trait HttpClient {
    type Response: HttpResponse + Debug;
    fn get<U: IntoUrl>(&self, url: U) -> impl Future<Output = Result<Self::Response>>;
    fn post<U: IntoUrl, F: Serialize>(
        &self,
        url: U,
        form: &F,
    ) -> impl Future<Output = Result<Self::Response>>;
}

/// Isolates [reqwest::Response] for testing
#[cfg_attr(test, mockall::automock)]
pub trait HttpResponse {
    fn status(&self) -> StatusCode;
    fn text(self) -> impl Future<Output = Result<String>>;
    fn json<T: DeserializeOwned + 'static>(self) -> impl Future<Output = Result<T>>;
}

impl HttpClient for ReqwestClient {
    type Response = ReqwestResponse;

    async fn get<U: IntoUrl>(&self, url: U) -> Result<Self::Response> {
        Ok(ReqwestClient::get(self, url).send().await?)
    }

    async fn post<U: IntoUrl, F: Serialize>(&self, url: U, form: &F) -> Result<Self::Response> {
        Ok(ReqwestClient::post(self, url).form(form).send().await?)
    }
}

impl HttpResponse for ReqwestResponse {
    fn status(&self) -> StatusCode {
        ReqwestResponse::status(self)
    }

    async fn text(self) -> Result<String> {
        Ok(ReqwestResponse::text(self).await?)
    }

    async fn json<T: DeserializeOwned>(self) -> Result<T> {
        Ok(ReqwestResponse::json(self).await?)
    }
}

pub struct CookieClient<C: HttpClient, S: CookieStore> {
    pub client: C,
    pub cookie_store: Arc<S>,
}
