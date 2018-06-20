use std::io;
use std::io::prelude::*;
use std::mem;

#[derive(Debug)]
pub struct LimitErrorReader<R> {
    inner: io::Take<R>,
}

impl<R: Read> LimitErrorReader<R> {
    pub fn new(r: R, limit: u64) -> LimitErrorReader<R> {
        LimitErrorReader {
            inner: r.take(limit),
        }
    }
}

impl<R: Read> Read for LimitErrorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(buf) {
            Ok(0) if self.inner.limit() == 0 => Err(io::Error::new(
                io::ErrorKind::Other,
                "maximum limit reached when reading",
            )),
            e => e,
        }
    }
}

pub fn read_le_u32<R: Read + ?Sized>(r: &mut R) -> io::Result<u32> {
    let mut b = [0; 4];
    read_fill(r, &mut b)?;
    Ok(
        u32::from(b[0])
            | (u32::from(b[1]) << 8)
            | (u32::from(b[2]) << 16)
            | (u32::from(b[3]) << 24),
    )
}

pub fn read_fill<R: Read + ?Sized>(r: &mut R, mut slice: &mut [u8]) -> io::Result<()> {
    while !slice.is_empty() {
        let n = r.read(slice)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::Other, "end of file reached"));
        }
        slice = &mut mem::replace(&mut slice, &mut [])[n..];
    }
    Ok(())
}
