# Slot map: array storage with persistent indices

`Vec<T>`-like collection with stable indices.
The underlying array's indices are re-used by incrementing a 
versioning tag in the index type.

The `SlotMapIndex` type consists of a `u32` for storing the
index into the underlying array, and a `u32` for storing
the version. Deleting and inserting more times than the maximum 
value of `u32` will cause overflow and index conflict bugs.

## Example

```rust
# use slotmapvec::*;
let mut map = SlotMapVec::new();

map.insert(123213);
let idx = map.insert(34234);
map.insert(654654);

map.remove(idx);
let idx2 = map.insert(999);
assert_eq!(map.get(idx), None);
assert_eq!(map.get(idx2), Some(&999));
```
