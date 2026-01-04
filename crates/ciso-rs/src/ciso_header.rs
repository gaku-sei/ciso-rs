use std::io;
use std::mem;
use std::slice;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CisoHeader {
    pub magic: [u8; 4],   // 'C','I','S','O'
    pub header_size: u32, // == 0x18
    pub total_bytes: u64,
    pub block_size: u32, // 0x800
    pub ver: u8,         // 0x01
    pub align: u8,
    pub rsv_06: [u8; 2],
}

impl CisoHeader {
    #[must_use]
    #[expect(clippy::cast_possible_truncation)] // CisoHeader will always fit u32
    pub fn new(total_bytes: u64) -> Self {
        Self {
            magic: *b"CISO",
            header_size: mem::size_of::<CisoHeader>() as u32,
            total_bytes,
            block_size: 0x800,
            ver: 0x01,
            align: 0,
            rsv_06: [0; 2],
        }
    }

    pub fn read_from(r: &mut impl io::Read) -> io::Result<Self> {
        read_struct(r)
    }

    pub fn write_into(&self, w: &mut impl io::Write) -> io::Result<()> {
        write_struct(w, self)
    }
}

fn read_struct<T>(r: &mut impl io::Read) -> io::Result<T> {
    let mut val = mem::MaybeUninit::<T>::uninit();
    unsafe {
        let buf = slice::from_raw_parts_mut(val.as_mut_ptr().cast::<u8>(), mem::size_of::<T>());
        r.read_exact(buf)?;
        Ok(val.assume_init())
    }
}

fn write_struct<T>(w: &mut impl io::Write, val: &T) -> io::Result<()> {
    unsafe {
        let buf = slice::from_raw_parts(std::ptr::from_ref(val).cast::<u8>(), mem::size_of::<T>());
        w.write_all(buf)
    }
}
