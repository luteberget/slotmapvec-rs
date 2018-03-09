//! Slot map: array storage with persistent indices
//!
//! `Vec<T>`-like collection with stable indices.
//! The underlying array's indices are re-used by incementing a
//! versioning tag in the index type.
//!
//! The `SlotMapIndex` type consists of a `u32` for storing the
//! index into the underlying array, and a `u32` for storing
//! the version. Deleting and inserting more times than the maximum
//! value of `u32` will cause overflow and index conflict bugs.
//!
//! # Examples
//!
//! ```
//! # use slotmapvec::*;
//! let mut map = SlotMapVec::new();
//!
//! map.insert(123213);
//! let idx = map.insert(34234);
//! map.insert(654654);
//!
//! map.remove(idx);
//! let idx2 = map.insert(999);
//!
//! assert_eq!(map.get(idx), None);
//! assert_eq!(map.get(idx2), Some(&999));
//! ```

#![crate_name = "slotmapvec"]

use std::mem;
use std::ops;
use std::iter::IntoIterator;


/// Slot map: array storage with persistent indices
///
/// See [module documentation] for more details.
///
/// [module documentation]: index.html
#[derive(Clone,Debug)]
pub struct SlotMapVec<T> {
    // Backing storage
    entries: Vec<Entry<T>>,

    // Offset of the next free slot. If there are no
    // free slots, this value is equal to the number of elements.
    next_free: usize,

    // Number of elements stored in the map.
    // Number of free slots can be calculated by taking entries.len() - len.
    len: usize,
}

/// An index into a `SlotMapVec`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SlotMapIndex {
    slot: u32,
    version: u32,
}

impl<T> Default for SlotMapVec<T> {
    fn default() -> Self {
        SlotMapVec::new()
    }
}

// TODO: switch to this entry type to save one word.
//  #[derive(Clone,Debug)]
//   pub enum Entry<T> {
//       Free(u32, u32),
//       Occupied(u32, T),
//   }

#[derive(Clone,Debug)]
struct Entry<T> {
    version: u32,
    content: Occupation<T>,
}

#[derive(Clone,Debug)]
enum Occupation<T> {
    Free(usize),
    Occupied(T),
}

/// An iterator over the values stored in a `SlotMapVec`.
pub struct Iter<'a, T: 'a> {
    entries: std::slice::Iter<'a, Entry<T>>,
    curr: usize,
}

/// A mutable iterator over the values stored in a `SlotMapVec`.
pub struct IterMut<'a, T: 'a> {
    entries: std::slice::IterMut<'a, Entry<T>>,
    curr: usize,
}

impl<T> SlotMapVec<T> {
    /// Construct a new, empty `SlotMapVec`.
    ///
    /// The function does not allocate.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slotmapvec::*;
    /// let slotmap :SlotMapVec<i32> = SlotMapVec::new();
    /// ```
    pub fn new() -> SlotMapVec<T> {
        SlotMapVec::with_capacity(0)
    }

    /// Construct a new `SlotMapVec` with the specified capacity.
    pub fn with_capacity(capacity: usize) -> SlotMapVec<T> {
        SlotMapVec {
            entries: Vec::with_capacity(capacity),
            len: 0,
            next_free: 0,
        }
    }

    /// Returns the number of values the map can store without reallocating.
    pub fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    // pub fn reserve(&mut self, additional: usize) {
    //     if self.capacity() - self.len + self.free_list.len() >= additional {
    //         return;
    //     }
    //     let need = self.len() + additional;
    //     self.entries.reserve(need);
    // }
    // pub fn clear(&mut self) {
    //    self.entries.clear();
    //    self.len = 0;
    //    self.
    //

