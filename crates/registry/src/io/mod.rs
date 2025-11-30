use std::sync::Arc;

use flatbuffers::{Follow, Verifiable};
use memmap2::Mmap;
use tokio::sync::{Mutex, RwLock};

use crate::partition::Partition;

pub mod s3;

/// Load orderable partitions from some source.
pub trait PartitionIOScheme<'a, T>
where
    T: Follow<'a> + Verifiable + 'a,
{
    fn open(&'a self, id: usize) -> impl futures::Future<Output = &'a RwLock<Partition<'a, T>>>;
}

impl<'a, P, T> PartitionIOScheme<'a, T> for Arc<P>
where
    P: PartitionIOScheme<'a, T>,
    T: Follow<'a> + Verifiable + 'a,
{
    async fn open(&'a self, id: usize) -> &'a RwLock<Partition<'a, T>> {
        self.as_ref().open(id).await
    }
}

#[derive(Clone)]
pub struct Index {
    pub mmap: Arc<Mutex<Mmap>>,
}

/// Load an index from some source.
pub trait IndexIOScheme<'a, T>
where
    T: Follow<'a> + Verifiable,
{
    fn open(&self) -> impl futures::Future<Output = Index>;
}

impl<'a, I, T> IndexIOScheme<'a, T> for Arc<I>
where
    T: Follow<'a> + Verifiable,
    I: IndexIOScheme<'a, T>,
{
    async fn open(&self) -> Index {
        self.as_ref().open().await
    }
}

#[cfg(test)]
mod test {
    use crate::io::{PartitionIOScheme as _, s3::MMapS3IoScheme};
    use crate::naming::NumericalPrefixed;
    use crate::proto::litehouse::Entry;

    #[tokio::test]
    async fn mmap_s3_io_scheme() {
        let scheme = NumericalPrefixed::new(".");
        let scheme = MMapS3IoScheme::<Entry, NumericalPrefixed>::new(scheme, None);
        let _partition = scheme.open(0).await;
    }
}
