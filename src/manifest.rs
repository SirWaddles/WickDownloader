use crate::http::HttpService;
use crate::auth::AccessToken;
use crate::err::{WickError, WickResult, make_err};
use std::collections::HashMap;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Deserializer};
use hyper::{Request, Body};

const MANIFEST_URL: &'static str = "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/public/assets/Windows/4fe75bbc5a674f4f9b356b5c90567da5/Fortnite?label=Live";
const DOWNLOAD_BASE: &'static str = "Builds/Fortnite/CloudDir/ChunksV3/";

// Numbers are stored as psuedo-buffers, where every three characters represents a single byte in the buffer, as a plaintext integer.
fn parse_int_blob(val: &str) -> Cursor<Vec<u8>> {
    let data: Vec<u8> = val.as_bytes().chunks_exact(3).map(|v| -> u8 { std::str::from_utf8(v).unwrap().parse().unwrap() }).collect();
    Cursor::new(data)
}

pub fn parse_int_blob_u32(val: &str) -> u32 {
    if val.len() > 12 {
        panic!("int blob too long");
    }
    parse_int_blob(val).read_u32::<LittleEndian>().unwrap()
}

pub fn parse_int_blob_u64(val: &str) -> u64 {
    if val.len() > 24 {
        panic!("int blob too long");
    }
    parse_int_blob(val).read_u64::<LittleEndian>().unwrap()
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AppManifestItem {
    signature: String,
    distribution: String,
    path: String,
    additional_distributions: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppManifest {
    app_name: String,
    label_name: String,
    build_version: String,
    catalog_item_id: String,
    expires: String,
    asset_id: String,
    items: HashMap<String, AppManifestItem>,
}

impl AppManifest {
    pub fn get_distributions(&self) -> WickResult<&Vec<String>> {
        match self.items.get("MANIFEST") {
            Some(item) => Ok(&item.additional_distributions),
            None => make_err("Could not get manifest"),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ChunkManifest {
    app_name_string: String,
    build_version_string: String,
    file_manifest_list: Vec<ChunkManifestFile>,
    chunk_hash_list: HashMap<String, String>,
    data_group_list: HashMap<String, String>,
}

impl ChunkManifest {
    pub fn get_files_mut<F>(&mut self, pred: F) -> Vec<ChunkManifestFile>
        where F: Fn(&str) -> bool {
        self.file_manifest_list.split_off(0).into_iter()
            .filter(|v| pred(&v.filename))
            .collect()
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ChunkManifestFile {
    pub filename: String,
    file_hash: String,
    file_chunk_parts: Vec<ChunkManifestChunkPart>,
    #[serde(default = "Vec::new")]
    install_tags: Vec<String>,
}

impl ChunkManifestFile {
    pub fn get_chunks(&self) -> &Vec<ChunkManifestChunkPart> {
        &self.file_chunk_parts
    }
}

fn read_blob<'de, D>(deserializer: D) -> Result<u32, D::Error> where D: Deserializer<'de> {
    let raw: &str = Deserialize::deserialize(deserializer)?;
    Ok(parse_int_blob_u32(raw))
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ChunkManifestChunkPart {
    guid: String,
    #[serde(deserialize_with = "read_blob")]
    offset: u32,
    #[serde(deserialize_with = "read_blob")]
    size: u32,
}

impl ChunkManifestChunkPart {
    pub fn get_url(&self, manifest: &ChunkManifest) -> WickResult<String> {
        Ok(DOWNLOAD_BASE.to_owned() + match manifest.data_group_list.get(&self.guid) {
            Some(data) => &data[data.len() - 2..],
            None => return make_err("Could not find Data Group"),
        } + "/" + &match manifest.chunk_hash_list.get(&self.guid) {
            Some(data) => format!("{:016X}", parse_int_blob_u64(data)),
            None => return make_err("Could not find chunk hash"),
        } + "_" + &self.guid + ".chunk")
    }

    pub fn get_offset(&self) -> u32 {
        self.offset
    }

    pub fn get_size(&self) -> u32 {
        self.size
    }
}

pub fn create_app_manifest(manifest: &str) -> WickResult<AppManifest> {
    match serde_json::from_str(manifest) {
        Ok(res) => Ok(res),
        Err(_) => Err(WickError::new_str(format!("App Manifest Create Error: {}", &manifest[..std::cmp::min(200, manifest.len())]), 14)),
    }
}

pub fn create_chunk_manifest(manifest: &str) -> WickResult<ChunkManifest> {
    match serde_json::from_str(manifest) {
        Ok(res) => Ok(res),
        Err(_) => Err(WickError::new_str(format!("Chunk Manifest Create Error: {}", &manifest[..std::cmp::min(200, manifest.len())]), 14)),
    }
}

pub async fn get_manifest(http: &HttpService, token: &AccessToken) -> WickResult<AppManifest> {
    let req = Request::builder()
        .method("GET")
        .uri(MANIFEST_URL)
        .header("Authorization", "bearer ".to_owned() + token.get_access_token())
        .body(Body::empty())?;

    let manifest = http.post_url_string(req).await?;

    match serde_json::from_str(&manifest) {
        Ok(res) => Ok(res),
        Err(_) => Err(WickError::new_str(format!("App Manifest Read Error: {}", &manifest[..std::cmp::min(200, manifest.len())]), 14))
    }
}

pub async fn get_chunk_manifest(http: &HttpService, manifest: &AppManifest) -> WickResult<ChunkManifest> {
    let manifest_item = match manifest.items.get("MANIFEST") {
        Some(item) => item,
        None => make_err("Could not retrieve manifest")?,
    };

    let manifest_url = manifest_item.distribution.clone() + &manifest_item.path + "?" + &manifest_item.signature;
    let chunk_manifest = http.get_url_string(&manifest_url).await?;

    match serde_json::from_str(&chunk_manifest) {
        Ok(res) => Ok(res),
        Err(_) => Err(WickError::new_str(format!("Chunk Manifest Read Error: {}", &chunk_manifest[..std::cmp::min(200, chunk_manifest.len())]), 15))
    }
}
