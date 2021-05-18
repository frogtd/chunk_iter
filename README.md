# chunk_iter
It makes any iterable into chunks, using const generics.

# Usage
```rust
use chunk_iter::ChunkIter;

for x in iter.chunks::<3>() {
    println!("{:?}", x); // x is a size 3 array of what iter contains
}
```