use std::{marker::PhantomData, ops::Range};

use flatbuffers::{Follow, Verifiable};
use futures::StreamExt;
use tokio::sync::RwLock;

use crate::{io::PartitionIOScheme, partition::Partition};

/// A partitioning scheme for the registry. Accepts lowercase a-z, hyphens, and underscores.
pub trait PartitioningScheme<'a, T>
where
    T: Follow<'a> + Verifiable + 'a,
{
    type IOScheme: PartitionIOScheme<'a, T>;

    /// The number of partitions under the scheme
    fn count(&self) -> usize;

    /// The partition that a particular key can be found in
    fn partition(&self, prefix: &str) -> Result<usize, ()>;

    /// The partitions that a particular range exists within
    ///
    /// This is inclusive of the end so that abc..def will also
    /// return entries for def.
    fn range(&self, prefix_start: &str, prefix_end: &str) -> Result<Range<usize>, ()>;

    /// Get the underlying io scheme
    fn io_scheme(&self) -> &Self::IOScheme;

    /// Get the partition that a key belongs to
    ///
    /// Note: if the key contains invalid characters, this function will error.
    fn get(
        &'a self,
        prefix: &str,
    ) -> Result<impl futures::Future<Output = &'a RwLock<Partition<'a, T>>>, ()> {
        let id = self.partition(prefix)?;
        Ok(self.open(id))
    }

    fn open(&'a self, id: usize) -> impl futures::Future<Output = &'a RwLock<Partition<'a, T>>> {
        self.io_scheme().open(id)
    }

    fn all(&'a self) -> impl futures::Stream<Item = &'a RwLock<Partition<'a, T>>> {
        futures::stream::iter(0..self.count()).then(move |c| self.open(c))
    }
}

/// splits it evenly by alphabet
pub struct Alphabetical<
    'a,
    const LETTERS: usize,
    T: Follow<'a> + Verifiable,
    IO: PartitionIOScheme<'a, T>,
> {
    /// Partitions the space based on the starting letters.
    /// An array of length one looks at the first, two,
    /// the first two, etc.
    scheme: [Split; LETTERS],
    io: IO,
    _phantom: PhantomData<&'a T>,
}

#[derive(Copy, Clone)]
pub enum Split {
    /// Split this letter into a single space
    One = 1,
    /// Split this letter into two spaces
    Two = 2,
    /// Split this letter into seven spaces
    Seven = 7,
    /// Split this letter into fourteen spaces
    Fourteen = 14,
    /// Split this letter into twenty-six spaces
    TwentyEight = 28,
}

impl<'a, const LETTERS: usize, T, IO> Alphabetical<'a, LETTERS, T, IO>
where
    T: Follow<'a> + Verifiable,
    IO: PartitionIOScheme<'a, T>,
{
    pub fn new(scheme: [Split; LETTERS], io: IO) -> Self {
        Alphabetical {
            scheme,
            io,
            _phantom: PhantomData,
        }
    }
}

impl<'a, const LETTERS: usize, T, IO> PartitioningScheme<'a, T> for Alphabetical<'a, LETTERS, T, IO>
where
    T: Follow<'a> + Verifiable,
    IO: PartitionIOScheme<'a, T>,
{
    type IOScheme = IO;

    fn count(&self) -> usize {
        self.scheme
            .iter()
            .fold(1, |acc, split| acc * *split as usize)
    }

    fn partition(&self, prefix: &str) -> Result<usize, ()> {
        let mut number = 0;
        let mut total_buckets = self.count();
        for (split, letter) in self.scheme.iter().zip(prefix.chars()) {
            let bucket_divisor = *split as usize;
            let bucket_count = 28 / bucket_divisor;

            // map ascii characters a-z + '-' and '_' to 0-27
            let index = if letter.is_ascii_lowercase() {
                letter as usize - 'a' as usize
            } else if letter == '-' {
                26
            } else if letter == '_' {
                27
            } else {
                return Err(());
            };

            let offset = index / bucket_count;

            total_buckets /= bucket_divisor;
            number += total_buckets * offset;
        }
        Ok(number)
    }

    fn range(&self, prefix_start: &str, prefix_end: &str) -> Result<Range<usize>, ()> {
        let start = self.partition(prefix_start)?;
        let end = self.partition(prefix_end)?;
        Ok(start..(end + 1))
    }

    fn io_scheme(&self) -> &Self::IOScheme {
        &self.io
    }
}

