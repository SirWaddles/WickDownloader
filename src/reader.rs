use std::sync::Arc;
use std::io::{Seek, SeekFrom};
use tokio::io::AsyncReadExt;
use crate::err::WickResult;
use crate::chunks::ChunkReader;
use john_wick_parse::decompress::oodle;
use john_wick_parse::dispatch::{ReaderData, FIoStoreTocCompressedBlockEntry, FIoOffsetAndLength, align_value};

async fn get_block(reader: &mut ChunkReader, block: &FIoStoreTocCompressedBlockEntry) -> WickResult<Vec<u8>> {
    reader.seek(SeekFrom::Start(block.offset))?;

    let block_size = align_value(block.compressed_size, 16) as usize;
    let mut buf = vec![0u8; block_size];
    reader.read_exact(&mut buf).await?;

    if block.compression_method == 0 {
        return Ok(buf);
    }

    Ok(oodle::decompress_stream(block.size as u64, &buf).unwrap())
}

pub async fn get_chunk(reader: &mut ChunkReader, data: Arc<ReaderData>, chunk: &FIoOffsetAndLength) -> WickResult<Vec<u8>> {
    let length = chunk.length as usize;
    let mut buf = vec![0u8; length];
    let mut written: usize = 0;
    let mut pos = chunk.offset as usize;
    let block_size = data.get_header().get_block_size() as usize;

    while written < length {
        let block_idx = pos / block_size;
        let block = data.get_block(block_idx).unwrap();
        let block_data = get_block(reader, block).await?;
        let offset = pos % block_size;
        let to_write = std::cmp::min(block.size as usize - offset, length - written);

        let target_buf = &mut buf[written..(written + to_write)];
        target_buf.copy_from_slice(&block_data[offset..(offset + to_write)]);
        written += to_write;
        pos += to_write;
    }

    Ok(buf)
}