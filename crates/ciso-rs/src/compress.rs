use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::mem;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use flate2::{Compress, Compression, FlushCompress};
use memmap2::Mmap;
use parking_lot::{Condvar, Mutex};

use crate::ciso_header::CisoHeader;

#[derive(Clone)]
enum Job {
    Block { index: usize },
    End,
}

#[derive(Clone)]
struct Block {
    index: usize,
    compressed: Vec<u8>, // empty = plain
}

#[expect(clippy::cast_possible_truncation)]
pub fn compress_ciso(mut input: File, mut output: File, level: u32) -> io::Result<()> {
    let num_cpus = num_cpus::get();
    let queue_cap = num_cpus * 2;
    let threads = num_cpus.max(1); // Nb of threads used for block compression

    let total_bytes = input.seek(SeekFrom::End(0))?;
    input.seek(SeekFrom::Start(0))?;

    let header = CisoHeader::new(total_bytes);
    let block_size = header.block_size as usize;
    let total_blocks = (total_bytes as usize).div_ceil(block_size);

    let index_size = (total_blocks + 1) * 4;
    let mut index = vec![0u32; total_blocks + 1];

    // Write header and empty bytes into output
    header.write_into(&mut output)?;
    output.write_all(&vec![0u8; index_size])?;

    let mmap = Arc::new(unsafe { Mmap::map(&input)? });

    let mut writer = BufWriter::with_capacity(1 << 20, output); // 1MiB
    let mut write_pos = mem::size_of::<CisoHeader>() as u64 + index_size as u64;

    let jobs = BoundedQueue::<Job>::new(queue_cap);
    let results = BoundedQueue::<Block>::new(queue_cap);

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    {
        let jobs = jobs.clone();
        handles.push(thread::spawn(move || {
            for index in 0..total_blocks {
                jobs.push(Job::Block { index });
            }
            // Send end signal to all threads
            for _ in 0..threads {
                jobs.push(Job::End);
            }
        }));
    }

    for _ in 0..threads {
        let jobs = jobs.clone();
        let results = results.clone();
        let mmap = mmap.clone();

        handles.push(thread::spawn(move || {
            compression_task(&jobs, &results, &mmap, block_size, level);
        }));
    }

    let mut next = 0usize;
    let mut pending = HashMap::with_capacity(queue_cap * threads * 2);

    while next < total_blocks {
        let block = results.pop();
        pending.insert(block.index, block);

        while let Some(block) = pending.remove(&next) {
            if (write_pos >> header.align) > u64::from(u32::MAX) {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "CSO too large"));
            }

            index[next] = (write_pos >> header.align) as u32;

            if block.compressed.is_empty() {
                index[next] |= 0x8000_0000;
                let start = next * block_size;
                let end = (start + block_size).min(mmap.len());
                writer.write_all(&mmap[start..end])?;
                write_pos += (end - start) as u64;
            } else {
                writer.write_all(&block.compressed)?;
                write_pos += block.compressed.len() as u64;
            }

            next += 1;
        }
    }

    index[total_blocks] = (write_pos >> header.align) as u32;

    writer.seek(SeekFrom::Start(mem::size_of::<CisoHeader>() as u64))?;
    for i in index {
        writer.write_all(&i.to_le_bytes())?;
    }

    writer.flush()?;

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}

#[expect(clippy::cast_possible_truncation)]
fn compression_task(
    jobs: &BoundedQueue<Job>,
    results: &BoundedQueue<Block>,
    mmap: &Mmap,
    block_size: usize,
    level: u32,
) {
    let mut out_buf = vec![0u8; block_size * 2];
    let mut comp = Compress::new(Compression::new(level), false);

    loop {
        match jobs.pop() {
            Job::End => break,
            Job::Block { index } => {
                comp.reset();

                let start = index * block_size;
                let end = (start + block_size).min(mmap.len());
                let input = &mmap[start..end];

                comp.compress(input, &mut out_buf, FlushCompress::Finish)
                    .unwrap();
                let size = comp.total_out() as usize;

                let mut compressed = Vec::new();
                if size < input.len() {
                    compressed.reserve(size);
                    compressed.extend_from_slice(&out_buf[..size]);
                }

                results.push(Block { index, compressed });
            }
        }
    }
}

#[derive(Clone)]
struct BoundedQueue<T> {
    capacity: usize,
    inner: Arc<(Mutex<VecDeque<T>>, Condvar)>,
}

impl<T> BoundedQueue<T> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            inner: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
        }
    }

    fn push(&self, val: T) {
        let (lock, cvar) = &*self.inner;
        let mut queue = lock.lock();
        while queue.len() >= self.capacity {
            cvar.wait(&mut queue);
        }
        queue.push_back(val);
        cvar.notify_one();
    }

    fn pop(&self) -> T {
        let (lock, cvar) = &*self.inner;
        let mut queue = lock.lock();
        while queue.is_empty() {
            cvar.wait(&mut queue);
        }
        let value = queue.pop_front().unwrap(); // Safety: emptyness checked above
        cvar.notify_one();
        value
    }
}
