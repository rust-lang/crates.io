use std::io::prelude::*;
use std::io;
use openssl::hash::{Hasher, MessageDigest};

pub struct HashingReader<R> {
    inner: R,
    hasher: Hasher,
}

impl<R: Read> HashingReader<R> {
    pub fn new(r: R) -> HashingReader<R> {
        HashingReader {
            inner: r,
            hasher: Hasher::new(MessageDigest::sha256()).unwrap(),
        }
    }

    pub fn finalize(mut self) -> Vec<u8> {
        self.hasher.finish().unwrap()
    }
}

impl<R: Read> Read for HashingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let amt = self.inner.read(buf)?;
        self.hasher.update(&buf[..amt]).unwrap();
        Ok(amt)
    }
}