    /// Returns the number of stored values.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if no values are stored in the map.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return an iterator over all elements of the map along with
    /// their index.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slotmapvec::*;
    /// let mut map = SlotMapVec::new();
    ///
    /// for i in 0..3 {
    ///   map.insert(i);
    /// }
    ///
    /// let mut i = 0;
    /// for (idx,val) in map.iter() {
    ///   assert_eq!(*val, i);
    ///   i += 1;
    /// }
    /// ```
    pub fn iter(&self) -> Iter<T> {
        Iter {
            entries: self.entries.iter(),
            curr: 0,
        }
    }

    /// Return an iterator over mutable references to all elements
    /// of the map along with their index.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slotmapvec::*;
    /// let mut map = SlotMapVec::new();
    ///
    /// for i in 0..3 {
    ///   map.insert(i);
    /// }
    ///
    /// let mut i = 0;
    /// for (idx,val) in map.iter_mut() {
    ///   *val *= 2;
    ///   assert_eq!(*val, i);
    ///   i += 2;
    /// }
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            entries: self.entries.iter_mut(),
            curr: 0,
        }
    }

    /// Returns a reference to the value associated with the given key.
    ///
    /// If the given key is not associated with a values, then `None` is
    /// returned.
    pub fn get(&self, key: SlotMapIndex) -> Option<&T> {
        match self.entries.get(key.slot as usize) {
            Some(&Entry { ref version, content: Occupation::Occupied(ref obj) }) => {
                if *version == key.version {
                    Some(obj)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Returns a mutable reference to the value associated with the given key.
    ///
    /// If the given key is not associated with a values, then `None` is
    /// returned.
    pub fn get_mut(&mut self, key: SlotMapIndex) -> Option<&mut T> {
        match self.entries.get_mut(key.slot as usize) {
            Some(&mut Entry { ref version, content: Occupation::Occupied(ref mut obj) }) => {
                if *version == key.version {
                    Some(obj)
                } else {
                    None
                }
            }
            _ => None,
        }
    }


    /// Insert a value into the map, returning the index to the value.
    ///
    /// The returned index will always refer uniquely to the inserted value
    /// (though it may be mutated), even after objects have been deleted
    /// and inserted into the same storage slot.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slotmapvec::*;
    /// let mut map = SlotMapVec::new();
    /// let key = map.insert("hello");
    /// assert_eq!(map[key], "hello");
    /// ```
    pub fn insert(&mut self, val: T) -> SlotMapIndex {
        if self.next_free == self.entries.len() {
            let slot = self.next_free;
            self.entries.push(Entry {
                version: 0,
                content: Occupation::Occupied(val),
            });
            self.next_free += 1;
            self.len += 1;
            SlotMapIndex {
                slot: slot as u32,
                version: 0,
            }
        } else {
            let slot = self.next_free;
            let version = self.entries[slot].version + 1;
            let prev = mem::replace(&mut self.entries[slot],
                                    Entry {
                                        version: version,
                                        content: Occupation::Occupied(val),
                                    });
            match prev {
                Entry { content: Occupation::Free(next), .. } => {
                    self.next_free = next;
                }
                _ => panic!("inconsistent internal state in SlotMapVec"),
            }
            self.len += 1;
            SlotMapIndex {
                slot: slot as u32,
                version: version,
            }
        }
    }

    /// Removes and returs the value associated with the given key.
    ///
    /// The key is never reused in this map, except if the underlying
    /// storage type overflows.
    pub fn remove(&mut self, key: SlotMapIndex) -> Option<T> {
        match self.entries.get_mut(key.slot as usize) {
            Some(entry) => {
                if entry.version != key.version {
                    None
                } else if let Occupation::Free(_) = entry.content {
                    None
                } else {
                    let prev = mem::replace(&mut entry.content, Occupation::Free(self.next_free));
                    self.next_free = key.slot as usize;
                    self.len -= 1;
                    match prev {
                        Occupation::Occupied(o) => Some(o),
                        _ => unreachable!(),
                    }
                }
            }
            _ => None,
        }
    }

    /// Return `true` if a value is associated with the given key.
    pub fn contains(&self, key: SlotMapIndex) -> bool {
        match self.entries.get(key.slot as usize) {
            Some(&Entry { ref version, content: Occupation::Occupied(_) }) => {
                *version == key.version
            }
            _ => false,
        }
    }
}

impl<T> ops::Index<SlotMapIndex> for SlotMapVec<T> {
    type Output = T;
    fn index(&self, key: SlotMapIndex) -> &T {
        match self.entries[key.slot as usize] {
            Entry { ref version, content: Occupation::Occupied(ref obj) } => {
                if *version != key.version {
                    panic!("invalid key")
                } else {
                    obj
                }
            }
            _ => panic!("invalid key"),
        }
    }
}

impl<T> ops::IndexMut<SlotMapIndex> for SlotMapVec<T> {
    fn index_mut(&mut self, key: SlotMapIndex) -> &mut T {
        match self.entries[key.slot as usize] {
            Entry { ref version, content: Occupation::Occupied(ref mut obj) } => {
                if *version != key.version {
                    panic!("invalid key")
                } else {
                    obj
                }
            }
            _ => panic!("invalid key"),
        }
    }
}


impl<'a, T> IntoIterator for &'a SlotMapVec<T> {
    type Item = (SlotMapIndex, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut SlotMapVec<T> {
    type Item = (SlotMapIndex, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (SlotMapIndex, &'a T);

    fn next(&mut self) -> Option<(SlotMapIndex, &'a T)> {
        while let Some(entry) = self.entries.next() {
            let key = SlotMapIndex {
                slot: self.curr as u32,
                version: entry.version,
            };
            self.curr += 1;

            if let Occupation::Occupied(ref value) = entry.content {
                return Some((key, value));
            }
        }

        None
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (SlotMapIndex, &'a mut T);

    fn next(&mut self) -> Option<(SlotMapIndex, &'a mut T)> {
        while let Some(entry) = self.entries.next() {
            let key = SlotMapIndex {
                slot: self.curr as u32,
                version: entry.version,
            };
            self.curr += 1;

            if let Occupation::Occupied(ref mut value) = entry.content {
                return Some((key, value));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        println!("Printing debug test:");
        let mut x = SlotMapVec::new();
        println!("X: {:?}", x);
        x.insert(123213);
        println!("X: {:?}", x);
        let mid = x.insert(34234);
        println!("X: {:?}", x);
        x.insert(654654);
        println!("X: {:?}", x);
        println!("get mid {:?}: {:?}", mid, x.get(mid));
        x.remove(mid);
        println!("X: {:?}", x);
        let ni = x.insert(999);
        println!("X @ {:?}: {:?}", ni, x);
        println!("Printing debug test done.");
    }

    #[test]
    fn size_it() {
        let mut x = SlotMapVec::new();
        let _slot = x.insert(123213);
        let slotsize = std::mem::size_of::<SlotMapIndex>();
        println!("sizeof(SlotMapIndex) == {}", slotsize);

        println!("sizeof(SlotMap<String>) == {}",
                 std::mem::size_of::<SlotMapVec<String>>());
        println!("sizeof(Entry<String>) == {}",
                 std::mem::size_of::<Entry<String>>());
        println!("sizeof(Entry<u64>) == {}",
                 std::mem::size_of::<Entry<u64>>());
        println!("sizeof(Entry<Box<u64>>) == {}",
                 std::mem::size_of::<Entry<Box<u64>>>());
    }

    #[test]
    fn iterator() {
        let mut x = SlotMapVec::new();
        x.insert(9.0);
        x.insert(7.0);
        x.insert(5.0);
        let three = x.insert(3.0);
        x.insert(-3.0);
        let low = x.insert(-8.0);
        x.insert(-6.0);


        for (_, v) in x.iter_mut() {
            *v += 1.0;
        }
        x.remove(three);
        x.insert(3.5);
        x.insert(3.0);
        x.remove(low);
        for v in x.iter() {
            println!("val: {:?}", v);
        }
    }
}
