//! Apply cache for BDD operations
use manager::cache::lru::*;
use repr::bdd::*;

const INITIAL_CAPACITY: usize = 20; // given as a power of two

/// An Ite structure, assumed to be in standard form.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub struct Ite {
    pub f: BddPtr,
    pub g: BddPtr,
    pub h: BddPtr,
}

impl Ite {
    /// Returns a new Ite in standard form and a Bool indicating whether to complement the Ite
    pub fn new(f: BddPtr, g: BddPtr, h: BddPtr) -> (Ite, bool) {
        // standardize the ite
        // See pgs. 115-117 of "Algorithms and Data Structures in VLSI Design"
        // first, introduce constants if possible
        // let (f, g, h) = match (f, g, h) {
        //     (f, g, h) if f == h => (f, g, BddPtr::false_node()),
        //     (f, g, h) if f == h.neg() => (f, g, BddPtr::true_node()),
        //     (f, g, h) if f == g.neg() => (f, BddPtr::false_node(), h),
        //     _ => (f, g, h),
        // };

        // now, standardize for negation: ensure f and g are non-negated
        // follow the table on p.g 116
        // let (f, g, h, compl) = match (f, g, h) {
        //     (f, g, h) if f.is_compl() && !h.is_compl() => (f.neg(), h, g, false),
        //     (f, g, h) if !f.is_compl() && g.is_compl() => (f, g.neg(), h.neg(), true),
        //     (f, g, h) if f.is_compl() && h.is_compl() => (f.neg(), h.neg(), g.neg(), true),
        //     _ => (f, g, h, false),
        // };
        // assert!(!f.is_compl() && !g.is_compl());
        // (Ite { f, g, h }, compl)
        (Ite { f, g, h }, false)
    }
}

/// The top-level data structure that caches applications
pub struct BddApplyTable {
    /// a vector of applications, indexed by the top label of the first pointer.
    table: Lru<Ite, BddPtr>,
}

impl BddApplyTable {
    pub fn new() -> BddApplyTable {
        let tbl = BddApplyTable {
            table: Lru::new(INITIAL_CAPACITY),
        };
        tbl
    }

    /// Insert an ite (f, g, h) into the apply table
    pub fn insert(&mut self, f: BddPtr, g: BddPtr, h: BddPtr, res: BddPtr) -> () {
        // convert the ITE into a canonical form
        let (ite, compl) = Ite::new(f, g, h);
        self.table.insert(ite, if compl { res.neg() } else { res });
        // println!("Inserted Ite({:?}, {:?}, {:?}, standardized {:?}", f, g, h, ite);
    }

    pub fn get(&mut self, f: BddPtr, g: BddPtr, h: BddPtr) -> Option<BddPtr> {
        let (ite, compl) = Ite::new(f, g, h);
        // println!("Looking up Ite({:?}, {:?}, {:?}, standardized {:?}\n", f, g, h, ite);
        let r = self.table.get(ite);
        if compl {
            r.map(|v| v.neg())
        } else {
            r
        }
    }

    pub fn get_stats(&self) -> ApplyCacheStats {
        self.table.get_stats()
    }

    /// Push a new application table to the back of the list
    pub fn new_last(&mut self) -> () {}
}



#[cfg(test)]
mod test_bdd_apply_table {
    use BddPtr;
  quickcheck! {
      fn insert_eq(f: BddPtr, g: BddPtr, h: BddPtr, r: BddPtr) -> bool {
          let mut tbl = super::BddApplyTable::new();
          tbl.insert(f, g, h, r);
          let lookup = tbl.get(f, g, h);
          r == lookup.unwrap()
      }
  }

  quickcheck! {
      fn insert_eq_neg(f: BddPtr, g: BddPtr, r: BddPtr) -> bool {
          let mut tbl = super::BddApplyTable::new();
          tbl.insert(f, BddPtr::true_node(), g, r);
          let lookup = tbl.get(f.neg(), g.neg(), BddPtr::false_node());
          if lookup.is_some() {
            r.neg() == lookup.unwrap()
          } else {
              true
          }
      }
  }
}

