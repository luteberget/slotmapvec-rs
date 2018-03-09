# Slot map: array storage with persistent indices

`Vec<T>`-like collection with stable indices.
The underlying array's indices are re-used by incementing a 
versioning tag in the index type.

The `SlotMapIndex` type consists of a `u32` for storing the
index into the underlying array, and a `u32` for storing
the version. Deleting and inserting more times than the maximum 
value of `u32` will cause overflow and index conflict bugs.
