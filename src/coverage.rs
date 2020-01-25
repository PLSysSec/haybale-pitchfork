use haybale::{ExecutionManager, Project};
use haybale::backend::Backend;
use llvm_ir::Name;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Hash)]
struct BB {
    pub modname: String,
    pub funcname: String,  // always the mangled name here (as appears in the LLVM)
    pub bbname: Name,
}

pub struct BlocksSeen(HashSet<BB>);

impl BlocksSeen {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn update_with_current_path<B: Backend>(&mut self, em: &ExecutionManager<B>) {
        self.0.extend(em.state().get_path().iter().map(|pathentry| BB {
            modname: pathentry.0.module.name.clone(),
            funcname: pathentry.0.func.name.clone(),
            bbname: pathentry.0.bb.name.clone(),
        }));
    }

    /// Returns an iterator of all the (unique) `BB`s in the given function which
    /// were seen at least once by this `BlocksSeen`.
    ///
    /// `funcname` must be a fully mangled name, as appears in the LLVM.
    fn seen_blocks_in_fn<'a, 'b>(&'a self, funcname: &'a str) -> impl Iterator<Item = &'a BB> {
        self.0.iter().filter(move |bb| bb.funcname == funcname)
    }

    /// Returns the percentage of basic blocks in the given function which were seen at least
    /// once by this `BlocksSeen`.  The returned number will be in the range [0,1].
    #[allow(dead_code)]  // this code is currently dead (as of this writing), but seems like a thing we might want in the future
    pub fn block_coverage_of_fn_as_percent(&self, proj: &Project, funcname: &str) -> f64 {
        let blocks_seen: usize = self.seen_blocks_in_fn(funcname).count();
        let (func, _) = proj.get_func_by_name(funcname).unwrap_or_else(|| panic!("Failed to find function {:?} to compute block coverage", funcname));
        let blocks_total: usize = func.basic_blocks.len();
        blocks_seen as f64 / blocks_total as f64
    }
}

/// This struct describes block coverage of a single function.
pub struct BlockCoverage {
    /// The percentage of basic blocks in the function which were seen at least
    /// once by the `BlocksSeen`. Will be in the range [0,1].
    pub percentage: f64,

    /// The block names in the function which were seen by the `BlocksSeen`.
    pub seen_blocks: BTreeSet<Name>,  // BTreeSet rather than HashSet so that you can easily iterate over them in order if desired

    /// The block names in the function which were not seen by the `BlocksSeen`.
    pub missed_blocks: BTreeSet<Name>,  // BTreeSet rather than HashSet so that you can easily iterate over them in order if desired
}

impl BlockCoverage {
    /// `funcname` may be either a mangled or demangled name here.
    pub fn new(proj: &Project, funcname: &str, blocks_seen: &BlocksSeen) -> Self {
        let (func, _) = proj.get_func_by_name(funcname).unwrap_or_else(|| panic!("Failed to find function {:?} to compute block coverage", funcname));
        let seen_blocks: BTreeSet<_> = blocks_seen
            .seen_blocks_in_fn(&func.name)  // the mangled name, even if `funcname` is demangled
            .map(|bb| bb.bbname.clone())
            .collect();
        let missed_blocks: BTreeSet<_> = func
            .basic_blocks
            .iter()
            .filter(|bb| !seen_blocks.contains(&bb.name))
            .map(|bb| bb.name.clone())
            .collect();
        Self {
            percentage: seen_blocks.len() as f64 / (seen_blocks.len() + missed_blocks.len()) as f64,
            seen_blocks,
            missed_blocks,
        }
    }
}

/// Returns a map from (mangled) function names to the `BlockCoverage` of that
/// function, as seen by the given `BlocksSeen`.
pub fn compute_coverage_stats(proj: &Project, blocks_seen: &BlocksSeen) -> HashMap<String, BlockCoverage> {
    let funcs_seen: HashSet<String> = blocks_seen.0.iter().map(|bb| bb.funcname.clone()).collect();
    funcs_seen.into_iter().map(|funcname| {
        let bc = BlockCoverage::new(proj, &funcname, blocks_seen);
        (funcname, bc)
    }).collect()
}
