use dotenvy_macro::dotenv;
use nanoid::nanoid;
use tokio::{
    fs::{File, OpenOptions},
    io::{self, AsyncRead, AsyncReadExt},
};

const TEMP_PATH: &str = dotenv!("TEMP_PATH");
const NANOID_ALPHABET: &[char] = &[
    '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'A', 'b', 'B', 'c', 'C', 'd', 'D', 'e',
    'E', 'f', 'F', 'g', 'G', 'h', 'H', 'i', 'I', 'j', 'J', 'k', 'K', 'l', 'L', 'm', 'M', 'n', 'N',
    'o', 'O', 'p', 'P', 'q', 'Q', 'r', 'R', 's', 'S', 't', 'T', 'u', 'U', 'v', 'V', 'w', 'W', 'x',
    'X', 'y', 'Y', 'z', 'Z',
];
const CHUNK_SIZE: usize = 2048; // smaller chunk size = less memory usage, slower uploads
pub const ENC_CHUNK_SIZE: usize = CHUNK_SIZE;
pub const DEC_CHUNK_SIZE: usize = CHUNK_SIZE + 16;

pub async fn read_chunk<R>(reader: &mut R, size: usize) -> io::Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let mut chunk = Vec::with_capacity(size);
    let mut take = reader.take(size as u64);
    take.read_to_end(&mut chunk).await?;

    Ok(chunk)
}

pub async fn temp_file() -> io::Result<(File, String)> {
    let temp_id = nanoid!();
    let temp_path = format!("{TEMP_PATH}{temp_id}");
    let temp_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)
        .await?;
    Ok((temp_file, temp_path))
}

pub fn friendly_id(len: usize) -> String {
    nanoid!(len, &NANOID_ALPHABET)
}
