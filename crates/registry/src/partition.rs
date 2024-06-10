use std::{marker::PhantomData, ops::Range, path::PathBuf};

use flatbuffers::{FlatBufferBuilder, Follow, Verifiable, WIPOffset, SIZE_SIZEPREFIX};

use memmap2::Mmap;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt as _, BufReader},
};

use crate::{
    proto::litehouse::{Entry, EntryArgs, Version},
    registry::Partitionable,
};

/// An element in a partitioning scheme.
///
/// A partition is a list of size-prefixed flatbuffer objects stored sequentially in a file.
/// That way, we can lazily stream data as needed rather than loading the entire file into memory.
/// This is particularly useful for inserts.
///
/// At the start of the file is a u64 with the number of elements in the file.
pub struct Partition<'a, T>
where
    T: Follow<'a> + Verifiable,
{
    access: Mmap,
    path: PathBuf,
    file: File,
    size: u64,
    _type: PhantomData<&'a T>,
}

impl<'a, T> Partition<'a, T>
where
    T: Follow<'a> + Verifiable,
{
    pub async fn new<P: Into<PathBuf>>(path: P) -> Self {
        let path = path.into();
        tracing::debug!("opening partition {path:?}");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .await
            .unwrap();
        let data = unsafe { memmap2::Mmap::map(&file).unwrap() };
        let size = file.metadata().await.unwrap().len();
        tracing::debug!("opened partition {path:?} with size {size}");
        tracing::debug!("slice size is {}", data.len());

        Self {
            access: data,
            path,
            file,
            size,
            _type: PhantomData,
        }
    }

    /// Get all the offsets of the chunks in the partition
    fn elements(&self) -> impl Iterator<Item = Range<usize>> + '_ {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&self.path)
            .unwrap()
            .metadata()
            .unwrap();
        tracing::debug!("checking file metadata again: {file:?}");

        Self::elements_inner(&self.access)
    }

    #[tracing::instrument(skip(access))]
    fn elements_inner(access: &[u8]) -> impl Iterator<Item = Range<usize>> + '_ {
        let mut offset = 0;
        std::iter::from_fn(move || {
            tracing::trace!("checking offset {offset:?} out of {}", access.len());
            let len_arr = access.get(offset..offset + SIZE_SIZEPREFIX)?;

            let len = u32::from_le_bytes(len_arr.try_into().unwrap()) as usize;
            tracing::debug!("took LEN {len:?} from {len_arr:?}");
            let range = offset..offset + len + SIZE_SIZEPREFIX;
            offset = offset + len + SIZE_SIZEPREFIX;
            tracing::trace!("found entry at {range:?}, new offset {offset:?}");
            Some(range)
        })
    }

    /// Get all the entries in the partition. This data is sorted.
    fn entries_with_offset(&'a self) -> impl Iterator<Item = (T::Inner, Range<usize>)> {
        let elems = self.elements();
        let closure = move |range: Range<usize>| {
            let entry = &self.access[range.clone()];
            tracing::trace!("loading data from {entry:?}");
            let entry = flatbuffers::size_prefixed_root::<T>(entry).unwrap();
            (entry, range)
        };

        elems.map(closure)
    }

    /// Get all the entries in the partition. This data is sorted.
    pub fn entries(&'a self) -> impl Iterator<Item = T::Inner> {
        self.entries_with_offset().map(|(entry, _)| entry)
    }

    pub fn count(&'a self) -> usize {
        self.entries().count()
    }

    /// Read all the data up to the entry we want to insert, write it, then all the remaining
    /// to a new file. Finally, rename the new file to the old file.
    ///
    /// Note: rust does not make it easy to express that T2 and T must be the same modulo
    /// lifetimes, so it is up to the programmer to ensure that T2 == T.
    #[tracing::instrument(skip(self, insert))]
    pub async fn insert<'b, IB, T2>(&'b mut self, insert: IB)
    where
        IB: IntoBuffer<'b, T2> + for<'c> PartialOrd<&'c T::Inner>,
        T2: Follow<'b> + Verifiable,
    {
        // load the existing entry
        let tmp = std::env::temp_dir().join("index-new.bin"); // todo: use a real temp dir
        tracing::debug!("using temp file {tmp:?}");
        let mut file = OpenOptions::new()
            .truncate(true)
            .write(true)
            .create(true)
            .open(&tmp)
            .await
            .unwrap();
        let mut cursor = BufReader::new(&mut file);

        {
            let lower_bound = {
                let mut start = None;
                tracing::debug!("looking for insertion point");
                for x in self.elements() {
                    let entry = &self.access[x.clone()];

                    tracing::debug!("checking {x:?} {entry:?}");

                    // SAFETY: due to the way flatbuffers works, T needs a lifetime that is
                    // tied to that of the struct. This causes the borrow checker to complain
                    // about reborrows later, however we know here that entry is dropped
                    let entry: &'static [u8] = unsafe { core::mem::transmute(entry) };

                    let entry = flatbuffers::size_prefixed_root::<T>(entry).unwrap();
                    if insert < &entry {
                        start = Some(x.start);
                        break;
                    }
                }

                match start {
                    Some(start) => start,
                    None => {
                        tracing::debug!("inserting at end");
                        self.access.len()
                    }
                }
            };

            let bytes = cursor.write(&self.access[0..lower_bound]).await.unwrap();
            tracing::trace!("wrote {bytes} bytes from 0 to {lower_bound}");

            // write the new entry to the new file
            let mut builder = FlatBufferBuilder::new();
            let offset = insert.into_buffer(&mut builder);
            builder.finish_size_prefixed(offset, None);
            let bytes = builder.finished_data();
            tracing::trace!("wrote {} bytes", bytes.len());
            let written = cursor.write(bytes).await.unwrap();

            let bytes = cursor.write(&self.access[lower_bound..]).await.unwrap();
            tracing::trace!(
                "wrote {bytes} bytes from {} to {}",
                lower_bound,
                self.access.len()
            );
        }

        // move the file
        tracing::trace!("moving {tmp:?} to {:?}", self.path);
        tokio::fs::rename(tmp, &self.path).await.unwrap();

        // we store the file already but lets reopen it to be safe
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
            .await
            .unwrap();

        self.size = file.metadata().await.unwrap().len();
        self.access = unsafe { Mmap::map(&file) }.unwrap();
        self.file = file;
    }
}

