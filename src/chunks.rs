use crate::err::{WickResult, make_err};
use crate::http::HttpService;
use crate::manifest::AppManifest;
use crate::spool::Spool;
use john_wick_parse::manifest::{Manifest, FFileManifest, FChunkPart};
use std::convert::AsRef;
use std::io::{Cursor, Read, Seek, SeekFrom, Result as IOResult};
use byteorder::{LittleEndian, ReadBytesExt};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, AsyncSeekExt, ReadBuf};
use futures::{join, FutureExt};
use futures::stream::StreamExt;
use futures::channel::mpsc;
use flate2::bufread::ZlibDecoder;

#[allow(dead_code)]
struct ChunkGuid {
    data: [u32; 4],
}

impl ChunkGuid {
    fn new<T>(cursor: &mut T) -> WickResult<Self> where T: ReadBytesExt {
        let mut data = [0u32; 4];
        for i in 0..4 {
            data[i] = cursor.read_u32::<LittleEndian>()?;
        }
        Ok(Self {
            data
        })
    }
}

#[allow(dead_code)]
struct ChunkSha {
    data: [u8; 20],
}

impl ChunkSha {
    fn new<T>(cursor: &mut T) -> WickResult<Self> where T: Read {
        let mut data = [0u8; 20];
        cursor.read_exact(&mut data)?;
        Ok(Self {
            data
        })
    }
}

#[allow(dead_code)]
struct ChunkHeader {
    version: u32,
    size: u32,
    data_size: u32,
    guid: ChunkGuid,
    hash: u64,
    stored: u8,
    sha: ChunkSha,
    hash_type: u8,
}

#[allow(dead_code)]
struct Chunk {
    header: ChunkHeader,
    data: Vec<u8>,
}

impl Chunk {
    fn new<T>(data: T, chunk: &ChunkDownload) -> WickResult<Self> where T: AsRef<[u8]> {
        let mut cursor = Cursor::new(data);
        let _magic = cursor.read_u32::<LittleEndian>()?;
        let header = ChunkHeader {
            version: cursor.read_u32::<LittleEndian>()?,
            size: cursor.read_u32::<LittleEndian>()?,
            data_size: cursor.read_u32::<LittleEndian>()?,
            guid: ChunkGuid::new(&mut cursor)?,
            hash: cursor.read_u64::<LittleEndian>()?,
            stored: cursor.read_u8()?,
            sha: ChunkSha::new(&mut cursor)?,
            hash_type: cursor.read_u8()?,
        };

        cursor.seek(SeekFrom::Start(header.size as u64))?;
        let mut data = vec![0u8; header.data_size as usize];
        cursor.read_exact(&mut data)?;

        if header.stored & 0x01 == 1 {
            let mut decompressed_data = Vec::new();
            let mut decompressor = ZlibDecoder::new(data.as_slice());
            decompressor.read_to_end(&mut decompressed_data)?;
            let chunk_size = chunk.length as usize;
            let chunk_offset = chunk.offset as usize;
            let mut final_data = vec![0u8; chunk_size];
            final_data.copy_from_slice(&decompressed_data[chunk_offset..(chunk_offset + chunk_size)]);
            data = final_data;
        }

        Ok(Self {
            header, data
        })
    }
}

#[derive(Debug, Clone)]
struct ChunkDownload {
    position: u64,
    length: u32,
    url: String,
    offset: u32,
    index: usize,
}

type ChunkData = (ChunkDownload, Chunk);

async fn write_chunks(mut receiver: mpsc::UnboundedReceiver<ChunkData>, filesize: u64, target: &str) -> WickResult<()> {
    let mut file = File::create(target).await?;
    file.set_len(filesize).await?;
    while let Some((data, chunk)) = receiver.next().await {
        file.seek(SeekFrom::Start(data.position)).await?;
        file.write_all(&chunk.data).await?;
    }
    Ok(())
}

async fn download_chunk(http: Arc<HttpService>, chunk: ChunkDownload) -> WickResult<ChunkData> {
    let data = http.get_url(&chunk.url).await?;
    let chunk_data = Chunk::new(data, &chunk)?;
    Ok((chunk, chunk_data))
}

async fn send_chunk(http: Arc<HttpService>, chunk: ChunkDownload, sender: mpsc::UnboundedSender<ChunkData>) -> WickResult<()> {
    let data = download_chunk(http.clone(), chunk).await?;
    sender.unbounded_send(data)?;
    Ok(())
}

const REQUEST_COUNT: usize = 20;

const DOWNLOAD_BASE: &'static str = "Builds/Fortnite/CloudDir/ChunksV4/";

fn make_chunk_url(manifest: &Manifest, chunk: &FChunkPart) -> WickResult<String> {
    let chunk_info = match manifest.get_chunks().iter().find(|v| v.guid == chunk.guid) {
        Some(c) => c,
        None => return make_err("Could not find chunk hash"),
    };
    let mut url = DOWNLOAD_BASE.to_owned();
    url += &format!("{:02}", chunk_info.group_number);
    url += "/";
    url += &format!("{:016X}", chunk_info.hash);
    url += "_";
    url += &(format!("{}", chunk.guid).to_uppercase());
    url += ".chunk";
    
    Ok(url)
}

