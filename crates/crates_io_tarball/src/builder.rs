use flate2::Compression;
use flate2::read::GzEncoder;
use std::io::Read;

pub struct TarballBuilder {
    inner: tar::Builder<Vec<u8>>,
}

impl TarballBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let inner = tar::Builder::new(vec![]);
        Self { inner }
    }

    pub fn add_file(mut self, path: &str, content: &[u8]) -> Self {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_cksum();
        self.inner.append_data(&mut header, path, content).unwrap();

        self
    }

    /// Adds a directory entry to the tarball.
    pub fn add_dir(mut self, path: &str) -> Self {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Directory);
        header.set_size(0);
        header.set_cksum();
        self.inner
            .append_data(&mut header, path, std::io::empty())
            .unwrap();

        self
    }

    pub fn add_pax_extensions<'key, 'value, I>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = (&'key str, &'value [u8])>,
    {
        self.inner.append_pax_extensions(headers).unwrap();

        self
    }

    pub fn add_symlink(mut self, path: &str, target: &str) -> Self {
        let mut header = tar::Header::new_gnu();
        header.set_path(path).unwrap();
        header.set_link_name(target).unwrap();
        header.set_size(0);
        header.set_cksum();

        self.inner.append(&header, b"".as_slice()).unwrap();

        self
    }

    pub fn build_unzipped(self) -> Vec<u8> {
        self.inner.into_inner().unwrap()
    }

    pub fn build(self) -> Vec<u8> {
        self.build_with_compression(Compression::default())
    }

    pub fn build_with_compression(self, compression: Compression) -> Vec<u8> {
        let tarball_bytes = self.build_unzipped();

        let mut gzip_bytes = vec![];
        GzEncoder::new(tarball_bytes.as_slice(), compression)
            .read_to_end(&mut gzip_bytes)
            .unwrap();

        gzip_bytes
    }
}

impl AsMut<tar::Builder<Vec<u8>>> for TarballBuilder {
    fn as_mut(&mut self) -> &mut tar::Builder<Vec<u8>> {
        &mut self.inner
    }
}
