use std::{marker::PhantomData, ops::Range, path::PathBuf};

use flatbuffers::{FlatBufferBuilder, Follow, Verifiable, WIPOffset, SIZE_SIZEPREFIX};

use futures::Future;
use memmap2::Mmap;
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncWriteExt as _, BufReader},
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
            .open(&path)
            .await
            .unwrap();
        let data = unsafe { memmap2::Mmap::map(&file).unwrap() };
        let size = file.metadata().await.unwrap().len();
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
        Self::elements_inner(&self.access)
    }

    fn elements_inner(access: &[u8]) -> impl Iterator<Item = Range<usize>> + '_ {
        let mut offset = 0;
        std::iter::from_fn(move || {
            let Some(len) = access.get(offset..offset + SIZE_SIZEPREFIX) else {
                return None;
            };

            let len = usize::from_le_bytes(len.try_into().unwrap());
            let range = offset..offset + SIZE_SIZEPREFIX + len;
            offset = offset + SIZE_SIZEPREFIX + len;
            Some(range)
        })
    }

    /// Get all the entries in the partition. This data is sorted.
    fn entries_with_offset(&'a self) -> impl Iterator<Item = (T::Inner, Range<usize>)> {
        let elems = self.elements();
        let closure = move |range: Range<usize>| {
            let entry = &self.access[range.clone()];
            let entry = flatbuffers::root::<T>(entry).unwrap();
            (entry, range)
        };

        elems.map(closure)
    }

    /// Get all the entries in the partition. This data is sorted.
    pub fn entries(&'a self) -> impl Iterator<Item = T::Inner> {
        self.entries_with_offset().map(|(entry, _)| entry)
    }

    pub fn count(&self) -> usize {
        todo!()
    }

    /// Read all the data up to the entry we want to insert, write it, then all the remaining
    /// to a new file. Finally, rename the new file to the old file.
    pub fn insert<IB: IntoBuffer<'a, T> + for<'c> PartialOrd<&'c T::Inner> + 'a>(
        &'a mut self,
        insert: IB,
    ) -> impl Future<Output = ()> + 'a {
        async move {
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
                // tracing::debug!("got {} items", self.entries(&access).await.count());
                let lower_bound = {
                    let mut start = None;
                    for x in self.elements() {
                        let entry = &self.access[x.clone()];

                        // SAFETY: due to the way flatbuffers works, T needs a lifetime that is
                        // tied to that of the struct. This causes the borrow checker to complain
                        // about reborrows later, however we know here that entry is dropped
                        let data: &'static [u8] = unsafe { core::mem::transmute(entry) };

                        let entry = flatbuffers::root::<T>(data).unwrap();
                        if insert > &entry {
                            start = Some(x.start);
                            break;
                        }
                    }
                    start
                };

                tracing::debug!("lower bound {:?}", lower_bound);

                // write up to the lower bound to the new file
                if let Some(range) = lower_bound {
                    tracing::trace!("writing up to {range:?}");
                    cursor.write(&self.access[0..range]).await.unwrap();
                }

                // write the new entry to the new file
                let mut builder = FlatBufferBuilder::new();
                let offset = insert.into_buffer(&mut builder);
                builder.finish(offset, None);
                let bytes = builder.finished_data();
                cursor
                    .write(&(bytes.len() as u32).to_be_bytes())
                    .await
                    .unwrap();
                cursor.write(bytes).await.unwrap();

                // write the rest of the entries to the new file
                if let Some(range) = lower_bound {
                    tracing::trace!("writing from {:?}", range);
                    cursor.write(&self.access[range..]).await.unwrap();
                }
            }

            // move the file
            tracing::trace!("moving {tmp:?} to {:?}", self.path);
            tokio::fs::rename(tmp, &self.path).await.unwrap();

            // we store the file already but lets reopen it to be safe
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&self.path)
                .await
                .unwrap();

            self.size = file.metadata().await.unwrap().len();
            self.access = unsafe { Mmap::map(&file) }.unwrap();
            self.file = file;
        }
    }
}

pub trait IntoBuffer<'a, T: Follow<'a> + Verifiable> {
    fn into_buffer(self, builder: &mut FlatBufferBuilder<'a>) -> WIPOffset<T>;
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn insert() {}
}
