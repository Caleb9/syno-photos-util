//! Session file ($HOME/.syno-photos-util) support

use crate::commands::login::creds::DeviceId;
use crate::fs::Fs;
use crate::http::Url;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

#[cfg(unix)]
use std::{fs::Permissions, os::unix::fs::PermissionsExt};

#[derive(Debug, Deserialize, Serialize)]
pub struct Conf {
    pub session: Option<Session>,
    pub device_ids: HashMap<String, DeviceId>,
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    #[serde_as(as = "DisplayFromStr")]
    pub url: Url,
    pub cookie: String,
}

impl Conf {
    pub fn new() -> Self {
        Conf {
            session: None,
            device_ids: HashMap::new(),
        }
    }

    pub fn is_logged_in(&self) -> bool {
        self.session.is_some()
    }

    pub fn get_session_url(&self) -> Option<Url> {
        self.session
            .as_ref()
            .map(|s| Url::parse(s.url.as_str()).expect("dsm_url should be valid"))
    }

    pub fn get_device_id(&self, url: &Url) -> Option<&DeviceId> {
        self.device_ids.get(url.as_str())
    }

    pub fn set_device_id<D: Into<Option<DeviceId>>>(&mut self, device_id: D) {
        if !self.is_logged_in() {
            return;
        }
        let url = self.session.as_ref().unwrap().url.to_string();
        if let Some(did) = device_id.into() {
            self.device_ids.insert(url, did);
        } else {
            self.device_ids.remove(&url);
        }
    }

    const CONF_FILE: &'static str = ".syno-photos-util";
    #[cfg(unix)]
    const OWNER_RW: u32 = 0o600;

    pub fn try_save<F: Fs>(&self, fs: &F) -> Result<()> {
        let tmp_path = &Self::tmp_path();
        let data = serde_json::to_string(self)?;
        fs.write(tmp_path, data.as_bytes())?;
        let conf_path = &Self::conf_path(fs)?;
        fs.copy(tmp_path, conf_path)?;
        fs.remove_file(tmp_path)?;
        #[cfg(unix)]
        fs.set_permissions(conf_path, Permissions::from_mode(Self::OWNER_RW))?;
        Ok(())
    }

    pub fn try_load<F: Fs>(fs: &F) -> Option<Self> {
        Self::conf_path(fs)
            .map_or(None, |conf_path| {
                #[cfg(unix)]
                {
                    let _ = fs.metadata(&conf_path).map(|m| {
                        /* & 0o777 removes file type part from the mode. Otherwise, the mode is
                         * 6 octal digits instead of 3 */
                        let mode = m.permissions().mode() & 0o777;
                        if mode != Self::OWNER_RW {
                            log::warn!(
                                "{} mode is {mode:o}, should be {:o}",
                                conf_path.display(),
                                Self::OWNER_RW
                            );
                        }
                    });
                }
                fs.read_to_string(conf_path).ok()
            })
            .and_then(|data| serde_json::from_str(data.as_str()).ok())
    }

    fn conf_path<F: Fs>(fs: &F) -> Result<PathBuf> {
        fs.home_dir()
            .map(|home| home.join(Self::CONF_FILE))
            .ok_or(anyhow!("unable to find home dir"))
    }

    fn tmp_path() -> PathBuf {
        // TODO consider if this needs to be isolated
        env::temp_dir().join(Self::CONF_FILE)
    }
}
