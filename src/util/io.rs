use std::old_io;
use std::old_io::util::LimitReader;

pub struct LimitErrorReader<R> {
    inner: LimitReader<R>,
}

impl<R: Reader> LimitErrorReader<R> {
    pub fn new(r: R, limit: usize) -> LimitErrorReader<R> {
        LimitErrorReader { inner: LimitReader::new(r, limit) }
    }
}

impl<R: Reader> Reader for LimitErrorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> old_io::IoResult<usize> {
        self.inner.read(buf).map_err(|e| {
            if e.kind == old_io::EndOfFile && self.inner.limit() == 0 {
                old_io::IoError {
                    kind: old_io::OtherIoError,
                    desc: "maximum limit reached when reading",
                    detail: None,
                }
            } else {
                e
            }
        })
    }
}