pub trait IntoBuffer<'a, T: Follow<'a> + Verifiable> {
    fn into_buffer(self, builder: &mut FlatBufferBuilder<'a>) -> WIPOffset<T>;
}

pub struct IntoEntry {
    pub title: String,
    pub version: (u16, u16, u16),
    pub description: String,
    pub capabilities: Vec<String>,
    pub schema: String,
    pub size: u32,
    pub sha: String,
}

impl<'a, 'b> PartialEq<&'b Entry<'a>> for IntoEntry {
    // check only title then version
    fn eq(&self, other: &&Entry<'a>) -> bool {
        Some(self.title.as_str()).eq(&other.title())
            && Some(self.version.0) == other.version().map(|v| v.major())
            && Some(self.version.1) == other.version().map(|v| v.minor())
            && Some(self.version.2) == other.version().map(|v| v.patch())
    }
}

impl<'a, 'b> PartialOrd<&'b Entry<'a>> for IntoEntry {
    // check title then version
    fn partial_cmp(&self, other: &&Entry<'a>) -> Option<std::cmp::Ordering> {
        Some(self.title.as_str()).partial_cmp(&other.title())
    }
}

impl Partitionable for IntoEntry {
    fn key(&self) -> &str {
        &self.title
    }
}

impl<'a> IntoBuffer<'a, Entry<'a>> for IntoEntry {
    fn into_buffer(self, builder: &mut FlatBufferBuilder<'a>) -> WIPOffset<Entry<'a>> {
        let title = Some(builder.create_string(&self.title));
        let description = Some(builder.create_string(&self.description));
        let schema = Some(builder.create_string(&self.schema));
        let sha = Some(builder.create_string(&self.sha));

        builder.start_vector::<WIPOffset<&str>>(self.capabilities.len());
        for cap in &self.capabilities {
            let str = builder.create_string(cap);
            builder.push(str);
        }
        let capabilities = Some(builder.end_vector(self.capabilities.len()));

        Entry::create(
            builder,
            &EntryArgs {
                title,
                version: Some(&Version::new(
                    self.version.0,
                    self.version.1,
                    self.version.2,
                )),
                description,
                schema,
                capabilities,
                size_: self.size,
                sha,
            },
        )
    }
}

#[cfg(test)]
mod test {
    use tracing::Instrument;

    use crate::partition::{IntoEntry, Partition};
    use crate::proto::litehouse::Entry;

    #[tokio::test]
    async fn count() {
        let tmp = temp_dir::TempDir::new().unwrap();
        let path = tmp.path().join("test.bin");
        let partition = Partition::<Entry>::new(&path).await;
        assert_eq!(partition.count(), 0);
        assert_eq!(partition.count(), 0);
    }

    #[tokio::test]
    async fn insert() {
        let tmp = temp_dir::TempDir::new().unwrap();
        let path = tmp.path().join("test.bin");

        let mut partition = Partition::<Entry>::new(&path).await;
        for (idx, name) in (1..5).enumerate() {
            assert_eq!(partition.count(), idx);
            let _ = partition
                .insert(IntoEntry {
                    title: name.to_string(),
                    version: (1, 2, 3),
                    size: 1234,
                    description: "test".to_string(),
                    sha: "abcd".to_string(),
                    schema: "{}".to_string(),
                    capabilities: vec![],
                })
                .await;
        }
    }

    #[tokio::test]
    #[ignore] // this is expensive
    async fn insert_many() {
        let tmp = temp_dir::TempDir::new().unwrap();
        let path = tmp.path().join("test.bin");

        tmp.leak();
        println!("{path:?}");

        let mut partition = Partition::<Entry>::new(&path).await;
        for (idx, name) in (1..10000).enumerate() {
            assert_eq!(partition.count(), idx);
            tracing::info!("GOT LENGTH {idx}");

            let _ = partition
                .insert(IntoEntry {
                    title: name.to_string(),
                    version: (1, 2, 3),
                    size: 1234,
                    description: "test".to_string(),
                    sha: "abcd".to_string(),
                    schema: "{}".to_string(),
                    capabilities: vec![],
                })
                .await
                .instrument(tracing::info_span!("inserting"));
            tracing::info!("INSERTED");
        }
    }
}
