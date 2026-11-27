//! Remembers the last 512 bytes read from a tar stream.
//!
//! Each entry in a tar archive starts with a header, followed by the file
//! contents. The header is always exactly 512 bytes long and stores, among
//! other things, the size of the contents. Extra metadata, like pax records,
//! comes as separate data in front of the header. A pax `size` record
//! overrides the size in the header.
//!
//! Newer Cargo versions use the pax size, like `tar-codec` does. Older Cargo
//! versions use the header size. If the two sizes differ, old and new Cargo
//! versions extract different files from the same archive, so crates.io
//! rejects such archives. This rules out a parser differential that could
//! hide files from one Cargo version or from the checks in this crate.
//! `tar-codec` 0.0.17 only reports the final size, so this module recovers
//! the raw header to compare both sizes.
//!
//! [`LastBlockReader`] sits between the gzip decoder and `tar-codec`, so all
//! bytes of the archive pass through it. It keeps a copy of the last 512
//! bytes and a count of all bytes so far, and shares both through a
//! [`LastBlock`].
//!
//! `tar-codec` reads one entry at a time. It reads the header, stops, and
//! returns the entry. It reads the contents later, and only if needed. So
//! right after `tar-codec` returns an entry, the copy holds its header:
//!
//! ```text
//!   ... | pax records | header | contents | next entry ...
//!                     ^        ^
//!                     |        +-- tar-codec stopped reading here
//!                     +-- position of the entry
//! ```
//!
//! [`LastBlock::get()`] makes sure of this. It returns the copy only if the
//! count is exactly 512 bytes past the position of the entry. Otherwise it
//! returns `None`, and the caller rejects the tarball.
//!
//! TODO: Remove this module, `check_header_size()` and
//! `TarballError::SizeMismatch` once
//! <https://github.com/astral-sh/astral-codec/pull/404> is released, and use
//! `PaxDecodePolicy::allow_size_mismatch(false)` instead.

use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll};

use tar_framing::{BLOCK_SIZE, Block};
use tokio::io::{AsyncRead, ReadBuf};

/// Wraps a reader and records the bytes read from it in a [`LastBlock`].
///
/// The wrapper passes all bytes through unchanged.
pub struct LastBlockReader<R> {
    inner: R,
    last_block: LastBlock,
}

impl<R> LastBlockReader<R> {
    /// Wraps `inner` and returns the wrapper together with a [`LastBlock`]
    /// handle to its recording.
    ///
    /// The handle lets the caller look at the recorded bytes while the
    /// wrapper itself is owned by `tar-codec`.
    pub fn new(inner: R) -> (Self, LastBlock) {
        let last_block = LastBlock::default();
        let reader = Self {
            inner,
            last_block: last_block.clone(),
        };
        (reader, last_block)
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for LastBlockReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // `buf` may already contain bytes from earlier reads. Only the bytes
        // that the inner reader adds now are new.
        let start = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = result {
            // Readers must only add bytes to `buf`. If the inner reader
            // removed some anyway, the recording would be wrong, so fail the
            // read instead.
            let Some(new) = buf.filled().get(start..) else {
                let message = "inner reader removed bytes from the read buffer";
                return Poll::Ready(Err(io::Error::other(message)));
            };
            self.last_block.record(new);
        }
        result
    }
}

/// A shared handle to the last 512 bytes read through a [`LastBlockReader`].
///
/// Cloning the handle does not copy the recorded bytes. All clones see the
/// same data, which lets the caller look at what the reader recorded.
#[derive(Clone, Debug, Default)]
pub struct LastBlock(Arc<Mutex<RecordedBytes>>);

impl LastBlock {
    /// Returns the 512 bytes starting at `position` if they are the last 512
    /// bytes that were read.
    ///
    /// Returns `None` if fewer or more bytes were read since `position`.
    pub fn get(&self, position: u64) -> Option<Block> {
        let recorded = self.lock();
        // Returns `None` if fewer than 512 bytes were read so far.
        let last_bytes_start = recorded.bytes_read.checked_sub(BLOCK_SIZE as u64)?;
        if position != last_bytes_start {
            return None;
        }
        Some(recorded.last_bytes)
    }

    /// Adds the bytes of one read to the recording.
    fn record(&self, read: &[u8]) {
        let mut recorded = self.lock();
        recorded.bytes_read += read.len() as u64;

        // Only the last 512 bytes of a read can end up in the recording.
        let new = &read[read.len().saturating_sub(BLOCK_SIZE)..];

        // Move the bytes that are still recent to the front, then put the
        // new bytes at the end. With `[a b c d]` as the last bytes and `[e f]`
        // as the new bytes, this results in `[c d e f]`.
        recorded.last_bytes.copy_within(new.len().., 0);
        recorded.last_bytes[BLOCK_SIZE - new.len()..].copy_from_slice(new);
    }

    fn lock(&self) -> MutexGuard<'_, RecordedBytes> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Debug)]
struct RecordedBytes {
    /// The last 512 bytes that were read, oldest first. Until 512 bytes have
    /// been read, the front is filled with zeros.
    last_bytes: Block,
    /// The number of bytes read so far, which is also the position in the
    /// stream right after `last_bytes`.
    bytes_read: u64,
}

impl Default for RecordedBytes {
    fn default() -> Self {
        let last_bytes = [0; BLOCK_SIZE];
        Self {
            last_bytes,
            bytes_read: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BLOCK_SIZE, Block, LastBlock, LastBlockReader};
    use tokio::io::AsyncReadExt;

    const HEADER: Block = [1; BLOCK_SIZE];
    const EMPTY: Block = [0; BLOCK_SIZE];

    /// Reads all of `data` through a [`LastBlockReader`], using reads of at
    /// most `read_len` bytes, and returns the recording.
    async fn read_all(data: &[u8], read_len: usize) -> LastBlock {
        let (mut reader, last_block) = LastBlockReader::new(data);
        let mut buf = vec![0; read_len];
        while reader.read(&mut buf).await.unwrap() > 0 {}
        last_block
    }

    #[tokio::test]
    async fn header_read_in_several_small_reads() {
        let data = [EMPTY, HEADER].concat();
        let last_block = read_all(&data, 100).await;
        assert_some_eq!(last_block.get(BLOCK_SIZE as u64), HEADER);
    }

    #[tokio::test]
    async fn header_at_the_end_of_one_large_read() {
        let data = [EMPTY, EMPTY, HEADER].concat();
        let last_block = read_all(&data, data.len()).await;
        assert_some_eq!(last_block.get(2 * BLOCK_SIZE as u64), HEADER);
    }

    #[tokio::test]
    async fn nothing_read() {
        let last_block = read_all(&[], BLOCK_SIZE).await;
        assert_none!(last_block.get(0));
    }

    #[tokio::test]
    async fn more_bytes_read_after_the_header() {
        let data = [HEADER.as_slice(), &[0]].concat();
        let last_block = read_all(&data, BLOCK_SIZE).await;
        assert_none!(last_block.get(0));
    }
}
