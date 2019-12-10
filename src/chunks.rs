use crate::err::{WickResult, make_err};
use crate::http::HttpService;
use crate::manifest::{ChunkManifest, ChunkManifestFile, AppManifest};
use crate::spool::{Spool, PinnedBoxFuture};
use std::convert::AsRef;
use std::io::{Cursor, Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use futures::{join, FutureExt};
use futures::stream::StreamExt;
use futures::channel::mpsc;
use flate2::bufread::ZlibDecoder;

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

struct Chunk {
    header: ChunkHeader,
    data: Vec<u8>,
}

impl Chunk {
    fn new<T>(data: T, chunk: &ChunkDownload) -> WickResult<Self> where T: AsRef<[u8]> {
        let mut cursor = Cursor::new(data);
        let magic = cursor.read_u32::<LittleEndian>()?;
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

struct ChunkDownload {
    position: u64,
    length: u32,
    url: String,
    offset: u32,
}

type ChunkData = (ChunkDownload, Chunk);

async fn write_chunks(mut receiver: mpsc::UnboundedReceiver<ChunkData>, file: &ChunkManifestFile, filesize: u64) -> WickResult<()> {
    let filename = match &file.filename.split("/").last() {
        Some(path) => path.to_owned(),
        None => return make_err("Could not get filename"),
    };
    let mut file = File::create(filename).await?;
    file.set_len(filesize).await?;
    while let Some((data, chunk)) = receiver.next().await {
        file.seek(SeekFrom::Start(data.position)).await?;
        file.write_all(&chunk.data).await?;
    }
    Ok(())
}

async fn download_chunk(http: &HttpService, chunk: ChunkDownload) -> WickResult<ChunkData> {
    let data = http.get_url(&chunk.url).await?;
    let chunk_data = Chunk::new(data, &chunk)?;
    Ok((chunk, chunk_data))
}

async fn send_chunk(http: &HttpService, chunk: ChunkDownload, sender: mpsc::UnboundedSender<ChunkData>) -> WickResult<()> {
    let data = download_chunk(&http, chunk).await?;
    sender.unbounded_send(data)?;
    Ok(())
}

const REQUEST_COUNT: usize = 20;

pub async fn download_file(http: &HttpService, manifest: &ChunkManifest, app: &AppManifest, file: &ChunkManifestFile) -> WickResult<()> {
    let distributions = app.get_distributions()?;
    let mut downloads = Vec::new();
    let mut position = 0;
    let mut i = 0;
    for chunk in file.get_chunks() {
        let download = ChunkDownload {
            position,
            length: chunk.get_size(),
            offset: chunk.get_offset(),
            url: distributions[i % distributions.len()].to_owned() + &chunk.get_url(&manifest)?,
        };
        downloads.push(download);
        position += chunk.get_size() as u64;
        i += 1;
    }

    let (file_sender, file_receiver) = mpsc::unbounded::<ChunkData>();
    let chunk_downloads: Vec<PinnedBoxFuture<WickResult<()>>> = downloads.into_iter().map(|v| {
        send_chunk(&http, v, file_sender.clone()).boxed()
    }).collect();

    let (r1, r2) = join!(
        write_chunks(file_receiver, &file, position),
        Spool::build(chunk_downloads, REQUEST_COUNT).then(|x| async move {
            file_sender.close_channel();
            Ok(()) as WickResult<()>
        })
    );
    // wat
    r1?; r2?;

    Ok(())
}