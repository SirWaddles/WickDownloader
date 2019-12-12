mod http;
mod auth;
mod manifest;
mod err;
mod chunks;
mod spool;

use std::sync::Arc;
use std::io::{Seek, SeekFrom, Cursor};
use tokio::io::{AsyncReadExt};
const PAK_SIZE: u32 = 8 + 16 + 20 + 1 + 16 + (32 * 5);

#[tokio::main]
async fn main() {
    let http_service = Arc::new(crate::http::HttpService::new());
    let access_token = auth::get_token(&http_service).await.unwrap();
    let app_manifest = manifest::get_manifest(&http_service, &access_token).await.unwrap();
    let mut chunk_manifest = manifest::get_chunk_manifest(&http_service, &app_manifest).await.unwrap();

    let files = chunk_manifest.get_files_mut(|v| &v[v.len() - 4..] == ".pak" && &v[..8] == "Fortnite");
    let file = &files[0];

    let mut reader = chunks::make_reader(http_service.clone(), &chunk_manifest, &app_manifest, file).unwrap();
    reader.seek(SeekFrom::End(-(PAK_SIZE as i64))).unwrap();
    let mut header = [0u8; PAK_SIZE as usize];
    reader.read_exact(&mut header).await.unwrap();
    let mut header_cursor = Cursor::new(header);
}
