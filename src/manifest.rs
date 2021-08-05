use crate::http::HttpService;
use crate::auth::AccessToken;
use crate::err::{WickError, WickResult, make_err};
use std::collections::HashMap;
use serde::{Deserialize};
use hyper::{Request, Body};
use john_wick_parse::manifest::Manifest;
use std::io::Write;
use std::fs::File;

const MANIFEST_URL: &'static str = "https://launcher-public-service-prod06.ol.epicgames.com/launcher/api/public/assets/Windows/4fe75bbc5a674f4f9b356b5c90567da5/Fortnite?label=Live";

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
    pub fn get_distributions(&self) -> WickResult<Vec<String>> {
        match self.items.get("MANIFEST") {
            Some(item) => {
                let mut distributions = item.additional_distributions.clone();
                distributions.push(item.distribution.clone());
                Ok(distributions)
                
            },
            None => make_err("Could not get manifest"),
        }
    }
}

pub fn create_app_manifest(manifest: &str) -> WickResult<AppManifest> {
    match serde_json::from_str(manifest) {
        Ok(res) => Ok(res),
        Err(_) => Err(WickError::new_str(format!("App Manifest Create Error: {}", &manifest[..std::cmp::min(200, manifest.len())]), 14)),
    }
}

pub async fn get_manifest(http: &HttpService, token: &AccessToken) -> WickResult<AppManifest> {
    let req = Request::builder()
        .method("GET")
        .uri(MANIFEST_URL)
        .header("Authorization", "bearer ".to_owned() + token.get_access_token())
        .body(Body::empty())?;

    let manifest = http.post_url(req).await?;

    File::create("manifest.test").unwrap().write_all(&manifest[..])?;
    let str_manifest = std::str::from_utf8(&manifest)?.to_owned();

    match serde_json::from_str(&str_manifest) {
        Ok(res) => Ok(res),
        Err(_) => Err(WickError::new_str(format!("App Manifest Read Error: {}", &str_manifest[..std::cmp::min(200, manifest.len())]), 14))
    }
}

pub async fn get_chunk_manifest(http: &HttpService, manifest: &AppManifest) -> WickResult<Manifest> {
    let manifest_item = match manifest.items.get("MANIFEST") {
        Some(item) => item,
        None => make_err("Could not retrieve manifest")?,
    };

    let manifest_url = manifest_item.distribution.clone() + &manifest_item.path + "?" + &manifest_item.signature;
    let chunk_manifest = http.get_url(&manifest_url).await?;

    let manifest_parse = Manifest::from_buffer(&chunk_manifest)?;
    Ok(manifest_parse)
}
