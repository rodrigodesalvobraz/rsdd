//! Stores BDD applications in an LRU cache.
use bdd::*;
use std::ptr;
use util::*;
use std::hash::Hasher;
use fnv;
use twox_hash::XxHash;

const LOAD_FACTOR: f64 = 0.7;
const INITIAL_CAPACITY: usize = 16392;
const GROWTH_RATE: usize = 8;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct ApplyOp(pub Op, pub BddPtr, pub BddPtr);

/// Data structure stored in the subtables
#[derive(Debug, Hash, Clone)]
struct Element {
    a: BddPtr,
    b: BddPtr,
    result: BddPtr,
    occupied: bool,
    offset: u32,
}

impl Element {
    fn new(a: BddPtr, b: BddPtr, res: BddPtr) -> Element {
        Element {
            a: a,
            b: b,
            result: res,
            occupied: true,
            offset: 0
        }
    }
}

fn elem_eq(a: &Element, b: &Element) -> bool {
    a.a == b.a && a.b == b.b
}

/// Each variable has an associated sub-table
struct SubTable {
    tbl: Vec<Element>,
    len: usize,
    cap: usize,
}

struct SubTableIter<'a> {
    tbl: &'a SubTable,
    pos: usize
}

impl<'a> Iterator for SubTableIter<'a> {
    type Item = &'a Element;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.tbl.tbl.len() {
            None
        } else {
            self.pos += 1;
            let itm = self.tbl.tbl[self.pos-1].clone();
            if itm.occupied {
                Some(&self.tbl.tbl[self.pos-1])
            } else {
                self.next()
            }
        }
    }
}

#[inline]
fn hash_pair(a: BddPtr, b: BddPtr) -> u64 {
    let mut hasher = XxHash::with_seed(1123859);
    hasher.write_u64(a.raw());
    hasher.write_u64(b.raw());
    hasher.finish()
}

impl SubTable {
    fn new(minimum_size: usize) -> SubTable {
        let tbl_sz = ((minimum_size as f64 * (1.0 + LOAD_FACTOR)) as usize).next_power_of_two();
        let v: Vec<Element> = zero_vec(tbl_sz);
        SubTable {
            tbl: v,
            len: 0,
            cap: tbl_sz,
        }
    }

    fn insert(&mut self, a: BddPtr, b: BddPtr, result: BddPtr) -> () {
        if (self.len + 1) as f64 > (self.cap as f64 * LOAD_FACTOR) {
            self.grow();
        }

        let hash_v = hash_pair(a, b);
        let mut pos = (hash_v as usize) % self.cap;
        let mut searcher = Element::new(a, b, result);
        loop {
            if self.tbl[pos].occupied {
                // first, check if they are equal.
                if elem_eq(&self.tbl[pos], &searcher) {
                    return;
                } else {
                }
                // they are not equal, see if we should swap for the closer one
                if self.tbl[pos].offset < searcher.offset {
                    // swap the searcher with the current element
                    let tmp = searcher;
                    searcher = self.tbl[pos].clone();
                    self.tbl[pos] = tmp;
                } else {
                    searcher.offset += 1;
                }
            } else {
                // found an open spot, insert
                self.tbl[pos] = searcher;
                self.len += 1;
                return;
            }
            pos = (pos + 1) % self.cap;
        }
    }

    fn iter<'a>(&'a self) -> SubTableIter<'a> {
        SubTableIter { tbl: self, pos: 0 }
    }

    fn get(&self, a: BddPtr, b: BddPtr) -> Option<BddPtr> {
        let hash_v = hash_pair(a, b);
        let mut pos = (hash_v as usize) % self.cap;
        loop {
            if self.tbl[pos].occupied {
                let itm = self.tbl[pos].clone();
                if itm.a == a && itm.b == b {
                    return Some(self.tbl[pos].result.clone());
                }
            } else {
                return None;
            }
            pos = (pos + 1) % self.cap;
        }
    }

    /// grow the hashtable to accomodate more elements
    fn grow(&mut self) -> () {
        let new_sz = self.cap * GROWTH_RATE;
        let new_v = zero_vec(new_sz);
        let mut new_tbl = SubTable {
            tbl: new_v,
            len: 0,
            cap: new_sz
        };

        for i in self.iter() {
            new_tbl.insert(i.a, i.b, i.result);
        }

        // copy new_tbl over the current table
        self.tbl = new_tbl.tbl;
        self.cap = new_tbl.cap;
        self.len = new_tbl.len;
    }

    fn avg_offset(&self) -> f64 {
        let mut offs : usize = 0;
        for i in self.iter() {
            offs += i.offset as usize;
        }
        offs as f64 / (self.len as f64)
    }
}

/// The top-level data structure which caches applications
pub struct ApplyTable {
    or_tables: Vec<SubTable>,
    and_tables: Vec<SubTable>,
}

impl ApplyTable {
    pub fn new(num_vars: usize) -> ApplyTable {
        let mut tbl = ApplyTable {
            or_tables: Vec::with_capacity(num_vars),
            and_tables: Vec::with_capacity(num_vars)
        };
        for _ in 0..num_vars {
            tbl.or_tables.push(SubTable::new(INITIAL_CAPACITY));
            tbl.and_tables.push(SubTable::new(INITIAL_CAPACITY));
        }
        tbl
    }

    /// Insert an operation into the apply table. Note that operations are
    /// normalized by first sorting the sub-BDDs such that BDD A occurs first
    /// in the ordering; this increases cache hit rate and decreases duplicate
    /// storage
    pub fn insert(&mut self, op: ApplyOp, res: BddPtr) -> () {
        let ApplyOp(op, a, b) = op;
        let tbl = a.var() as usize;
        match op {
            Op::BddAnd => self.and_tables[tbl].insert(a, b, res),
            Op::BddOr => self.or_tables[tbl].insert(a, b, res)
        }
    }

    pub fn get(&self, op: ApplyOp) -> Option<BddPtr> {
        let ApplyOp(op, a, b) = op;
        let tbl = a.var() as usize;
        match op {
            Op::BddAnd => self.and_tables[tbl].get(a, b),
            Op::BddOr => self.or_tables[tbl].get(a, b)
        }
    }
}

#[test]
fn apply_cache_simple() {
    let mut tbl = ApplyTable::new(10);
    for var in 0..10 {
        for i in 0..10000 {
            let op = ApplyOp(Op::BddAnd,
                             BddPtr::new(VarLabel::new(var), TableIndex::new(i)),
                             BddPtr::new(VarLabel::new(var+1), TableIndex::new(i)));
            let result = BddPtr::new(VarLabel::new(var), TableIndex::new(i));
            tbl.insert(op, result);
        }
    }
    for var in 0..10 {
        for i in 0..10000 {
            let op = ApplyOp(Op::BddAnd,
                             BddPtr::new(VarLabel::new(var), TableIndex::new(i)),
                             BddPtr::new(VarLabel::new(var+1), TableIndex::new(i)));
            let result = BddPtr::new(VarLabel::new(var), TableIndex::new(i));
            assert_eq!(tbl.get(op).unwrap(), result);
        }
    }
}