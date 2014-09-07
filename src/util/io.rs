use std::io;
use std::io::util::LimitReader;

pub struct LimitErrorReader<R> {
    inner: LimitReader<R>,
}

impl<R: Reader> LimitErrorReader<R> {
    pub fn new(r: R, limit: uint) -> LimitErrorReader<R> {
        LimitErrorReader { inner: LimitReader::new(r, limit) }
    }
}

impl<R: Reader> Reader for LimitErrorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::IoResult<uint> {
        self.inner.read(buf).map_err(|e| {
            if e.kind == io::EndOfFile && self.inner.limit() == 0 {
                io::IoError {
                    kind: io::OtherIoError,
                    desc: "maximum limit reached when reading",
                    detail: None,
                }
            } else {
                e
            }
        })
    }
}
