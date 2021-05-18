
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
    /// let iter = vec![0,1,2,3,4,5, 6].into_iter();
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
}

impl<T, I: Iterator<Item = T>, const N: usize> Drop for Chunks<T, I, N> {
    fn drop(&mut self) {
        for x in 0..self.needs_dropping {
            let _ = unsafe { self.buffer[x].as_ptr().read() };
        }
    }
}

