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
        /*
            rustfmt wanted to merge the lines together so had to use this
            to stop this from occurring
        */
        #[cfg_attr(rustfmt, rustfmt_skip)]
        #[allow(deprecated)]
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
