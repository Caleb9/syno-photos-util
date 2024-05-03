use super::creds::UserCredentials;
use crate::commands::api_client::{ApiClient, ApiParams};
use crate::http::Url;
use anyhow::Result;
use syno_api::auth::{self, dto::Login};

pub async fn login<C: ApiClient>(
    creds: &UserCredentials<'_>,
    remember_dev: bool,
    dsm_url: &Url,
    client: &C,
) -> Result<Login> {
    let mut form = Vec::from([
        ("account", creds.account.as_str()),
        ("passwd", creds.passwd.as_str()),
        ("format", "cookie"),
        (
            "enable_device_token",
            if remember_dev { "yes" } else { "no" },
        ),
    ]);
    if let Some(otp) = &creds.otp_code {
        form.push(("otp_code", otp));
    }
    if let Some(did) = &creds.device_id {
        form.push(("device_id", did.as_str()));
    }
    if remember_dev {
        form.push(("device_name", "syno_photos_util"));
    }
    let login_dto = client
        .post(
            dsm_url.clone(),
            ApiParams::new(auth::API, "login", 6),
            &form,
        )
        .await?;
    Ok(login_dto)
}
