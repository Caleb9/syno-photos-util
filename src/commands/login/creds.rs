use crate::io::{Io, read_input};
use anyhow::{Result, bail};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use yapp::PasswordReader;

#[derive(Debug)]
pub struct UserCredentials<'a> {
    pub account: String,
    pub passwd: String,
    pub otp_code: Option<String>,
    pub device_id: Option<&'a DeviceId>,
}

impl<'a> UserCredentials<'a> {
    pub fn new(account: String, passwd: String, device_id: Option<&'a DeviceId>) -> Self {
        UserCredentials {
            account,
            passwd,
            otp_code: None,
            device_id,
        }
    }

    pub fn read_otp<I: Io>(&mut self, io: &mut I) -> Result<()> {
        let otp = read_input("OTP code", io)?;
        if otp.trim().is_empty() {
            bail!("missing OTP code")
        }
        self.otp_code = Some(otp);
        Ok(())
    }
}

pub trait InputReader {
    fn unwrap_or_read_stdin<I: Io>(&mut self, io: &mut I, prompt: &str) -> Result<String>;
    fn unwrap_or_read_password<I: Io>(&mut self, io: &mut I) -> Result<String>;
}

impl InputReader for Option<String> {
    fn unwrap_or_read_stdin<I: Io>(&mut self, io: &mut I, prompt: &str) -> Result<String> {
        if self.as_ref().is_some_and(|v| !v.trim().is_empty()) {
            Ok(self.take().unwrap())
        } else {
            let input = read_input(prompt, io)?;
            if input.trim().is_empty() {
                bail!("missing {prompt}")
            }
            Ok(input)
        }
    }

    fn unwrap_or_read_password<I: Io>(&mut self, io: &mut I) -> Result<String> {
        let password = if self.as_ref().is_some_and(|v| !v.trim().is_empty()) {
            self.take().unwrap()
        } else {
            let passwd_reader = io.password_reader();
            passwd_reader.read_password_with_prompt("DSM password: ")?
        };
        Ok(password)
    }
}

#[derive(Debug, Display, Deserialize, Serialize)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn new(value: String) -> Result<Self> {
        if value.trim().is_empty() {
            bail!("value should not be empty")
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
