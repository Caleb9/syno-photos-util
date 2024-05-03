//! CLI options

use crate::http::Url;
use anyhow::{bail, Result};
pub use clap::Parser;
use clap::Subcommand;
use std::time::Duration;

/// syno-photos-util
///
/// Helper for a number of tasks unavailable in Synology Photos web interface
///
/// Project website: <https://github.com/caleb9/syno-photos-util>
#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// HTTP request timeout in seconds
    ///
    /// Must be greater or equal to 5. When Synology Photos does not respond within the timeout, an
    /// error is displayed. Try to increase the value for slow connections
    #[arg(
        long = "timeout",
        default_value = "30",
        value_parser = try_parse_duration)]
    pub timeout_seconds: Duration,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Sign in to Synology DSM
    ///
    /// Required before other commands can be used. Writes session key to $HOME/.syno-photo-util
    /// file
    Login {
        /// HTTP(S) address of Synology DSM
        ///
        /// Port numbers can be omitted when using standard ones (5000 and 5001). When using an
        /// alias (e.g. https://my.nas/photo or similar) if default ports are not be detected
        /// correctly, specify them explicitly.
        dsm_url: Option<Url>,

        /// DSM user account name
        ///
        /// When not specified, will be read from standard input (first line)
        #[arg(short, long)]
        user: Option<String>,

        /// DSM user account password
        ///
        /// When not specified, will be read from standard input (second line)
        #[arg(short, long)]
        password: Option<String>,

        /// Omit OTP code verification on future runs
        ///
        /// Only applicable when 2-factor authentication is enabled on user account.
        /// Writes device id to $HOME/.syno-photo-util file
        #[arg(long)]
        remember: bool,
    },

    /// List file locations (folders) of photos in an album
    List {
        /// Album name; can also be a person name in "People" auto-album
        album_name: String,
    },

    /// Export (accessible) album photos to a folder in the user's Personal Space
    ///
    /// Requires that home service is enabled on DSM
    Export {
        /// Album name; can be a person name in "People" auto-album
        album_name: String,

        /// Folder name in user's Personal Space (must exist)
        folder_path: String,
    },

    /// Sign out of DSM
    ///
    /// Removes session key from $HOME/.syno-photo-util file
    Logout {
        /// Enforce OTP verification on future runs
        ///
        /// Only applicable when 2-factor authentication is enabled on user account.
        /// Removes ALL device ids from $HOME/.syno-photo-util file
        #[arg(long)]
        forget: bool,
    },

    /// Check DSM sign-in status
    Status,

    /// Check if new version is available
    CheckUpdate,
}

fn try_parse_duration(arg: &str) -> Result<Duration> {
    let seconds = arg.parse()?;
    if seconds < 5 {
        bail!("must not be less than 5".to_string())
    } else {
        Ok(Duration::from_secs(seconds))
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}
