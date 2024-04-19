use super::creds::UserCredentials;
use crate::commands::api_client::{ApiClient, ApiQueryParams};
use crate::http::Url;
use anyhow::Result;
use syno_api::auth::{self, dto::Login};

pub async fn login<C: ApiClient>(
    creds: &UserCredentials<'_>,
    remember_dev: bool,
    dsm_url: &Url,
    client: &C,
) -> Result<Login> {
    let mut query_params = Vec::from([
        ("account", creds.account.as_str()),
        ("passwd", creds.passwd.as_str()),
        ("format", "sid"),
        (
            "enable_device_token",
            if remember_dev { "yes" } else { "no" },
        ),
    ]);
    if let Some(otp) = &creds.otp_code {
        query_params.push(("otp_code", otp));
    }
    if let Some(did) = &creds.device_id {
        query_params.push(("device_id", did.as_str()));
    }
    if remember_dev {
        query_params.push(("device_name", "syno_photos_util"));
    }
    let login_dto = client
        .get(
            dsm_url.clone(),
            ApiQueryParams::new(auth::API, "login", 6, None),
            &query_params,
        )
        .await?;
    Ok(login_dto)
}
