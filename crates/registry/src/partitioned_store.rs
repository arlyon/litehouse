//! A registry of litehouse plugins
//!
//! The registry consists of an index, as well as a list of entries,
//! partitioned into a series of files. As of now, we use flatbuffers
//! to serialize the data, and memmap to map the files into memory
//! during reading.
//!
//! This is grouped into a few different layers:
//! - Registry: the top-level struct that manages the index and entries

use flatbuffers::{Follow, Verifiable};
use futures::{Future, stream::StreamExt};
use memmap2::Mmap;
use std::{marker::PhantomData, mem::transmute, sync::Mutex};
use tokio::sync::RwLock;

use crate::{
    io::IndexIOScheme,
    partition::{IntoBuffer, Partition},
    partition_scheme::PartitioningScheme,
};

pub struct Registry<'a, T, P>
where
    T: Follow<'a, Inner = T> + Verifiable + 'a,
    P: PartitioningScheme<'a, T>,
{
    partitioning: P,
    _phantom: PhantomData<&'a T>,
}

pub trait Partitionable {
    fn key(&self) -> &str;
}

impl<'a, T, P> Registry<'a, T, P>
where
    T: Follow<'a, Inner = T> + Verifiable,
    P: PartitioningScheme<'a, T>,
{
    pub fn new(p: P) -> Self {
        Self {
            partitioning: p,
            _phantom: PhantomData,
        }
    }

    /// converts between two partitioning schemes by iterating over all the keys and
    /// reinserting them according to the new scheme
    // async fn repartition<
    //     'b,
    //     T2: Follow<'b, Inner = T2> + Verifiable + 'b + Partitionable,
    //     P2: PartitioningScheme<'b, T2> + 'b,
    //     I2: IndexIOScheme<'b, T2>,
    // >(
    //     self,
    //     other: P2,
    //     io: I2,
    // ) -> Registry<'b, T2, P2, I2> {
    //     let r_next = Registry::<T2, P2, I2>::new(other, io);
    //     // let mut partitions = self.partitions();

    //     r_next
    // }

    // rust does not provide an API over flat mapping locks so there
    // is not much we can do to provide this API. leaving the impl
    // here as a reminder
    // fn entries(&'a self) -> impl Stream<Item = <T as Follow<'a>>::Inner> {
    //     self.partitions()
    //         .then(move |p| async { tokio_stream::iter(p.entries().await) })
    //         .flatten()
    // }

    pub async fn range(
        &'a self,
        prefix_start: &str,
        prefix_end: &str,
    ) -> Vec<&'a RwLock<Partition<'a, T>>> {
        let partitions = self.partitioning.range(prefix_start, prefix_end).unwrap();
        let mut out = vec![];
        for p in partitions {
            out.push(self.partitioning.open(p).await);
        }
        out
    }

    pub async fn insert<'b, IB, T2>(&'a self, item: IB) -> Result<(), ()>
    where
        IB: Partitionable + IntoBuffer<'b, T2> + for<'c> PartialOrd<&'c T::Inner> + 'a,
        T2: Follow<'b> + Verifiable + 'b,
        'a: 'b,
    {
        let partition = self.partitioning.get(item.key())?.await;
        let mut partition = partition.write().await;
        {
            let partition_ptr: &mut Partition<T> = &mut partition;
            // SAFETY: we have an exclusive lock here which is dropped when the insert ends
            let partition_ptr: &mut Partition<T> = unsafe { transmute(partition_ptr) };
            let _ = partition_ptr.insert(item).await;
        }
        Ok(())
    }

    /// Calculate how balanced the partitions are
    ///
    /// Load all of the partitions, and calculate the variance of the number of keys
    /// 0 means that all partitions have the same number of keys
    /// 1 means that the average is infinitely small relative to the maximum
    pub async fn variance(&'a self) -> f32 {
        let (max, sum, count) = self
            .partitions()
            .fold((0, 0, 0), |(max, sum, count), p_b| async move {
                let p_b = p_b.read().await;
                let b_count = {
                    let p_b_ptr: &Partition<T> = &p_b;
                    // SAFETY: we have an exclusive read lock here which is dropped when the calculation ends
                    let p_b_ptr: &Partition<T> = unsafe { transmute(p_b_ptr) };
                    p_b_ptr.count()
                };
                (std::cmp::max(max, b_count), sum + b_count, count + 1)
            })
            .await;
        let average = sum as f32 / count as f32;
        average / max as f32
    }

    fn partitions(&'a self) -> impl futures::Stream<Item = &'a RwLock<Partition<'a, T>>> {
        self.partitioning.all()
    }

    /// Get the number of partitions that are in use
    pub fn partition_count(&'a self) -> impl Future<Output = usize> + 'a {
        self.partitions().count()
    }

    pub async fn count(&'a self) -> usize {
        self.partitions()
            .then(|p| async {
                let p = p.read().await;
                let p: &Partition<T> = &p;
                // SAFETY: we have an exclusive read lock here which is dropped when the insert ends
                let p_ptr: &Partition<T> = unsafe { transmute(p) };
                p_ptr.count()
            })
            .fold(0, |acc, next| std::future::ready(acc + next))
            .await
    }
}

