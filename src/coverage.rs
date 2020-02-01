use haybale::ExecutionManager;
use haybale::backend::Backend;
use llvm_ir::{Function, Module, Name};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone)]
struct BB<'p> {
    pub module: &'p Module,
    pub func: &'p Function,
    pub bbname: Name,
}

// Our implementations of PartialEq, Eq, PartialOrd, Ord assume that module
// names are unique, and that function names are unique within modules

impl<'p> PartialEq for BB<'p> {
    fn eq(&self, other: &Self) -> bool {
        self.module.name == other.module.name
            && self.func.name == other.func.name
            && self.bbname == other.bbname
    }
}

impl<'p> Eq for BB<'p> { }

impl<'p> PartialOrd for BB<'p> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))  // defer to the Ord implementation
    }
}

impl<'p> Ord for BB<'p> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.module.name, &self.func.name, &self.bbname).cmp(
            &(&other.module.name, &other.func.name, &other.bbname)
        )
    }
}

pub struct BlocksSeen<'p>(BTreeSet<BB<'p>>);

impl<'p> BlocksSeen<'p> {
    pub fn new() -> Self {
        Self(BTreeSet::new())
    }

    pub fn update_with_current_path<B: Backend>(&mut self, em: &ExecutionManager<'p, B>) {
        self.0.extend(em.state().get_path().iter().map(|pathentry| BB {
            module: pathentry.0.module,
            func: pathentry.0.func,
            bbname: pathentry.0.bb.name.clone(),
        }));
    }

    /// Returns an iterator of all the (unique) `BB`s in the given function which
    /// were seen at least once by this `BlocksSeen`.
    ///
    /// `funcname` must be a fully mangled name, as appears in the LLVM.
    fn seen_blocks_in_fn<'a>(&'a self, funcname: &'a str) -> impl Iterator<Item = &'a BB> {
        self.0.iter().filter(move |bb| bb.func.name == funcname)
    }

    /// Returns the percentage of basic blocks in the given function which were seen at least
    /// once by this `BlocksSeen`.  The returned number will be in the range [0,1].
    #[allow(dead_code)]  // this code is currently dead (as of this writing), but seems like a thing we might want in the future
    pub fn block_coverage_of_fn_as_percent(&self, funcname: &str) -> f64 {
        let blocks_seen: usize = self.seen_blocks_in_fn(funcname).count();
        let func = match self.seen_blocks_in_fn(funcname).next() {
            Some(bb) => bb.func,
            None => return 0.0,  // we haven't seen any blocks in that function, so coverage is 0%
        };
        let blocks_total: usize = func.basic_blocks.len();
        blocks_seen as f64 / blocks_total as f64
    }

    /// Returns a map from (mangled) function names to the `BlockCoverage` of that
    /// function, as seen by this `BlocksSeen`.
    pub fn full_coverage_stats(&self) -> HashMap<String, BlockCoverage> {
        let funcs_seen: HashSet<String> = self.0.iter().map(|bb| bb.func.name.clone()).collect();
        funcs_seen.into_iter().filter_map(|funcname| {
            BlockCoverage::new(&funcname, self).map(|bc| (funcname, bc))
        }).collect()
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
    /// `funcname` must be a fully mangled name, as appears in the LLVM.
    ///
    /// Returns `None` if we seem to have seen no blocks from functions named `funcname`.
    pub fn new(funcname: &str, blocks_seen: &BlocksSeen) -> Option<Self> {
        let func = blocks_seen.seen_blocks_in_fn(funcname).next()?.func;
        let seen_blocks: BTreeSet<_> = blocks_seen
            .seen_blocks_in_fn(funcname)
            .map(|bb| bb.bbname.clone())
            .collect();
        let missed_blocks: BTreeSet<_> = func
            .basic_blocks
            .iter()
            .filter(|bb| !seen_blocks.contains(&bb.name))
            .map(|bb| bb.name.clone())
            .collect();
        Some(Self {
            percentage: seen_blocks.len() as f64 / (seen_blocks.len() + missed_blocks.len()) as f64,
            seen_blocks,
            missed_blocks,
        })
    }
}
