use reqwest::StatusCode;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct HttpError(pub StatusCode);

impl Display for HttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(reason) = self.0.canonical_reason() {
            write!(f, "{reason}")
        } else {
            write!(f, "{}", self.0.as_str())
        }
    }
}

impl Error for HttpError {}
