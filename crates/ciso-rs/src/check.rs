use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::mem;

use flate2::{Decompress, FlushDecompress};

use crate::ciso_header::CisoHeader;

#[expect(clippy::cast_possible_truncation)]
pub fn check_ciso(mut file: File, full: bool) -> io::Result<()> {
    let file_len = file.seek(SeekFrom::End(0))?;
    file.seek(SeekFrom::Start(0))?;

    let header = CisoHeader::read_from(&mut file)?;
    let block_size = header.block_size as usize;
    let total_blocks = (header.total_bytes as usize).div_ceil(block_size);

    let index_pos = mem::size_of::<CisoHeader>() as u64;
    file.seek(SeekFrom::Start(index_pos))?;

    let mut index = vec![0u32; total_blocks + 1];
    for v in &mut index {
        let mut buf = [0u8; 4];
        file.read_exact(&mut buf)?;
        *v = u32::from_le_bytes(buf);
    }

    let data_start = index_pos + index.len() as u64 * 4;

    let end_off = u64::from(index[total_blocks] & 0x7fff_ffff) << header.align;
    if end_off > file_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "index end exceeds file size",
        ));
    }

    let mut prev_off = data_start;

    for i in 0..total_blocks {
        let raw = index[i];
        let plain = raw & 0x8000_0000 != 0;

        let off = u64::from(raw & 0x7fff_ffff) << header.align;
        let next = u64::from(index[i + 1] & 0x7fff_ffff) << header.align;

        if off < data_start {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("block {i}: offset before data start"),
            ));
        }

        if off < prev_off {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("non-monotonic index at block {i}"),
            ));
        }

        let expected_size = if i + 1 == total_blocks {
            header.total_bytes as usize - i * block_size
        } else {
            block_size
        };

        let size = if plain {
            expected_size as u64
        } else {
            next.checked_sub(off).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "negative compressed size")
            })?
        };

        if plain && next - off != expected_size as u64 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid plain block size at {i}"),
            ));
        }

        if off + size > file_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("block {i} exceeds file size"),
            ));
        }

        if full && !plain {
            if size > (block_size as u64 * 2) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("compressed block {i} too large"),
                ));
            }

            file.seek(SeekFrom::Start(off))?;
            let mut buf = vec![0u8; size as usize];
            file.read_exact(&mut buf)?;

            let mut out = vec![0u8; expected_size];
            let mut decomp = Decompress::new(false);

            decomp
                .decompress(&buf, &mut out, FlushDecompress::Finish)
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("invalid zlib at {i}"))
                })?;

            if decomp.total_out() as usize != expected_size {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid decompressed size at {i}"),
                ));
            }
        }

        prev_off = off;
    }

    Ok(())
}
