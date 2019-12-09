#[derive(Debug)]
pub struct WickError {
    error: &'static str,
}

pub type WickResult<T> = Result<T, WickError>;

impl WickError {
    fn new(error: &'static str) -> Self {
        WickError {
            error
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
    fn from(error: http::Error) -> Self {
        Self::new("HTTP Error")
    }
}

impl From<http::header::ToStrError> for WickError {
    fn from(error: http::header::ToStrError) -> Self {
        Self::new("Header String error")
    }
}

impl From<hyper::Error> for WickError {
    fn from(error: hyper::Error) -> Self {
        Self::new("HTTP Request Error")
    }
}

impl From<std::str::Utf8Error> for WickError {
    fn from(error: std::str::Utf8Error) -> Self {
        Self::new("Invalid UTF-8 Encoding")
    }
}

impl From<std::num::ParseIntError> for WickError {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::new("Could not parse as int")
    }
}

impl From<serde_json::Error> for WickError {
    fn from(error: serde_json::Error) -> Self {
        Self::new("Could not deserialize JSON")
    }
}

impl From<std::io::Error> for WickError {
    fn from(error: std::io::Error) -> Self {
        Self::new("Reader error")
    }
}

pub fn make_err<T>(msg: &'static str) -> Result<T, WickError> {
    Err(WickError {
        error: msg,
    })
}