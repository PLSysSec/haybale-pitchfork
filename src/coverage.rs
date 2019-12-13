use haybale::{ExecutionManager, Project};
use haybale::backend::Backend;
use llvm_ir::Name;
use std::collections::{HashMap, HashSet};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
struct BB {
    pub modname: String,
    pub funcname: String,
    pub bbname: Name,
}

pub struct BlocksSeen(HashSet<BB>);

impl BlocksSeen {
    pub fn new() -> Self {
        Self(HashSet::new())
    }

    pub fn update_with_current_path<B: Backend>(&mut self, em: &ExecutionManager<B>) {
        self.0.extend(em.state().get_path().iter().map(|pathentry| BB {
            modname: pathentry.modname.clone(),
            funcname: pathentry.funcname.clone(),
            bbname: pathentry.bbname.clone(),
        }));
    }

    pub fn num_unique_blocks_seen_in_fn(&self, funcname: &str) -> usize {
        self.0.iter().filter(|bb| bb.funcname == funcname).count()
    }

    /// Returns the percentage of basic blocks in the given function which were seen at least
    /// once by this `BlocksSeen`.  The returned number will be in the range [0,1].
    pub fn block_coverage_of_fn_as_percent(&self, proj: &Project, funcname: &str) -> f64 {
        let blocks_seen: usize = self.num_unique_blocks_seen_in_fn(funcname);
        let (func, _) = proj.get_func_by_name(funcname).unwrap_or_else(|| panic!("Failed to find function {:?} to compute block coverage", funcname));
        let blocks_total: usize = func.basic_blocks.len();
        blocks_seen as f64 / blocks_total as f64
    }
}

/// Map from function names to the percentage of basic blocks in that function
/// which were seen at least once by the `BlocksSeen`. Each percentage will be in
/// the range [0,1].
pub struct BlockCoverage(pub HashMap<String, f64>);

impl BlockCoverage {
    pub fn new(proj: &Project, blocks_seen: &BlocksSeen) -> Self {
        let funcs_seen: HashSet<String> = blocks_seen.0.iter().map(|bb| bb.funcname.clone()).collect();
        Self(funcs_seen
            .into_iter()
            .map(|funcname| {
                let percent = blocks_seen.block_coverage_of_fn_as_percent(proj, &funcname);
                (funcname, percent)
            }).collect()
        )
    }
}
