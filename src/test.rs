mod http;
mod auth;
mod manifest;
mod err;
mod chunks;
mod spool;

use std::sync::Arc;
use std::io::{Seek, SeekFrom, Cursor};
use std::fs;
use tokio::io::{AsyncReadExt};
use john_wick_parse::assets::Newable;
use john_wick_parse::archives::{FPakInfo, FPakIndex};
use block_modes::{BlockMode, Ecb, block_padding::ZeroPadding};
use aes_soft::Aes256;

const PAK_SIZE: u32 = 8 + 16 + 20 + 1 + 16 + (32 * 5);

#[tokio::main]
async fn main() {
    let http_service = Arc::new(crate::http::HttpService::new());
    /*let access_token = auth::get_token(&http_service).await.expect("Token failed");
    let app_manifest = manifest::get_manifest(&http_service, &access_token).await.expect("Manifest failed");
    let mut chunk_manifest = manifest::get_chunk_manifest(&http_service, &app_manifest).await.expect("Chunk manifest failed");*/
    let app_manifest_str = fs::read_to_string("app_manifest.json").unwrap();
    let chunk_manifest_str = fs::read_to_string("chunk_manifest.json").unwrap();
    let app_manifest = manifest::read_app_manifest(&app_manifest_str).expect("Deserializing App Manifest");
    let mut chunk_manifest = manifest::read_chunk_manifest(&chunk_manifest_str).expect("Deserializing Chunk Manifest");

    let files = chunk_manifest.get_files_mut(|v| &v[v.len() - 4..] == ".pak" && &v[..8] == "Fortnite");
    let file = &files[0];

    // Make reader
    let mut reader = chunks::make_reader(http_service.clone(), &chunk_manifest, &app_manifest, file).expect("Reader failed");

    // Seek to and read header info
    reader.seek(SeekFrom::End(-(PAK_SIZE as i64))).expect("Seek failed");
    let mut header = [0u8; PAK_SIZE as usize];
    reader.read_exact(&mut header).await.expect("Reading header failed");
    let mut header_cursor = Cursor::new(&header[..]);
    let pak_header = FPakInfo::new(&mut header_cursor).expect("Pak Header");

    // Retrieve and decrypt index
    let (index_start, index_length) = pak_header.get_index_sizes();
    reader.seek(SeekFrom::Start(index_start)).unwrap();
    let mut buffer = vec![0u8; index_length as usize];
    reader.read_exact(&mut buffer).await.expect("Reading index failed");

    let key_hex = fs::read_to_string("key.txt").unwrap();
    let key = hex::decode(&key_hex).unwrap();
    let decrypt = Ecb::<Aes256, ZeroPadding>::new_var(&key, Default::default()).unwrap();
    decrypt.decrypt(&mut buffer).expect("Failed decryption");

    // Interpret index
    let mut index_cursor = Cursor::new(buffer.as_slice());
    let pak_index = FPakIndex::new(&mut index_cursor).unwrap();
    fs::write("filelist.txt", pak_index.get_entries().iter().map(|v| v.get_filename()).fold(String::new(), |acc, v| acc + v + "\n")).unwrap();
}
