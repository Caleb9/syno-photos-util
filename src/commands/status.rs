use crate::conf::Conf;
use crate::io::Io;
use anyhow::Result;
use std::io::Write;

pub fn handle<I: Io>(conf: &Conf, io: &mut I) -> Result<()> {
    match conf.is_logged_in() {
        true => {
            writeln!(
                io.stdout(),
                "signed in to {}",
                conf.session.as_ref().unwrap().url
            )?;
        }
        false => {
            writeln!(
                io.stdout(),
                "signed out, use the 'login' command to sign-in to DSM"
            )?;
        }
    }
    Ok(())
}
