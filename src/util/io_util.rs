use std::io::prelude::*;
use std::io;
use std::mem;

pub struct LimitErrorReader<R> {
    inner: io::Take<R>,
}

impl<R: Read> LimitErrorReader<R> {
    pub fn new(r: R, limit: u64) -> LimitErrorReader<R> {
        LimitErrorReader { inner: r.take(limit) }
    }
}

impl<R: Read> Read for LimitErrorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(buf) {
            Ok(0) if self.inner.limit() == 0 => {
                Err(io::Error::new(io::ErrorKind::Other, "maximum limit reached when reading"),)
            }
            e => e,
        }
    }
}

pub fn read_le_u32<R: Read + ?Sized>(r: &mut R) -> io::Result<u32> {
    let mut b = [0; 4];
    read_fill(r, &mut b)?;
    Ok((b[0] as u32) | ((b[1] as u32) << 8) | ((b[2] as u32) << 16) | ((b[3] as u32) << 24),)
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
