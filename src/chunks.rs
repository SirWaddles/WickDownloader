use crate::err::{WickResult, make_err};
use crate::http::HttpService;
use crate::manifest::{ChunkManifest, ChunkManifestFile, ChunkManifestChunkPart};
use std::convert::AsRef;
use std::io::{Cursor, Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use bytes::BytesMut;
use flate2::bufread::ZlibDecoder;

const TEST_DIST: &'static str = "https://epicgames-download1.akamaized.net/";

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
        cursor.read_exact(&mut data);
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
    fn new<T>(data: T, chunk: &ChunkManifestChunkPart) -> WickResult<Self> where T: AsRef<[u8]> {
        let mut cursor = Cursor::new(data);
        let magic = cursor.read_u32::<LittleEndian>()?;
        println!("Magic: {}", magic);
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
            println!("compressed");
        }

        Ok(Self {
            header, data
        })
    }
}

pub async fn download_file(http: &HttpService, manifest: &ChunkManifest, file: &ChunkManifestFile) -> WickResult<()> {
    let test_part = &file.get_chunks()[0];
    /*let url = TEST_DIST.to_owned() + &test_part.get_url(&manifest)?;
    let chunk = http.get_url(&url).await?;

    std::fs::write("test.chunk", chunk).unwrap();*/
    let chunk_data = std::fs::read("test.chunk").unwrap();
    let chunk = Chunk::new(chunk_data, &test_part)?;

    
    println!("test: {} {} {}", chunk.header.size, chunk.header.data_size, chunk.data.len());
    println!("data: {} {}", test_part.get_offset(), test_part.get_size());

    Ok(())
}