#[cfg(test)]
mod test {
    use std::ops::Range;

    use test_case::test_case;

    use crate::{
        partition_scheme::{Alphabetical, PartitioningScheme as _, Split},
        proto::litehouse::Entry,
    };

    #[test_case("a", Ok(0))]
    #[test_case("b", Ok(0))]
    #[test_case("z", Ok(0))]
    #[test_case("!", Err(()) ; "invalid character")]
    fn one_partition(key: &str, partition: Result<usize, ()>) {
        let naming = crate::naming::NumericalPrefixed::new(".");
        let io = crate::io::s3::MMapS3IoScheme::<Entry, _>::new(naming, None);
        let scheme = Alphabetical::<1, Entry, _>::new([Split::One], io);
        assert_eq!(scheme.partition(key), partition);
    }

    #[test_case("a", Ok(0))]
    #[test_case("b", Ok(0))]
    #[test_case("n", Ok(0))]
    #[test_case("o", Ok(1))]
    #[test_case("z", Ok(1))]
    #[test_case("_", Ok(1))]
    fn two_partition(key: &str, partition: Result<usize, ()>) {
        let naming = crate::naming::NumericalPrefixed::new(".");
        let io = crate::io::s3::MMapS3IoScheme::<Entry, _>::new(naming, None);
        let scheme = Alphabetical::<1, Entry, _>::new([Split::Two], io);
        assert_eq!(scheme.partition(key), partition);
    }

    #[test_case("a", Ok(0))]
    #[test_case("b", Ok(1))]
    #[test_case("m", Ok(12))]
    #[test_case("n", Ok(13))]
    #[test_case("z", Ok(25))]
    fn twenty_eight_partition(key: &str, partition: Result<usize, ()>) {
        let naming = crate::naming::NumericalPrefixed::new(".");
        let io = crate::io::s3::MMapS3IoScheme::<Entry, _>::new(naming, None);
        let scheme = Alphabetical::<1, Entry, _>::new([Split::TwentyEight], io);
        assert_eq!(scheme.partition(key), partition);
    }

    #[tokio::test]
    #[test_case("aa", Ok(0))]
    #[test_case("ao", Ok(1))]
    #[test_case("be", Ok(2))]
    #[test_case("ma", Ok(24))]
    #[test_case("na", Ok(26))]
    #[test_case("za", Ok(50))]
    #[test_case("zz", Ok(51))]
    async fn twenty_eight_two_partition(key: &str, partition: Result<usize, ()>) {
        let naming = crate::naming::NumericalPrefixed::new(".");
        let io = crate::io::s3::MMapS3IoScheme::<Entry, _>::new(naming, None);
        let scheme = Alphabetical::<2, Entry, _>::new([Split::TwentyEight, Split::Two], io);
        assert_eq!(scheme.partition(key), partition);
    }

    #[test_case("a", "_", 0..28 ; "all")]
    #[test_case("a", "a", 0..1 ; "inclusive 1")]
    fn test_range(start: &str, end: &str, expected: Range<usize>) {
        let naming = crate::naming::NumericalPrefixed::new(".");
        let io = crate::io::s3::MMapS3IoScheme::<Entry, _>::new(naming, None);
        let scheme = Alphabetical::<1, Entry, _>::new([Split::TwentyEight], io);
        let partitions = scheme.range(start, end).unwrap();
        assert_eq!(partitions, expected);
    }
}
