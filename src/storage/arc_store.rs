use async_trait::async_trait;
use axum::body::Bytes;
use futures_util::stream::BoxStream;
use object_store::path::Path;
use object_store::{
    GetOptions, GetResult, ListResult, MultipartId, ObjectMeta, ObjectStore, Result,
};
use std::fmt::Debug;
use std::ops::Range;
use std::sync::Arc;
use tokio::io::AsyncWrite;

// This can be removed once https://github.com/apache/arrow-rs/pull/4502 is
// released.
#[derive(Debug, Clone)]
pub struct ArcStore(Arc<dyn ObjectStore>);

impl ArcStore {
    pub fn new<S: ObjectStore>(store: S) -> Self {
        Self(Arc::new(store))
    }
}

impl std::fmt::Display for ArcStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

#[async_trait]
impl ObjectStore for ArcStore {
    async fn put(&self, location: &Path, bytes: Bytes) -> Result<()> {
        self.0.put(location, bytes).await
    }

    async fn put_multipart(
        &self,
        location: &Path,
    ) -> Result<(MultipartId, Box<dyn AsyncWrite + Unpin + Send>)> {
        self.0.put_multipart(location).await
    }

    async fn abort_multipart(&self, location: &Path, multipart_id: &MultipartId) -> Result<()> {
        self.0.abort_multipart(location, multipart_id).await
    }

    async fn append(&self, location: &Path) -> Result<Box<dyn AsyncWrite + Unpin + Send>> {
        self.0.append(location).await
    }

    async fn get(&self, location: &Path) -> Result<GetResult> {
        self.0.get(location).await
    }

    async fn get_opts(&self, location: &Path, options: GetOptions) -> Result<GetResult> {
        self.0.get_opts(location, options).await
    }

    async fn get_range(&self, location: &Path, range: Range<usize>) -> Result<Bytes> {
        self.0.get_range(location, range).await
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<usize>]) -> Result<Vec<Bytes>> {
        self.0.get_ranges(location, ranges).await
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        self.0.head(location).await
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        self.0.delete(location).await
    }

    fn delete_stream<'a>(
        &'a self,
        locations: BoxStream<'a, Result<Path>>,
    ) -> BoxStream<'a, Result<Path>> {
        self.0.delete_stream(locations)
    }

    async fn list(&self, prefix: Option<&Path>) -> Result<BoxStream<'_, Result<ObjectMeta>>> {
        self.0.list(prefix).await
    }

    async fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> Result<BoxStream<'_, Result<ObjectMeta>>> {
        self.0.list_with_offset(prefix, offset).await
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        self.0.list_with_delimiter(prefix).await
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        self.0.copy(from, to).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        self.0.rename(from, to).await
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        self.0.copy_if_not_exists(from, to).await
    }

    async fn rename_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        self.0.rename_if_not_exists(from, to).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::Bytes;
    use object_store::memory::InMemory;

    #[tokio::test]
    async fn arc_store_sharing() {
        let storage1 = ArcStore::new(InMemory::new());
        let storage2 = storage1.clone();

        let path = "test".into();
        storage1.put(&path, Bytes::new()).await.unwrap();

        storage1.head(&path).await.unwrap();
        storage2.head(&path).await.unwrap();
    }
}
