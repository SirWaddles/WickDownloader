mod http;
mod manifest;
mod err;
mod auth;
mod chunks;
mod spool;
mod reader;

use std::sync::{Arc, Mutex};
pub use err::WickResult;
use tokio::io::{AsyncReadExt};
use john_wick_parse::dispatch::UtocManager;

pub struct ServiceState {
    http: Arc<crate::http::HttpService>,
    app_manifest: manifest::AppManifest,
    chunk_manifest: manifest::ChunkManifest,
    files: Vec<manifest::ChunkManifestFile>,
}

pub struct UtocService {
    utoc: UtocManager,
    reader: Mutex<chunks::ChunkReader>,
}

impl ServiceState {
    pub async fn new() -> WickResult<Self> {
        let http_service = Arc::new(crate::http::HttpService::new());
        let access_token = auth::get_token(&http_service).await?;
        let app_manifest = manifest::get_manifest(&http_service, &access_token).await?;
        let mut chunk_manifest = manifest::get_chunk_manifest(&http_service, &app_manifest).await?;

        // Filter out just the pak files
        let files = chunk_manifest.get_files_mut(|v| {
            let ext = &v[v.len() - 5..];
            (ext == ".utoc" || ext == ".ucas") && &v[..8] == "Fortnite"
        });

        Ok(Self {
            http: http_service,
            app_manifest,
            chunk_manifest,
            files,
        })
    }

    pub fn from_manifests(app_manifest: &str, chunk_manifest: &str) -> WickResult<Self> {
        let http_service = Arc::new(crate::http::HttpService::new());
        let app_manifest = manifest::create_app_manifest(app_manifest)?;
        let mut chunk_manifest = manifest::create_chunk_manifest(chunk_manifest)?;
        let files = chunk_manifest.get_files_mut(|v| {
            let ext = &v[v.len() - 5..];
            (ext == ".utoc" || ext == ".ucas") && &v[..8] == "Fortnite"
        });

        Ok(Self {
            http: http_service,
            app_manifest,
            chunk_manifest,
            files,
        })
    }

    pub fn get_paks(&self) -> Vec<String> {
        self.files.iter().map(|v| v.filename.to_owned()).collect()
    }

    pub async fn download_file(&self, file: String, target: String) -> WickResult<()> {
        let file = match self.files.iter().find(|v| v.filename == file) {
            Some(f) => f,
            None => return err::make_err("File does not exist"),
        };

        chunks::download_file(self.http.clone(), &self.chunk_manifest, &self.app_manifest, &file, &target).await?;

        Ok(())
    }

    pub async fn get_utoc(&self, file: &str) -> WickResult<UtocService> {
        if &file[file.len() - 5..] != ".utoc" {
            return err::make_err("Invalid Index File");
        }

        let file_entry = match self.files.iter().find(|v| v.filename == file) {
            Some(f) => f,
            None => return err::make_err("File does not exist"),
        };

        let mut reader = chunks::make_reader(self.http.clone(), &self.chunk_manifest, &self.app_manifest, &file_entry)?;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await?;
        let utoc = UtocManager::new(&buf, None)?;

        let mut ucas_file = file.to_owned();
        ucas_file.replace_range(file.len() - 5.., ".ucas");

        let file_entry = match self.files.iter().find(|v| v.filename == ucas_file) {
            Some(f) => f,
            None => return err::make_err("File does not exist"),
        };

        let reader = chunks::make_reader(self.http.clone(), &self.chunk_manifest, &self.app_manifest, &file_entry)?;

        Ok(UtocService {
            utoc,
            reader: Mutex::new(reader),
        })
    }
}

impl UtocService {
    pub async fn get_file(&self, file: &str) -> WickResult<Vec<u8>> {
        let offset = match self.utoc.get_file(file) {
            Some(o) => o,
            None => return err::make_err("File not found"),
        };

        let mut ucas_reader = self.reader.lock().unwrap().reset();
        let data = reader::get_chunk(&mut ucas_reader, self.utoc.get_reader_data(), &offset).await?;

        Ok(data)
    }

    pub fn get_file_list(&self) -> &Vec<String> {
        self.utoc.get_file_list()
    }
}

