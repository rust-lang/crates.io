use std::io;
use openssl::crypto::hash::{Hasher, HashType};

pub struct HashingReader<R> {
    inner: R,
    hasher: Hasher,
}

impl<R: Reader> HashingReader<R> {
    pub fn new(r: R) -> HashingReader<R> {
        HashingReader { inner: r, hasher: Hasher::new(HashType::SHA256) }
    }

    pub fn finalize(self) -> Vec<u8> { self.hasher.finalize() }
}

impl<R: Reader> Reader for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::IoResult<uint> {
        let amt = try!(self.inner.read(buf));
        self.hasher.update(buf.slice_to(amt));
        return Ok(amt)
    }
}
