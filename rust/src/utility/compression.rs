use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::io::prelude::*;

/// Compress bytes with gzip
pub fn compress_gzip(bytes: &mut [u8]) -> anyhow::Result<()> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    Ok(encoder.write_all(bytes)?)
}

/// Decompress gzip compressed bytes
pub(crate) fn decompress_gzip(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut buf = vec![];
    let mut gz = GzDecoder::new(bytes);

    gz.read_to_end(&mut buf)?;
    Ok(buf)
}
