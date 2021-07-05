//! Import the trait and use it:
//! ```
//! use chunk_iter::ChunkIter;
//!
//! let iter = vec![0,1,2,3,4,5, 6].into_iter();
//! let mut chunks = iter.chunks::<3>();
//! assert_eq!(chunks.next(), Some([0,1,2]));
//! assert_eq!(chunks.next(), Some([3,4,5]));
//! assert_eq!(chunks.next(), None);
//! ```
use std::mem::MaybeUninit;
/// ChunkIter trait: `use` this to use the `chunks` impl.
pub trait ChunkIter<T, I: Iterator<Item = T>> {
    /// Make chunks:
    /// ```
    /// use chunk_iter::ChunkIter;
    ///
    /// let iter = vec![0, 1, 2, 3, 4, 5, 6].into_iter();
    /// let mut chunks = iter.chunks::<3>();
    /// assert_eq!(chunks.next(), Some([0,1,2]));
    /// assert_eq!(chunks.next(), Some([3,4,5]));
    /// assert_eq!(chunks.next(), None);
    /// ```
    fn chunks<const N: usize>(self) -> Chunks<T, I, N>;
}

impl<T, I> ChunkIter<T, I> for I
where
    I: Iterator<Item = T>,
{
    fn chunks<const N: usize>(self) -> Chunks<T, I, N> {
        Chunks {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            iterator: self,
            needs_dropping: 0,
        }
    }
}
/// Chunk iterator, return value of `.chunks()`
pub struct Chunks<T, I: Iterator<Item = T>, const N: usize> {
    buffer: [MaybeUninit<T>; N],
    iterator: I,
    needs_dropping: usize,
}

impl<T, I: Iterator<Item = T>, const N: usize> Iterator for Chunks<T, I, N> {
    type Item = [T; N];

    fn next(&mut self) -> Option<Self::Item> {
        for x in &mut self.buffer {
            *x = MaybeUninit::new(self.iterator.next()?);
            self.needs_dropping += 1;
        }
        self.needs_dropping = 0;
        unsafe { Some(std::mem::transmute_copy(&self.buffer)) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iterator.size_hint();
        (lower / N, upper.map(|x| x / N))
    }
}

impl<T, I: Iterator<Item = T>, const N: usize> Drop for Chunks<T, I, N> {
    fn drop(&mut self) {
        for x in 0..self.needs_dropping {
            let _ = unsafe { self.buffer[x].as_ptr().read() };
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ChunkIter;
    use testdrop::TestDrop;

    #[test]
    fn basic_test() {
        let iter = vec![0, 1, 2, 3, 4, 5, 6, 7].into_iter();
        let mut chunks = iter.chunks::<3>();
        assert_eq!(chunks.next(), Some([0, 1, 2]));
        assert_eq!(chunks.next(), Some([3, 4, 5]));
        assert_eq!(chunks.next(), None);
    }

    #[test]
    fn drop_test() {
        let test_drop = TestDrop::new();
        let chunks = (0..10)
            .map(|_| test_drop.new_item().1)
            .collect::<Vec<_>>()
            .into_iter()
            .chunks::<3>();

        drop(chunks);

        assert_eq!(10, test_drop.num_tracked_items());
        assert_eq!(10, test_drop.num_dropped_items());
    }

    #[test]
    fn size_hint_test() {
        let iter = vec![0, 1, 2, 3, 4, 5, 6, 7].into_iter().chunks::<3>();

        assert_eq!(iter.size_hint(), (2, Some(2)))
    }
}
