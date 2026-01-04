use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};

use flate2::{Decompress, FlushDecompress, Status};

use crate::ciso_header::CisoHeader;

#[expect(clippy::cast_possible_truncation)]
pub fn decompress_ciso(mut input: File, mut output: File) -> io::Result<()> {
    let header = CisoHeader::read_from(&mut input)?;
    let block_size = header.block_size as usize;
    let total_blocks = (header.total_bytes as usize) / block_size;

    let mut index = vec![0u32; total_blocks + 1];
    for v in &mut index {
        let mut buf = [0u8; 4];
        input.read_exact(&mut buf)?;
        *v = u32::from_le_bytes(buf);
    }

    let mut in_buf = vec![0u8; block_size * 2];
    let mut out_buf = vec![0u8; block_size];

    for i in 0..total_blocks {
        let raw = index[i];
        let plain = raw & 0x8000_0000 != 0;

        let off = u64::from(raw & 0x7fff_ffff) << header.align;
        input.seek(SeekFrom::Start(off))?;

        if plain {
            input.read_exact(&mut out_buf)?;
        } else {
            let next = u64::from(index[i + 1] & 0x7fff_ffff) << header.align;
            let size = (next - off) as usize;

            input.read_exact(&mut in_buf[..size])?;

            let status = Decompress::new(false).decompress(
                &in_buf[..size],
                &mut out_buf,
                FlushDecompress::Finish,
            )?;

            if status != Status::StreamEnd {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "zlib error"));
            }
        }

        output.write_all(&out_buf)?;
    }

    Ok(())
}
