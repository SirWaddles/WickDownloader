mod http;
mod auth;
mod manifest;
mod err;
mod chunks;

#[tokio::main]
async fn main() {
    let http_service = crate::http::HttpService::new();
    /*let access_token = auth::get_token(&http_service).await.unwrap();
    let app_manifest = manifest::get_manifest(&http_service, &access_token).await.unwrap();
    let chunk_manifest = manifest::get_chunk_manifest(&http_service, &app_manifest).await.unwrap();*/

    let chunk_manifest_str = std::fs::read_to_string("manifest.json").unwrap();
    let mut chunk_manifest = manifest::read_chunk_manifest(&chunk_manifest_str).unwrap();
    let files = chunk_manifest.get_files_mut(|v| &v[v.len() - 4..] == ".pak" && &v[..8] == "Fortnite");
    let file = &files[0];
    
    chunks::download_file(&http_service, &chunk_manifest, &file).await.unwrap();
}