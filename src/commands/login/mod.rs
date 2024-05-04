use super::DsmError;
use crate::{
    conf::{Conf, Session},
    fs::Fs,
    http::{CookieStore, HttpClient, Url},
    io::{read_input, Io},
    CookieClient,
};
use anyhow::{anyhow, bail, Result};
use creds::{DeviceId, InputReader, UserCredentials};
use std::str::FromStr;
use syno_api::auth::dto::Login;
use syno_api::auth::error::AuthError;

mod api_client;
pub mod creds;

pub type LoginArgs = (Option<String>, Option<String>, bool);

pub async fn handle<C: HttpClient, S: CookieStore, I: Io, F: Fs>(
    dsm_url: Option<Url>,
    (user, password, remember_dev): LoginArgs,
    conf: &mut Conf,
    client: &CookieClient<C, S>,
    io: &mut I,
    fs: &F,
) -> Result<()> {
    let dsm_url = unwrap_or_read_dsm_url(dsm_url, conf, io)?;
    let login_dto = login_flow(
        &dsm_url,
        (user, password, remember_dev),
        conf,
        io,
        &client.client,
    )
    .await?;
    let session_cookie = client
        .cookie_store
        .cookies(&dsm_url)
        .expect("login response should contain session cookie");
    conf.session = Some(Session {
        url: dsm_url,
        cookie: String::from_str(session_cookie.to_str()?)?,
    });
    if remember_dev {
        conf.set_device_id(DeviceId::new(login_dto.did)?);
    }
    conf.try_save(fs)
}

/// Unwrap `url` or try to read it from `conf`, and if that also fails, from stdin.
fn unwrap_or_read_dsm_url<I: Io>(url: Option<Url>, conf: &Conf, io: &mut I) -> Result<Url> {
    let mut dsm_url = url.or_else(|| conf.get_session_url()).map_or_else(
        || Url::parse(read_input("DSM address", io)?.as_str()).map_err(|e| anyhow!(e)),
        Ok,
    )?;
    /* Set port to default value. This can potentially be problematic when reverse proxy is used
     * (omitting port means 80/443 instead of 5000/5001). */
    let is_behind_reverse_proxy = dsm_url.path() != "/";
    if dsm_url.port().is_none() && !is_behind_reverse_proxy {
        let port = match dsm_url.scheme() {
            "http" => 5000,
            "https" => 5001,
            _ => bail!("invalid URL scheme {}", dsm_url.scheme()),
        };
        dsm_url
            .set_port(Some(port))
            .expect("DSM URL address should be valid");
        log::info!("using DSM address: {dsm_url}")
    }
    Ok(dsm_url)
}

/// Fetch sid and did from SYNO.Auth API using account and password, then (if MFA is
/// enabled) read OTP code and try again.
async fn login_flow<I: Io, C: HttpClient>(
    dsm_url: &Url,
    (mut user, mut password, remember_dev): LoginArgs,
    conf: &Conf,
    io: &mut I,
    client: &C,
) -> Result<Login> {
    let mut user_credentials = UserCredentials::new(
        user.unwrap_or_read_stdin(io, "DSM user")?,
        password.unwrap_or_read_password(io)?,
        conf.get_device_id(dsm_url),
    );
    let login_result = api_client::login(&user_credentials, remember_dev, dsm_url, client).await;
    let login_dto = match login_result {
        Err(error) => {
            let auth_err = error.downcast::<DsmError>()?;
            match auth_err {
                DsmError::Auth(AuthError::MfaCodeRequired)
                | DsmError::Auth(AuthError::EnforceAuthWithMfa) => {
                    user_credentials.read_otp(io)?;
                    api_client::login(&user_credentials, remember_dev, dsm_url, client).await?
                }
                other => bail!(other),
            }
        }
        Ok(dto) => dto,
    };
    Ok(login_dto)
}
