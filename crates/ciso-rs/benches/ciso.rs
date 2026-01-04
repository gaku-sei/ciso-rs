use std::io::Write;
use std::{fs::File, time::Duration};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use tempfile::tempdir;

use ciso_rs::{compress_ciso, decompress_ciso};

const BLOCK_SIZE: usize = 2048;
const ISO_SIZE: usize = 128 * 1024 * 1024; // 128 MiB

fn make_fake_iso(file: &mut File, size: usize, block_size: usize) {
    for i in 0..(size / block_size) {
        let mut block = vec![0u8; block_size];

        if i % 2 == 0 {
            block.fill(0);
        } else {
            getrandom::fill(&mut block).unwrap();
        }

        file.write_all(&block).unwrap();
    }
}

fn bench_ciso(c: &mut Criterion) {
    let tmp = tempdir().unwrap();
    let iso_path = tmp.path().join("input.iso");
    let cso_path = tmp.path().join("output.cso");
    let out_path = tmp.path().join("output.iso");

    {
        let mut file = File::create(&iso_path).unwrap();
        make_fake_iso(&mut file, ISO_SIZE, BLOCK_SIZE);
    }

    let mut group = c.benchmark_group("ciso");

    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(10));

    group.bench_function(BenchmarkId::new("compress", "level6"), |b| {
        b.iter(|| {
            compress_ciso(
                File::open(&iso_path).unwrap(),
                File::create(&cso_path).unwrap(),
                6,
            )
            .unwrap();
        });
    });

    group.bench_function(BenchmarkId::new("decompress", "simple"), |b| {
        b.iter(|| {
            decompress_ciso(
                File::open(&cso_path).unwrap(),
                File::create(&out_path).unwrap(),
            )
            .unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_ciso);
criterion_main!(benches);