pub async fn download_file(http: Arc<HttpService>, manifest: &Manifest, app: &AppManifest, file: &FFileManifest, target: &str) -> WickResult<()> {
    let distributions = app.get_distributions()?;
    let mut downloads = Vec::new();
    let mut position = 0;
    let mut i = 0;
    for chunk in &file.chunk_parts {
        let download = ChunkDownload {
            position,
            length: chunk.size,
            offset: chunk.offset,
            url: distributions[i % distributions.len()].to_owned() + &make_chunk_url(manifest, &chunk)?,
            index: i,
        };
        downloads.push(download);
        position += chunk.size as u64;
        i += 1;
    }

    let (file_sender, file_receiver) = mpsc::unbounded::<ChunkData>();
    let chunk_downloads = downloads.into_iter().map(|v| {
        send_chunk(http.clone(), v, file_sender.clone())
    }).collect();

    let (r1, r2) = join!(
        write_chunks(file_receiver, position, target),
        Spool::build(chunk_downloads, REQUEST_COUNT).then(|_x| async move {
            file_sender.close_channel();
            Ok(()) as WickResult<()>
        })
    );
    // wat
    r1?; r2?;

    Ok(())
}

pub fn make_reader(http: Arc<HttpService>, manifest: &Manifest, app: &AppManifest, file: &FFileManifest) -> WickResult<ChunkReader> {
    let distributions = app.get_distributions()?;
    let mut downloads = Vec::new();
    let mut position = 0;
    let mut i = 0;
    for chunk in &file.chunk_parts {
        let download = ChunkDownload {
            position,
            length: chunk.size,
            offset: chunk.offset,
            url: distributions[i % distributions.len()].to_owned() + &make_chunk_url(manifest, &chunk)?,
            index: i,
        };
        downloads.push(download);
        position += chunk.size as u64;
        i += 1;
    }

    Ok(ChunkReader::new(http.clone(), downloads))
}

use std::pin::Pin;
use std::sync::Arc;
use futures::Future;
use futures::task::{Context, Poll};
use tokio::io::AsyncRead;

enum ChunkReaderState {
    Resolving(Pin<Box<dyn Future<Output=WickResult<ChunkData>> + Send>>),
    Idle(ChunkData),
}

pub struct ChunkReader {
    http: Arc<HttpService>,
    chunks: Arc<Vec<ChunkDownload>>,
    position: u64,
    current_chunk: usize,
    state: ChunkReaderState,
    total_size: u64,
}

impl ChunkReader {
    fn new(http: Arc<HttpService>, chunks: Vec<ChunkDownload>) -> Self {
        if chunks.len() <= 0 {
            panic!("Cannot read an empty chunk list.");
        }
        let total_size = {
            let last_chunk = chunks.last().unwrap();
            last_chunk.position + last_chunk.length as u64
        };
        let first_resolve = download_chunk(http.clone(), chunks[0].clone());
        Self {
            http: http.clone(),
            chunks: Arc::new(chunks),
            position: 0,
            current_chunk: 0,
            state: ChunkReaderState::Resolving(Box::pin(first_resolve)),
            total_size,
        }
    }

    pub fn reset(&self) -> Self {
        let first_resolve = download_chunk(self.http.clone(), self.chunks[0].clone());
        Self {
            http: Arc::clone(&self.http),
            chunks: Arc::clone(&self.chunks),
            position: 0,
            current_chunk: 0,
            state: ChunkReaderState::Resolving(Box::pin(first_resolve)),
            total_size: self.total_size,
        }
    }
}

impl Seek for ChunkReader {
    fn seek(&mut self, seek: SeekFrom) -> IOResult<u64> {
        let fpos = match seek {
            SeekFrom::Start(pos) => pos,
            SeekFrom::End(pos) => (pos + (self.total_size as i64)) as u64,
            SeekFrom::Current(pos) => (pos + (self.position as i64)) as u64,
        };
        let chunk = self.chunks.iter().find(|&i| fpos >= i.position && (i.position + i.length as u64) > fpos).expect("No chunk found for position");
        if self.current_chunk != chunk.index {
            self.state = ChunkReaderState::Resolving(Box::pin(download_chunk(self.http.clone(), chunk.clone())));
        }

        self.position = fpos;
        self.current_chunk = chunk.index;
        Ok(fpos)
    }
}

impl AsyncRead for ChunkReader {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut ReadBuf) -> Poll<IOResult<()>> {
        let this = self.get_mut();
        loop {
            match &mut this.state {
                ChunkReaderState::Resolving(resolve) => {
                    match resolve.as_mut().poll(cx) {
                        Poll::Ready(data) => {
                            this.state = ChunkReaderState::Idle(data.unwrap());
                        },
                        Poll::Pending => return Poll::Pending,
                    }
                },
                ChunkReaderState::Idle((download, data)) => {
                    let pos_in_chunk = (this.position - download.position) as usize;
                    let to_write = std::cmp::min(buf.remaining(), (download.length as usize) - pos_in_chunk);
                    if to_write > 0 {
                        this.position += to_write as u64;
                        buf.put_slice(&data.data[pos_in_chunk..(pos_in_chunk + to_write)]);
                        return Poll::Ready(Ok(()));
                    } else {
                        this.current_chunk += 1;
                        if this.current_chunk >= this.chunks.len() {
                            return Poll::Ready(Ok(())); // Nothing left to read
                        }
                        let resolve = download_chunk(this.http.clone(), this.chunks[this.current_chunk].clone());
                        this.state = ChunkReaderState::Resolving(Box::pin(resolve));
                    }
                },
            }
        }
    }
}