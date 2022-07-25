//! Import the trait and use it:
//! ```
//! use chunk_iter::ChunkIter;
//!
//! let iter = vec![0, 1, 2, 3, 4, 5, 6].into_iter();
//! let mut chunks = iter.chunks::<3>();
//! assert_eq!(chunks.next(), Some([0, 1, 2]));
//! assert_eq!(chunks.next(), Some([3, 4, 5]));
//! assert_eq!(chunks.next(), None);
//! ```
#![no_std]
use core::{
    iter::FusedIterator,
    mem::{ManuallyDrop, MaybeUninit},
    ptr,
};
/// ChunkIter trait: `use` this to use the `chunks` impl.
pub trait ChunkIter<T, I: Iterator<Item = T>> {
    /// Make chunks:
    /// ```
    /// use chunk_iter::ChunkIter;
    ///
    /// let iter = vec![0, 1, 2, 3, 4, 5, 6].into_iter();
    /// let mut chunks = iter.chunks::<3>();
    /// assert_eq!(chunks.next(), Some([0, 1, 2]));
    /// assert_eq!(chunks.next(), Some([3, 4, 5]));
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

impl<T, I: Iterator<Item = T>, const N: usize> Chunks<T, I, N> {
    const NONE: Option<T> = None;
    /// Gets the number of currently stored things in the backing array.
    /// This is usually empty, and only will have values after the backing iterator runs out.
    /// ```
    /// use chunk_iter::ChunkIter;
    ///
    /// let mut iter = vec![0, 1, 2, 3, 4].into_iter().chunks();
    /// assert_eq!(iter.next(), Some([0, 1, 2]));
    /// assert_eq!(iter.next(), None);
    ///
    /// assert_eq!(iter.currently_stored(), &[3, 4]);
    pub fn currently_stored(&self) -> &[T] {
        // SAFETY:
        // needs_dropping is the number of elements that are stored in the buffer
        unsafe { &*(&self.buffer[..self.needs_dropping] as *const [_] as *const [T]) }
    }

    /// Convert into array of currently stored items.
    /// This will only have values when the backing iterator has run out of values.
    /// ```
    /// use chunk_iter::ChunkIter;
    ///
    /// let mut iter = vec![0, 1, 2, 3, 4].into_iter().chunks();
    /// assert_eq!(iter.next(), Some([0, 1, 2]));
    /// assert_eq!(iter.next(), None);
    ///
    /// assert_eq!(iter.into_stored(), [Some(3), Some(4), None]);
    pub fn into_stored(self) -> [Option<T>; N] {
        let mut this = ManuallyDrop::new(self);
        let mut stored = [Self::NONE; N];
        for (x, item) in stored.iter_mut().enumerate().take(this.needs_dropping) {
            *item = unsafe { Some(this.buffer[x].as_ptr().read()) };
        }
        unsafe { ptr::drop_in_place(&mut this.iterator) };
        stored
    }
}

impl<T, I: Iterator<Item = T>, const N: usize> Iterator for Chunks<T, I, N> {
    type Item = [T; N];

    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY:
        // self.needs_dropping < N at all times [1]
        // therefore this can never be out of bounds
        for x in unsafe { self.buffer.get_unchecked_mut(self.needs_dropping..) } {
            *x = MaybeUninit::new(self.iterator.next()?);
            // [1] except for here right before it sets it to zero
            self.needs_dropping += 1;
        }
        self.needs_dropping = 0;
        // SAFETY: MaybeUninit<T> has the same size, alignment, and ABI as T
        unsafe { Some(core::mem::transmute_copy(&self.buffer)) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iterator.size_hint();
        (lower / N, upper.map(|x| x / N))
    }
}
impl<T, I: Iterator<Item = T> + ExactSizeIterator, const N: usize> ExactSizeIterator
    for Chunks<T, I, N>
{
    fn len(&self) -> usize {
        self.iterator.len() / N
    }
}
impl<T, I: Iterator<Item = T> + FusedIterator, const N: usize> FusedIterator for Chunks<T, I, N> {}

impl<T, I: Iterator<Item = T>, const N: usize> Drop for Chunks<T, I, N> {
    fn drop(&mut self) {
        for x in 0..self.needs_dropping {
            // SAFETY: needs_dropping only includes values that are initialized
            unsafe { self.buffer[x].as_ptr().read() };
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::ChunkIter;
    use testdrop::TestDrop;
    extern crate alloc;

    #[test]
    fn basic_test() {
        let iter = alloc::vec![0, 1, 2, 3, 4, 5, 6, 7].into_iter();
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
            .collect::<alloc::vec::Vec<_>>()
            .into_iter()
            .chunks::<3>();

        drop(chunks);

        assert_eq!(10, test_drop.num_tracked_items());
    }

    #[test]
    fn size_hint_test() {
        let iter = alloc::vec![0, 1, 2, 3, 4, 5, 6, 7]
            .into_iter()
            .chunks::<3>();

        assert_eq!(iter.size_hint(), (2, Some(2)))
    }

    #[test]
    fn currently_stored_test() {
        let mut iter = alloc::vec![0, 1, 2, 3, 4].into_iter().chunks::<3>();
        iter.next();
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
        assert_eq!(iter.currently_stored(), &[3, 4]);
        assert_eq!(iter.into_stored(), [Some(3), Some(4), None]);
    }
}
