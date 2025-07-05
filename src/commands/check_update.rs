use crate::http::{HttpClient, HttpResponse};
use crate::io::Io;
use anyhow::{Result, anyhow, bail};
use io::Write;
use std::io;

pub async fn handle<C: HttpClient, I: Io>(
    installed_version: &str,
    client: &C,
    io: &mut I,
) -> Result<()> {
    let response = client
        .get("https://index.crates.io/sy/no/syno-photos-util")
        .await?;
    let status = response.status();
    if !status.is_success() {
        bail!(
            status
                .canonical_reason()
                .unwrap_or(status.as_str())
                .to_string()
        )
    }
    let remote_crate = response
        .text()
        .await?
        .lines()
        .map(serde_json::from_str::<dto::Crate>)
        .filter_map(|r| r.ok())
        .rfind(|c| !c.yanked)
        .ok_or(anyhow!("Unable to read creates.io response"))?;
    if remote_crate.vers != installed_version {
        writeln!(io.stdout(), "Version {} is available!", remote_crate.vers)?;
    } else {
        writeln!(io.stdout(), "Everything up to date")?;
    }
    Ok(())
}

mod dto {
    use serde::Deserialize;

    #[derive(Debug, PartialEq, Deserialize)]
    pub struct Crate {
        pub vers: String,
        pub yanked: bool,
    }
}
