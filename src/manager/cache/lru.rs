//! A generic lossy LRU cache

use fnv::FnvHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use util::*;

#[derive(Debug, PartialEq, Clone)]
pub struct ApplyCacheStats {
    pub lookup_count: usize,
    pub miss_count: usize,
    pub conflict_count: usize,
    /// percentage of slots being utilized
    pub utilization: f64,
}

impl ApplyCacheStats {
    pub fn new() -> ApplyCacheStats {
        ApplyCacheStats {
            miss_count: 0,
            lookup_count: 0,
            conflict_count: 0,
            utilization: 0.0,
        }
    }
}

#[inline]
fn mask(p: usize) -> usize {
    (1 << p) - 1
}

#[inline]
fn wrap_add(v: usize, p: usize) -> usize {
    (v + 1) & mask(p)
}

/// cap `v` at 2^`p`
#[inline]
fn pow_cap(v: usize, p: usize) -> usize {
    v & mask(p)
}

/// Data structure stored in the subtables
#[derive(Debug, Hash, Clone, Eq, PartialEq)]
struct Element<K, V>
where
    K: Hash + Clone + Eq + PartialEq + Debug,
    V: Eq + PartialEq + Clone,
{
    key: K,
    val: V,
}

impl<K, V> Element<K, V>
where
    K: Hash + Clone + Eq + PartialEq + Debug,
    V: Eq + PartialEq + Clone,
{
    fn new(key: K, val: V) -> Element<K, V> {
        Element { key: key, val: val }
    }
}

/// Each variable has an associated sub-table
pub struct Lru<K, V>
where
    K: Hash + Clone + Eq + PartialEq + Debug,
    V: Eq + PartialEq + Clone,
{
    tbl: Vec<Option<Element<K, V>>>,
    len: usize,
    cap: usize, // a particular power of 2
    stat: ApplyCacheStats,
}

impl<K, V> Lru<K, V>
where
    K: Hash + Clone + Eq + PartialEq + Debug,
    V: Eq + PartialEq + Clone,
{
    /// create a new bdd cache with capacity `cap`, given as a power of 2
    pub fn new(cap: usize) -> Lru<K, V> {
        let v: Vec<Option<Element<K, V>>> = zero_vec(1 << cap);
        Lru {
            tbl: v,
            len: 0,
            cap: cap,
            stat: ApplyCacheStats::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    /// the capacity for this table (as a power of 2)
    pub fn cap(&self) -> usize {
        self.cap
    }

    pub fn insert(&mut self, key: K, val: V) -> () {
        let mut hasher: FnvHasher = Default::default();
        key.hash(&mut hasher);
        let hash_v = hasher.finish();
        let pos = pow_cap(hash_v as usize, self.cap);
        let e = Element::new(key, val);
        if self.tbl[pos].is_some() {
            self.stat.conflict_count += 1;
        }
        self.tbl[pos] = Some(e);
    }

    pub fn get(&mut self, key: K) -> Option<V> {
        self.stat.lookup_count += 1;
        let mut hasher: FnvHasher = Default::default();
        key.hash(&mut hasher);
        let hash_v = hasher.finish();
        let pos = pow_cap(hash_v as usize, self.cap);
        let v = &self.tbl[pos];
        match v {
            Some(ref v) if v.key == key => return Some(v.val.clone()),
            _ => {
                self.stat.miss_count += 1;
                return None;
            }
        }
        // if v.is_none() {
        //     self.stat.miss_count += 1;
        //     return None;
        // }

        // if v.key == key {
        //     return Some(v.val);
        // }

        // self.stat.miss_count += 1;
        // return None;
    }

    /// grow the hashtable to accomodate more elements
    fn grow(&mut self) -> () {
        let new_sz = self.cap + 1;
        let new_v = zero_vec(1 << new_sz);
        let mut new_tbl = Lru {
            tbl: new_v,
            len: 0,
            cap: new_sz,
            stat: ApplyCacheStats::new(),
        };

        for i in self.tbl.iter() {
            if i.is_some() {
                let i = i.clone().unwrap();
                new_tbl.insert(i.key.clone(), i.val.clone());
            }
        }

        // copy new_tbl over the current table
        self.tbl = new_tbl.tbl;
        self.cap = new_tbl.cap;
        self.len = new_tbl.len;
        // don't update the stats; we want to keep those
    }

    pub fn get_stats(&self) -> ApplyCacheStats {
        // compute utilization
        let mut c = 0;
        for i in self.tbl.iter() {
            if i.is_some() {
                c += 1;
            }
        }
        let mut r = self.stat.clone();
        r.utilization = (c as f64) / (self.tbl.len() as f64);
        r
    }
}

#[test]
fn test_cache() {
    let mut c: Lru<u64, u64> = Lru::new(14);
    for i in 0..10000 {
        c.insert(i, i);
    }
    // for i in 0..10000 {
    //     if c.get(i).is_none() {
    //         panic!("could not find {}", i);
    //     }
    //     assert_eq!(c.get(i).unwrap(), i)
    // }
}
