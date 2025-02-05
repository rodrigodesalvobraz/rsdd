use backing_store::robin_hood::*;
use backing_store::*;
use manager::rsbdd_manager::BddManager;
use manager::var_order::VarOrder;
use repr::sdd::*;
use repr::var_label::VarLabel;
use std::collections::HashMap;
use std::slice::from_raw_parts;
use util::btree::*;

const DEFAULT_RH_SZ: usize = 32768;

/// The SDD table consists of two types of node allocators: BDD
/// allocators, and SDD allocators
enum SubTable {
    BddSubTable {
        man: BddManager,
        /// convert from BDD var labels to SDD var labels
        conv: HashMap<VarLabel, VarLabel>,
    },
    SddSubTable {
        tbl: BackedRobinHoodTable<Vec<(SddPtr, SddPtr)>>,
    },
}

/// Handles memory management for the SDD manager
pub struct SddTable {
    /// mapping between sdd and bdd variable labels
    pub sdd_to_bdd: HashMap<VarLabel, VarLabel>,
    tables: Vec<SubTable>,
}

impl SddTable {
    pub fn new(vtree: &VTree) -> SddTable {
        println!("making new table with vtree {:?}", vtree);
        let mut t = SddTable {
            tables: Vec::new(),
            sdd_to_bdd: HashMap::new(),
        };

        for v in vtree.in_order_iter() {
            match v {
                &BTree::Leaf(ref o) => {
                    let mut new_order = Vec::new();
                    let mut m: HashMap<VarLabel, VarLabel> = HashMap::new();
                    for (var_idx, v) in o.iter().enumerate() {
                        t.sdd_to_bdd
                            .insert(v.clone(), VarLabel::new(var_idx as u64));
                        m.insert(VarLabel::new(var_idx as u64), v.clone());
                        new_order.push(VarLabel::new(var_idx as u64));
                    }
                    let man = BddManager::new(VarOrder::new(new_order));
                    t.tables.push(SubTable::BddSubTable { man: man, conv: m })
                }
                &BTree::Node(_, _, _) => {
                    let s = SubTable::SddSubTable {
                        tbl: BackedRobinHoodTable::new(DEFAULT_RH_SZ),
                    };
                    t.tables.push(s);
                }
            }
        }
        t
    }

    pub fn is_sdd(&self, ptr: SddPtr) -> bool {
        match self.tables[ptr.vtree() as usize] {
            SubTable::SddSubTable { tbl: _ } => true,
            _ => false,
        }
    }

    /// Converts a SDD var label into its internal BDD var label; panics on failure
    pub fn sdd_to_bdd_label(&self, label: &VarLabel) -> &VarLabel {
        if self.sdd_to_bdd.get(label).is_none() {
            println!("not found {:?} in table {:?}", label, self.sdd_to_bdd)
        };
        self.sdd_to_bdd.get(label).unwrap()
    }

    pub fn is_bdd(&self, ptr: SddPtr) -> bool {
        match self.tables[ptr.vtree() as usize] {
            SubTable::BddSubTable { man: _, conv: _ } => true,
            _ => false,
        }
    }

    /// get or insert a particular SDD node with vtree-node `vnode`
    pub fn get_or_insert_sdd(&mut self, sdd: &SddOr, vnode: usize) -> SddPtr {
        match &mut self.tables[vnode] {
            &mut SubTable::SddSubTable { ref mut tbl } => {
                let ptr = tbl.get_or_insert(&sdd.nodes);
                SddPtr::new_node(ptr.0 as usize, vnode as u16)
            }
            _ => panic!("invalid vnode: inserting SDD into BDD"),
        }
    }

    /// Fetch the slice for a set of or-nodes; panics if this is not an SDD node
    pub fn sdd_get_or(&self, ptr: SddPtr) -> &[(SddPtr, SddPtr)] {
        match &self.tables[ptr.vtree() as usize] {
            &SubTable::SddSubTable { ref tbl } => &tbl.deref(BackingPtr(ptr.idx() as u32)),
            _ => panic!("dereferencing BDD into SDD"),
        }
    }

    /// Fetch the BDD manager for a particular SDD vtree `vtree_idx`
    /// Panics if it not a BDD
    pub fn bdd_man(&self, vtree_idx: usize) -> &BddManager {
        match &self.tables[vtree_idx] {
            &SubTable::BddSubTable { ref man, ref conv } => man,
            _ => panic!("dereferencing SDD into BDD"),
        }
    }

    pub fn bdd_conv(&self, node: usize) -> &HashMap<VarLabel, VarLabel> {
        match &self.tables[node] {
            &SubTable::BddSubTable { ref man, ref conv } => conv,
            _ => panic!("dereferencing SDD into BDD"),
        }
    }

    /// Fetch the BDD manager for a particular SDD node level `node`
    /// Panics if it not a BDD
    pub fn bdd_man_mut(&mut self, node: usize) -> &mut BddManager {
        match &mut self.tables[node] {
            &mut SubTable::BddSubTable {
                ref mut man,
                ref conv,
            } => man,
            _ => panic!("dereferencing SDD into BDD"),
        }
    }
}
