#[derive(Debug)]
pub struct WickError {
    error: String,
}

pub type WickResult<T> = Result<T, WickError>;

impl WickError {
    fn new(error: &str) -> Self {
        WickError {
            error: error.to_owned(),
        }
    }

    fn new_str(error: String) -> Self {
        WickError {
            error,
        }
    }
}

impl std::fmt::Display for WickError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Manifest Error: {}", self.error)
    }
}
impl std::error::Error for WickError {}


// A whole buncha error converters.
// If I need more detail on these while debugging, I can add them to the struct in the converter.
impl From<http::Error> for WickError {
    fn from(_error: http::Error) -> Self {
        Self::new("HTTP Error")
    }
}

impl From<http::header::ToStrError> for WickError {
    fn from(_error: http::header::ToStrError) -> Self {
        Self::new("Header String error")
    }
}

impl From<hyper::Error> for WickError {
    fn from(_error: hyper::Error) -> Self {
        Self::new("HTTP Request Error")
    }
}

impl From<std::str::Utf8Error> for WickError {
    fn from(_error: std::str::Utf8Error) -> Self {
        Self::new("Invalid UTF-8 Encoding")
    }
}

impl From<std::num::ParseIntError> for WickError {
    fn from(_error: std::num::ParseIntError) -> Self {
        Self::new("Could not parse as int")
    }
}

impl From<serde_json::Error> for WickError {
    fn from(error: serde_json::Error) -> Self {
        Self::new_str(format!("Could not deserialize JSON: {}", error))
    }
}

impl From<std::io::Error> for WickError {
    fn from(_error: std::io::Error) -> Self {
        Self::new("Reader error")
    }
}

impl<T> From<futures::channel::mpsc::TrySendError<T>> for WickError {
    fn from(_error: futures::channel::mpsc::TrySendError<T>) -> Self {
        Self::new("Futures Channel error")
    }
}

impl From<john_wick_parse::assets::ParserError> for WickError {
    fn from(error: john_wick_parse::assets::ParserError) -> Self {
        Self::new_str(format!("Could not parse: {}", error))
    }
}

impl From<block_modes::InvalidKeyIvLength> for WickError {
    fn from(_error: block_modes::InvalidKeyIvLength) -> Self {
        Self::new("Invalid key")
    }
}

impl From<block_modes::BlockModeError> for WickError {
    fn from(_error: block_modes::BlockModeError) -> Self {
        Self::new("Decrypt error")
    }
}

impl From<hex::FromHexError> for WickError {
    fn from(_error: hex::FromHexError) -> Self {
        Self::new("Hex key error")
    }
}

pub fn make_err<T>(msg: &str) -> Result<T, WickError> {
    Err(WickError::new(msg))
}