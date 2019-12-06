use crate::http::HttpService;
use crate::auth::AccessToken;
use hyper::{Request, Body};

const MANIFEST_URL: &'static str = "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/public/assets/Windows/4fe75bbc5a674f4f9b356b5c90567da5/Fortnite?label=Live";

pub async fn get_manifest(http: &HttpService, token: &AccessToken) -> Result<String, Box<dyn std::error::Error>> {
    let req = Request::builder()
        .method("GET")
        .uri(MANIFEST_URL)
        .header("Authorization", "bearer ".to_owned() + token.get_access_token())
        .body(Body::empty())?;

    let manifest = http.post_url_string(req).await?;

    Ok(manifest)
}