pub struct Index<'a, IO: IndexIOScheme<'a, T>, T: Follow<'a> + Verifiable> {
    #[allow(dead_code)]
    io: IO,
    mmap: Mutex<Option<Mmap>>,
    _phantom: PhantomData<&'a T>,
}

impl<'a, IO: IndexIOScheme<'a, T>, T: Follow<'a> + Verifiable> Index<'a, IO, T> {
    pub fn new(io: IO) -> Self {
        Self {
            io,
            mmap: Default::default(),
            _phantom: PhantomData,
        }
    }

    pub async fn insert(&self, _title: &str) {
        let _partition = self.mmap.lock();

        todo!()
    }

    /// Get all items in the index whose keys start with the given prefix.
    pub async fn find(&self, _prefix: &str) -> impl Iterator<Item = T> {
        let _partition = self.mmap.lock();
        std::iter::empty()
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::collections::hash_map::Entry as MapEntry;
    use std::path::PathBuf;
    use std::sync::Arc;

    use flatbuffers::{Follow, Verifiable};

    use futures::stream::iter;
    use futures::{Stream, StreamExt};
    use stable_deref_trait::StableDeref;

    use tokio::sync::{Mutex, RwLock};

    use crate::io::Index;
    use crate::partition::IntoEntry;

    use crate::{
        io::{IndexIOScheme, PartitionIOScheme},
        partition::Partition,
        partition_scheme::{Alphabetical, Split},
        proto::litehouse::Entry,
        registry::Registry,
    };

    struct FakeIOScheme<'a, T>
    where
        T: Follow<'a> + Verifiable,
    {
        _tmp: Option<temp_dir::TempDir>,
        path: PathBuf,
        mmap: std::cell::OnceCell<Index>,
        mmaps: Mutex<HashMap<usize, Box<RwLock<Partition<'a, T>>>>>,
    }

    unsafe fn extend_lifetime<'a, T: StableDeref>(ptr: &T) -> &'a T::Target {
        &*(&**ptr as *const T::Target)
    }

    impl<'a, T> IndexIOScheme<'a, T> for FakeIOScheme<'a, T>
    where
        T: Follow<'a> + Verifiable,
    {
        async fn open(&self) -> Index {
            let map = self.mmap.get_or_init(|| {
                let path = std::env::temp_dir().join("index.bin");
                let file = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(&path)
                    .unwrap();
                let data = unsafe { memmap2::Mmap::map(&file).unwrap() };
                Index {
                    mmap: Arc::new(Mutex::new(data)),
                }
            });
            map.to_owned()
        }
    }

    impl<'a, T> PartitionIOScheme<'a, T> for FakeIOScheme<'a, T>
    where
        T: Follow<'a> + Verifiable,
    {
        async fn open(&'a self, id: usize) -> &RwLock<Partition<T>> {
            let mut mmaps = self.mmaps.lock().await;
            match mmaps.entry(id) {
                MapEntry::Occupied(e) => unsafe {
                    tracing::debug!("partition {} already exists", id);
                    extend_lifetime(e.get())
                },
                MapEntry::Vacant(v) => unsafe {
                    tracing::debug!("loading partition {}", id);
                    extend_lifetime(v.insert(Box::new(RwLock::new(self.load(id).await))))
                },
            }
        }
    }

    impl<'a, T> FakeIOScheme<'a, T>
    where
        T: Follow<'a> + Verifiable,
    {
        async fn load(&'a self, id: usize) -> Partition<T> {
            let path = self.path.join(format!("index-{}.bin", id));
            let p = Partition::new(path).await;
            tracing::info!("loaded fake partition");
            p
        }
    }

    fn make_test_registry<'a, T>()
    -> Registry<'a, T, Alphabetical<'a, 1, T, Arc<FakeIOScheme<'a, T>>>>
    where
        T: Follow<'a, Inner = T> + Verifiable,
    {
        let tmp = temp_dir::TempDir::new().unwrap();
        let io = Arc::new(FakeIOScheme {
            path: tmp.path().to_owned(),
            _tmp: None,
            mmap: Default::default(),
            mmaps: Default::default(),
        });
        tmp.leak();
        Registry::new(Alphabetical::new([Split::Seven], io.clone()))
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn partitions() {
        let reg = make_test_registry::<Entry>();

        for line in ["tasmota", "weather", "samsung"] {
            let entry = IntoEntry {
                title: line.to_string(),
                version: (1, 0, 0),
                description: "Lorem ipsum".to_string(),
                schema: "{}".to_string(),
                sha: "deadbeef".to_string(),
                size: 12_235_332,
                capabilities: vec![],
            };

            tracing::info!("inserting {:?}", line);

            _ = reg.insert(entry).await;
            // let var = reg.variance().await;
        }

        assert_eq!(reg.count().await, 3);
        println!("{}", reg.variance().await);
    }
}
