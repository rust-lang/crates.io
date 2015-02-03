use std::old_io;
use openssl::crypto::hash::{Hasher, Type};

pub struct HashingReader<R> {
    inner: R,
    hasher: Hasher,
}

impl<R: Reader> HashingReader<R> {
    pub fn new(r: R) -> HashingReader<R> {
        HashingReader { inner: r, hasher: Hasher::new(Type::SHA256) }
    }

    pub fn finalize(mut self) -> Vec<u8> { self.hasher.finish() }
}

impl<R: Reader> Reader for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> old_io::IoResult<usize> {
        let amt = try!(self.inner.read(buf));
        let _ = self.hasher.write_all(&buf[..amt]);
        return Ok(amt)
    }
}
