use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use ciso_rs::{check_ciso, compress_ciso, decompress_ciso};

const BLOCK_SIZE: usize = 2048;
const ISO_SIZE: usize = 32 * 1024 * 1024; // 32 MiB

#[expect(clippy::cast_possible_truncation)]
fn make_fake_iso(path: &PathBuf, size: usize, block_size: usize) -> std::io::Result<()> {
    let mut file = File::create(path)?;

    for i in 0..(size / block_size) {
        let mut block = vec![0u8; block_size];

        // Alternate compressible and uncompressible blocks
        if i % 3 == 0 {
            block.fill(0);
        } else if i % 3 == 1 {
            for (j, b) in block.iter_mut().enumerate() {
                *b = (j as u8).wrapping_add(i as u8);
            }
        } else {
            getrandom::fill(&mut block).unwrap();
        }

        file.write_all(&block)?;
    }

    Ok(())
}

#[test]
fn ciso_compress_check_decompress_roundtrip() -> std::io::Result<()> {
    let tmp = tempfile::tempdir()?;

    let iso_path = tmp.path().join("input.iso");
    let cso_path = tmp.path().join("output.cso");
    let out_path = tmp.path().join("output.iso");

    make_fake_iso(&iso_path, ISO_SIZE, BLOCK_SIZE)?;

    compress_ciso(File::open(&iso_path)?, File::create(&cso_path)?, 6)?;
    check_ciso(File::open(&cso_path)?, true)?;
    decompress_ciso(File::open(&cso_path)?, File::create(&out_path)?)?;

    let mut orig = Vec::new();
    let mut out = Vec::new();

    File::open(&iso_path)?.read_to_end(&mut orig)?;
    File::open(&out_path)?.read_to_end(&mut out)?;

    assert_eq!(orig.len(), out.len());
    assert_eq!(orig, out);

    Ok(())
}
