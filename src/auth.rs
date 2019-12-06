use crate::http::HttpService;
use hyper::{Request, Body};
use serde::Deserialize;

const CREDENTIAL_URL: &'static str = "https://account-public-service-prod03.ol.epicgames.com/account/api/oauth/token";
const CLIENT_POST_DATA: &'static str = "grant_type=client_credentials&token_token=eg1";
const EGS_AUTH: &'static str = "basic MzRhMDJjZjhmNDQxNGUyOWIxNTkyMTg3NmRhMzZmOWE6ZGFhZmJjY2M3Mzc3NDUwMzlkZmZlNTNkOTRmYzc2Y2Y=";

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct AccessToken {
    access_token: String,
    expires_in: i32,
    expires_at: String,
    token_type: String,
    client_id: String,
    internal_client: bool,
    client_service: String,
}

impl AccessToken {
    pub fn get_access_token(&self) -> &str {
        &self.access_token
    }
}

pub async fn get_token(http: &HttpService) -> Result<AccessToken, Box<dyn std::error::Error>> {
    let req = Request::builder()
        .method("POST")
        .uri(CREDENTIAL_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Authorization", EGS_AUTH)
        .body(Body::from(CLIENT_POST_DATA))?;

    let json_result = http.post_url_string(req).await?;
    Ok(serde_json::from_str(&json_result)?)
}