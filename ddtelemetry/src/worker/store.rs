// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2021-Present Datadog, Inc.

use std::{collections::VecDeque, hash::Hash};

mod queuehasmpap {
    use hashbrown::{hash_map::DefaultHashBuilder, raw::RawTable};
    use std::{
        collections::VecDeque,
        hash::{BuildHasher, Hash, Hasher},
    };

    pub struct QueueHashMap<K, V> {
        table: RawTable<usize>,
        hash_buidler: DefaultHashBuilder,
        items: VecDeque<(K, V)>,
        popped: usize,
    }

    impl<K, V> QueueHashMap<K, V>
    where
        K: PartialEq + Eq + Hash,
    {
        pub fn iter(&self) -> impl Iterator<Item = &(K, V)> {
            self.items.iter()
        }

        pub fn iter_idx(&self) -> impl Iterator<Item = usize> {
            self.popped..(self.popped + self.items.len())
        }

        pub fn len(&self) -> usize {
            self.items.len()
        }

        pub fn is_empty(&self) -> bool {
            self.items.is_empty()
        }

        pub fn pop_front(&mut self) -> Option<(K, V)> {
            let (k, v) = self.items.pop_front()?;
            let hash = make_hash(&self.hash_buidler, &k);
            let found = self.table.remove_entry(hash, |other| *other == self.popped);
            debug_assert!(found.is_some());
            debug_assert!(self.items.len() == self.table.len());
            self.popped += 1;
            Some((k, v))
        }

        pub fn get(&self, k: &K) -> Option<&V> {
            let hash = make_hash(&self.hash_buidler, k);
            let idx = self
                .table
                .get(hash, |other| &self.items[other - self.popped].0 == k)?;
            Some(&self.items[idx - self.popped].1)
        }

        pub fn get_idx(&self, idx: usize) -> Option<&(K, V)> {
            self.items.get(idx - self.popped)
        }

        pub fn get_mut_or_insert(&mut self, key: K, default: V) -> &mut V {
            let hash = make_hash(&self.hash_buidler, &key);
            if let Some(&idx) = self
                .table
                .get(hash, |other| self.items[other - self.popped].0 == key)
            {
                return &mut self.items[idx - self.popped].1;
            }
            self.insert_nocheck(hash, key, default);
            &mut self.items.back_mut().unwrap().1
        }

        pub fn insert(&mut self, key: K, value: V) -> (usize, bool) {
            let hash = make_hash(&self.hash_buidler, &key);
            if let Some(&idx) = self
                .table
                .get(hash, |other| self.items[other - self.popped].0 == key)
            {
                self.items[idx - self.popped].1 = value;
                (idx, false)
            } else {
                (self.insert_nocheck(hash, key, value), true)
            }
        }

        /// # Safety
        ///
        /// This function inserts a new item in the store unconditionnaly
        /// If the item already exists, it's drop implementation will not be called, and memory might leak
        ///
        /// The hash needs to be precomputed too
        fn insert_nocheck(&mut self, hash: u64, key: K, value: V) -> usize {
            let item_index = self.items.len() + self.popped;

            // Separate set and items since set is mutably borrowed, while items is unmutably
            let Self {
                table,
                items,
                popped,
                hash_buidler,
                ..
            } = self;
            table.insert(hash, item_index, |i| {
                make_hash(hash_buidler, &items[i - *popped].0)
            });
            self.items.push_back((key, value));
            item_index
        }
    }

    impl<K, V> Default for QueueHashMap<K, V> {
        fn default() -> Self {
            Self {
                table: RawTable::new(),
                hash_buidler: DefaultHashBuilder::new(),
                items: VecDeque::new(),
                popped: 0,
            }
        }
    }

    fn make_hash<T: Hash>(h: &DefaultHashBuilder, i: &T) -> u64 {
        let mut hasher = h.build_hasher();
        i.hash(&mut hasher);
        hasher.finish()
    }
}

pub use queuehasmpap::QueueHashMap;

#[derive(Default)]
/// Stores telemetry data item, like dependencies and integrations
///
/// * Bounds the length of the collection it uses to prevent memory leaks
/// * Tries to keep a list of items that it has seen (within max number of items)
/// * Tries to keep a list of items that haven't been sent to datadog yet
/// * Deduplicates items, to make sure we don't send the item twice
pub struct Store<T> {
    // unflushed and set contain indices into
    unflushed: VecDeque<usize>,
    items: QueueHashMap<T, ()>,
    max_items: usize,
}

impl<T> Store<T>
where
    T: PartialEq + Eq + Hash,
{
    pub fn new(max_items: usize) -> Self {
        Self {
            unflushed: VecDeque::new(),
            items: QueueHashMap::default(),
            max_items,
        }
    }

    pub fn insert(&mut self, item: T) {
        if self.items.get(&item).is_some() {
            return;
        }
        if self.items.len() == self.max_items {
            self.items.pop_front();
        }
        let (idx, _) = self.items.insert(item, ());
        if self.unflushed.len() == self.max_items {
            self.unflushed.pop_front();
        }
        self.unflushed.push_back(idx);
    }

    pub fn unflush_stored(&mut self) {
        for i in self.items.iter_idx() {
            self.unflushed.push_front(i);
        }
    }

    pub fn removed_flushed(&mut self, count: usize) {
        for _ in 0..count {
            self.unflushed.pop_front();
        }
    }

    pub fn flush_not_empty(&self) -> bool {
        !self.unflushed.is_empty()
    }

    pub fn unflushed(&self) -> impl Iterator<Item = &T> {
        self.unflushed
            .iter()
            .flat_map(|i| Some(&self.items.get_idx(*i)?.0))
    }
}

impl<T> Extend<T> for Store<T>
where
    T: PartialEq + Eq + Hash,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for i in iter {
            self.insert(i)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_insert() {
        let mut store = Store::new(10);
        store.insert("hello");
        store.insert("world");
        store.insert("world");

        assert_eq!(store.unflushed.len(), 2);
        assert_eq!(store.items.len(), 2);
        assert_eq!(store.unflushed().collect::<Vec<_>>(), &[&"hello", &"world"]);

        store.removed_flushed(1);
        assert_eq!(store.items.len(), 2);
        assert_eq!(store.unflushed().collect::<Vec<_>>(), &[&"world"]);

        store.removed_flushed(1);
        assert_eq!(store.items.len(), 2);
        assert!(store.unflushed().next().is_none());

        store.insert("hello");
        assert!(store.unflushed().next().is_none());
    }

    #[test]
    fn test_insert_spill() {
        let mut store = Store::new(5);
        for i in 2..15 {
            store.insert(i);
        }
        assert_eq!(store.unflushed.len(), 5);
        assert_eq!(store.items.len(), 5);

        assert_eq!(
            store.unflushed().collect::<Vec<_>>(),
            &[&10, &11, &12, &13, &14]
        )
    }

    #[test]
    fn test_insert_spill_no_unflush() {
        let mut store = Store::new(5);
        for i in 2..7 {
            store.insert(i);
        }
        assert_eq!(store.unflushed.len(), 5);

        assert_eq!(store.unflushed().collect::<Vec<_>>(), &[&2, &3, &4, &5, &6]);
        store.removed_flushed(4);

        for i in 7..10 {
            store.insert(i);
        }

        assert_eq!(store.unflushed.len(), 4);
        assert_eq!(store.unflushed().collect::<Vec<_>>(), &[&6, &7, &8, &9]);
    }
}
