#[derive(Debug)]
pub struct WickError {
    error: String,
    code: u32,
}

pub type WickResult<T> = Result<T, WickError>;

impl WickError {
    fn new(error: &str, code: u32) -> Self {
        WickError {
            error: error.to_owned(),
            code,
        }
    }

    pub(crate) fn new_str(error: String, code: u32) -> Self {
        WickError {
            error,
            code,
        }
    }

    pub fn get_code(&self) -> u32 {
        self.code
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
        Self::new("HTTP Error", 1)
    }
}

impl From<http::header::ToStrError> for WickError {
    fn from(_error: http::header::ToStrError) -> Self {
        Self::new("Header String error", 2)
    }
}

impl From<hyper::Error> for WickError {
    fn from(_error: hyper::Error) -> Self {
        Self::new("HTTP Request Error", 3)
    }
}

impl From<std::str::Utf8Error> for WickError {
    fn from(_error: std::str::Utf8Error) -> Self {
        Self::new("Invalid UTF-8 Encoding", 4)
    }
}

impl From<std::num::ParseIntError> for WickError {
    fn from(_error: std::num::ParseIntError) -> Self {
        Self::new("Could not parse as int", 4)
    }
}

impl From<serde_json::Error> for WickError {
    fn from(error: serde_json::Error) -> Self {
        Self::new_str(format!("Could not deserialize JSON: {}", error), 5)
    }
}

impl From<std::io::Error> for WickError {
    fn from(_error: std::io::Error) -> Self {
        Self::new("Reader error", 6)
    }
}

impl<T> From<futures::channel::mpsc::TrySendError<T>> for WickError {
    fn from(_error: futures::channel::mpsc::TrySendError<T>) -> Self {
        Self::new("Futures Channel error", 7)
    }
}

impl From<john_wick_parse::assets::ParserError> for WickError {
    fn from(error: john_wick_parse::assets::ParserError) -> Self {
        Self::new_str(format!("Could not parse: {}", error), 8)
    }
}

pub fn make_err<T>(msg: &str) -> Result<T, WickError> {
    Err(WickError::new(msg, 12))
}

// 13 - Authentication Error
// 14 - App Manifest Read Error
// 15 - Chunk Manifest Read Error