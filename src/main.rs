mod http;
mod auth;
mod manifest;

#[tokio::main]
async fn main() {
    let http_service = crate::http::HttpService::new();
    let access_token = auth::get_token(&http_service).await.unwrap();
    let manifest = manifest::get_manifest(&http_service, &access_token).await.unwrap();

    std::fs::write("manifest.json", manifest).unwrap();
}