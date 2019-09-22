//! The `BV`, `Memory`, `BtorRef`, and `Backend` in this module are
//! intended to be used qualified whenever there is a chance of confusing
//! them with `haybale::backend::{BV, Memory, BtorRef, Backend}`,
//! `haybale::memory::Memory`, or `boolector::BV`.

use boolector::{Btor, BVSolution};
use haybale::solver_utils::sat_with_extra_constraints;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct BtorRef {
    pub(crate) btor: haybale::backend::BtorRef,
    ct_violation_observed: Rc<RefCell<Option<CTViolation>>>,
}

impl BtorRef {
    pub fn ct_violation(&self) -> Option<CTViolation> {
        self.ct_violation_observed.borrow().clone()
    }

    pub fn record_ct_violation(&self, v: CTViolation) {
        *self.ct_violation_observed.borrow_mut() = Some(v);
    }
}

impl Default for BtorRef {
    fn default() -> Self {
        Self {
            btor: haybale::backend::BtorRef::default(),
            ct_violation_observed: Rc::new(RefCell::new(None)),
        }
    }
}

impl Deref for BtorRef {
    type Target = Btor;

    fn deref(&self) -> &Btor {
        &self.btor
    }
}

impl haybale::backend::SolverRef for BtorRef {
    type BV = BV;
    type Array = boolector::Array<Rc<Btor>>;

    fn duplicate(&self) -> Self {
        Self {
            btor: self.btor.duplicate(),
            ct_violation_observed: Rc::new(RefCell::new(self.ct_violation_observed.borrow().clone())),
        }
    }

    fn match_bv(&self, bv: &BV) -> Option<BV> {
        match bv {
            BV::Public(bv) => self.btor.match_bv(bv).map(BV::Public),
            BV::Secret { .. } => Some(bv.clone()),
        }
    }

    fn match_array(&self, array: &boolector::Array<Rc<Btor>>) -> Option<boolector::Array<Rc<Btor>>> {
        self.btor.match_array(array)
    }
}

impl From<BtorRef> for Rc<Btor> {
    fn from(btor: BtorRef) -> Rc<Btor> {
        btor.btor.into()
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum CTViolation {
    /// `Secret` values influenced an address calculation
    AddressCalculation,
    /// `Secret` values influenced control flow
    ControlFlowDecision,
    /// `Secret` values leaked externally, e.g. influenced arguments to a logging function
    LeakedExternally,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BV {
    Public(boolector::BV<Rc<Btor>>),
    /// `Secret` values are opaque because we don't care about their actual value, only how they are used.
    Secret {
        btor: BtorRef,
        width: u32,
        symbol: Option<String>,
    },
}

impl BV {
    pub fn is_secret(&self) -> bool {
        match self {
            BV::Public(_) => false,
            BV::Secret { .. } => true,
        }
    }

    /// Gets the value out of a `BV::Public`, panicking if it is instead a `BV::Secret`
    pub fn as_public(&self) -> &boolector::BV<Rc<Btor>> {
        match self {
            BV::Public(bv) => bv,
            BV::Secret { .. } => panic!("as_public on a BV::Secret"),
        }
    }
}

macro_rules! impl_unop_as_functor {
    ($f:ident) => {
        fn $f(&self) -> Self {
            match self {
                BV::Public(bv) => BV::Public(bv.$f()),
                BV::Secret { btor, width, .. } => BV::Secret { btor: btor.clone(), width: *width, symbol: None }, // use this macro only for unary ops which don't change the bitwidth
            }
        }
    };
}

macro_rules! impl_unop_as_functor_return_bool {
    ($f:ident) => {
        fn $f(&self) -> Self {
            match self {
                BV::Public(bv) => BV::Public(bv.$f()),
                BV::Secret { btor, .. } => BV::Secret { btor: btor.clone(), width: 1, symbol: None },
            }
        }
    };
}

macro_rules! impl_binop_as_functor {
    ($f:ident) => {
        fn $f(&self, other: &Self) -> Self {
            match (self, other) {
                (BV::Public(bv), BV::Public(other)) => BV::Public(bv.$f(other)),
                (BV::Secret { btor, width, .. }, _) => BV::Secret { btor: btor.clone(), width: *width, symbol: None },
                (_, BV::Secret { btor, width, .. }) => BV::Secret { btor: btor.clone(), width: *width, symbol: None },
            }
        }
    };
}

macro_rules! impl_binop_as_functor_return_bool {
    ($f:ident) => {
        fn $f(&self, other: &Self) -> Self {
            match (self, other) {
                (BV::Public(bv), BV::Public(other)) => BV::Public(bv.$f(other)),
                (BV::Secret { btor, .. }, _) => BV::Secret { btor: btor.clone(), width: 1, symbol: None },
                (_, BV::Secret { btor, .. }) => BV::Secret { btor: btor.clone(), width: 1, symbol: None },
            }
        }
    };
}

impl haybale::backend::BV for BV {
    type SolverRef = BtorRef;

    fn new(btor: BtorRef, width: u32, name: Option<&str>) -> Self {
        BV::Public(boolector::BV::new(btor.btor.into(), width, name))
    }
    fn from_bool(btor: BtorRef, b: bool) -> Self {
        BV::Public(boolector::BV::from_bool(btor.btor.into(), b))
    }
    fn from_i32(btor: BtorRef, i: i32, width: u32) -> Self {
        BV::Public(boolector::BV::from_i32(btor.btor.into(), i, width))
    }
    fn from_u32(btor: BtorRef, u: u32, width: u32) -> Self {
        BV::Public(boolector::BV::from_u32(btor.btor.into(), u, width))
    }
    fn from_i64(btor: BtorRef, i: i64, width: u32) -> Self {
        BV::Public(boolector::BV::from_i64(btor.btor.into(), i, width))
    }
    fn from_u64(btor: BtorRef, u: u64, width: u32) -> Self {
        BV::Public(boolector::BV::from_u64(btor.btor.into(), u, width))
    }
    fn zero(btor: BtorRef, width: u32) -> Self {
        BV::Public(boolector::BV::zero(btor.btor.into(), width))
    }
    fn one(btor: BtorRef, width: u32) -> Self {
        BV::Public(boolector::BV::one(btor.btor.into(), width))
    }
    fn ones(btor: BtorRef, width: u32) -> Self {
        BV::Public(boolector::BV::ones(btor.btor.into(), width))
    }
    fn from_binary_str(btor: BtorRef, bits: &str) -> Self {
        BV::Public(boolector::BV::from_binary_str(btor.btor.into(), bits))
    }
    fn from_dec_str(btor: BtorRef, num: &str, width: u32) -> Self {
        BV::Public(boolector::BV::from_dec_str(btor.btor.into(), num, width))
    }
    fn from_hex_str(btor: BtorRef, num: &str, width: u32) -> Self {
        BV::Public(boolector::BV::from_hex_str(btor.btor.into(), num, width))
    }
    fn as_binary_str(&self) -> Option<String> {
        match self {
            BV::Public(bv) => bv.as_binary_str(),
            BV::Secret { .. } => None,
        }
    }
    fn as_u64(&self) -> Option<u64> {
        match self {
            BV::Public(bv) => bv.as_u64(),
            BV::Secret { .. } => None,
        }
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            BV::Public(bv) => bv.as_bool(),
            BV::Secret { .. } => None,
        }
    }
    fn get_a_solution(&self) -> BVSolution {
        match self {
            BV::Public(bv) => bv.get_a_solution(),
            BV::Secret { .. } => panic!("get_a_solution() on a Secret value"),
        }
    }
    fn get_id(&self) -> i32 {
        match self {
            BV::Public(bv) => bv.get_id(),
            BV::Secret { .. } => panic!("get_id() on a Secret value"),
        }
    }
    fn get_width(&self) -> u32 {
        match self {
            BV::Public(bv) => bv.get_width(),
            BV::Secret { width, .. } => *width,
        }
    }
    fn get_symbol(&self) -> Option<&str> {
        match self {
            BV::Public(bv) => bv.get_symbol(),
            BV::Secret { symbol, .. } => symbol.as_ref().map(|s| s.as_ref()),
        }
    }
    fn set_symbol(&mut self, symbol: Option<&str>) {
        match self {
            BV::Public(bv) => bv.set_symbol(symbol),
            BV::Secret { btor, width, .. } => *self = BV::Secret { btor: btor.clone(), width: *width, symbol: symbol.map(|s| s.to_owned()) },
        }
    }
    fn is_const(&self) -> bool {
        match self {
            BV::Public(bv) => bv.is_const(),
            BV::Secret { .. } => false,
        }
    }
    fn has_same_width(&self, other: &Self) -> bool {
        match (self, other) {
            (BV::Public(bv), BV::Public(other)) => bv.has_same_width(other),
            _ => self.get_width() == other.get_width(),
        }
    }
    fn assert(&self) {
        match self {
            BV::Public(bv) => bv.assert(),
            BV::Secret { btor, .. } => btor.record_ct_violation(CTViolation::ControlFlowDecision),  // `Secret` values influencing a path constraint means they influenced a control flow decision
        }
    }
    fn is_failed_assumption(&self) -> bool {
        match self {
            BV::Public(bv) => bv.is_failed_assumption(),
            BV::Secret { .. } => false,
        }
    }

    impl_binop_as_functor!(_eq);
    impl_binop_as_functor!(_ne);
    impl_binop_as_functor!(add);
    impl_binop_as_functor!(sub);
    impl_binop_as_functor!(mul);
    impl_binop_as_functor!(udiv);
    impl_binop_as_functor!(sdiv);
    impl_binop_as_functor!(urem);
    impl_binop_as_functor!(srem);
    impl_binop_as_functor!(smod);
    impl_unop_as_functor!(inc);
    impl_unop_as_functor!(dec);
    impl_unop_as_functor!(neg);
    impl_binop_as_functor!(uaddo);
    impl_binop_as_functor!(saddo);
    impl_binop_as_functor!(usubo);
    impl_binop_as_functor!(ssubo);
    impl_binop_as_functor!(umulo);
    impl_binop_as_functor!(smulo);
    impl_binop_as_functor!(sdivo);
    impl_unop_as_functor!(not);
    impl_binop_as_functor!(and);
    impl_binop_as_functor!(or);
    impl_binop_as_functor!(xor);
    impl_binop_as_functor!(nand);
    impl_binop_as_functor!(nor);
    impl_binop_as_functor!(xnor);
    impl_binop_as_functor!(sll);
    impl_binop_as_functor!(srl);
    impl_binop_as_functor!(sra);
    impl_binop_as_functor!(rol);
    impl_binop_as_functor!(ror);
    impl_unop_as_functor_return_bool!(redand);
    impl_unop_as_functor_return_bool!(redor);
    impl_unop_as_functor_return_bool!(redxor);
    impl_binop_as_functor_return_bool!(ugt);
    impl_binop_as_functor_return_bool!(ugte);
    impl_binop_as_functor_return_bool!(sgt);
    impl_binop_as_functor_return_bool!(sgte);
    impl_binop_as_functor_return_bool!(ult);
    impl_binop_as_functor_return_bool!(ulte);
    impl_binop_as_functor_return_bool!(slt);
    impl_binop_as_functor_return_bool!(slte);

    fn zext(&self, i: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.zext(i)),
            BV::Secret { btor, width, .. } => BV::Secret { btor: btor.clone(), width: *width + i, symbol: None },
        }
    }
    fn sext(&self, i: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.sext(i)),
            BV::Secret { btor, width, .. } => BV::Secret { btor: btor.clone(), width: *width + i, symbol: None },
        }
    }
    fn slice(&self, high: u32, low: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.slice(high, low)),
            BV::Secret { btor, .. } => BV::Secret { btor: btor.clone(), width: high - low + 1, symbol: None },
        }
    }
    fn concat(&self, other: &Self) -> Self {
        match (self, other) {
            (BV::Public(bv), BV::Public(other)) => BV::Public(bv.concat(other)),
            (BV::Secret { btor, width, .. }, _) => BV::Secret { btor: btor.clone(), width: *width + other.get_width(), symbol: None },
            (_, BV::Secret { btor, width, .. }) => BV::Secret { btor: btor.clone(), width: *width + self.get_width(), symbol: None },
        }
    }
    fn repeat(&self, n: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.repeat(n)),
            BV::Secret { btor, width, .. } => BV::Secret { btor: btor.clone(), width: width * n, symbol: None },
        }
    }

    impl_binop_as_functor!(iff);
    impl_binop_as_functor!(implies);

    fn cond_bv(&self, truebv: &Self, falsebv: &Self) -> Self {
        match (self, truebv, falsebv) {
            (BV::Public(bv), BV::Public(truebv), BV::Public(falsebv)) => BV::Public(bv.cond_bv(truebv, falsebv)),
            (BV::Secret { btor, .. }, _, _) => BV::Secret { btor: btor.clone(), width: truebv.get_width(), symbol: None },
            (_, BV::Secret { btor, width, .. }, _) => BV::Secret { btor: btor.clone(), width: *width, symbol: None },
            (_, _, BV::Secret { btor, width, .. }) => BV::Secret { btor: btor.clone(), width: *width, symbol: None },
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Memory {
    btor: BtorRef,
    /// This memory holds the actual data
    mem: haybale::memory::Memory,
    /// This memory is a bitmap, with each bit indicating if the corresponding bit of `mem` is secret or not (1 for secret, 0 for public)
    shadow_mem: haybale::memory::Memory,
}

impl haybale::backend::Memory for Memory {
    type SolverRef = BtorRef;
    type Index = BV;
    type Value = BV;

    fn new_uninitialized(btor: BtorRef) -> Self {
        Self {
            mem: haybale::backend::Memory::new_uninitialized(btor.btor.clone()),
            shadow_mem: haybale::backend::Memory::new_zero_initialized(btor.btor.clone()), // shadow bits are zero-initialized (all public) even though the memory contents are uninitialized
            btor,  // out of order so it can be used above but moved in here
        }
    }
    fn new_zero_initialized(btor: BtorRef) -> Self {
        Self {
            mem: haybale::backend::Memory::new_zero_initialized(btor.btor.clone()),
            shadow_mem: haybale::backend::Memory::new_zero_initialized(btor.btor.clone()), // initialize to all public zeroes
            btor,  // out of order so it can be used above but moved in here
        }
    }
    fn read(&self, index: &Self::Index, bits: u32) -> Self::Value {
        match index {
            BV::Public(index) => {
                let shadow_cell = haybale::backend::Memory::read(&self.shadow_mem, index, bits);
                // In Boolector, reads on a constant array that return the default value are
                // nonetheless not constant (they are merely constrained to be equal to the
                // default value). So, we actually need to do a solve here.
                //
                // However, we really only care whether the shadow value is all zeroes (all
                // public) or not-all-zeroes (some or all secret). This means we can get away
                // with using a faster `sat_with_extra_constraints()` check rather than a slow
                // `get_possible_solutions_for_bv()` check.
                let rc: Rc<Btor> = self.btor.clone().into();
                let all_zeroes = boolector::BV::zero(rc.clone(), shadow_cell.get_width());
                if sat_with_extra_constraints(&rc, std::iter::once(&shadow_cell._ne(&all_zeroes))).unwrap() {
                    // This can happen multiple ways:
                    // (1) Some or all of the bits are secret;
                    // (2) The bits' secrecy is non-constant; i.e., the bits could be secret
                    //      or not, depending on the values of other variables. This can
                    //      happen, e.g, when reading from a symbolic address that could
                    //      point to either secret or public data.
                    // In either case, since all or part of the resulting value _could be_
                    // secret, we treat the resulting value as entirely secret (following the
                    // worst case).
                    BV::Secret { btor: self.btor.clone(), width: bits, symbol: None }
                } else {
                    // Since the above query was unsat, the only possible solution is that
                    // the bits are all public
                    BV::Public(haybale::backend::Memory::read(&self.mem, index, bits))
                }
            },
            BV::Secret { btor, .. } => {
                btor.record_ct_violation(CTViolation::AddressCalculation);
                BV::Secret { btor: btor.clone(), width: bits, symbol: None }
            }
        }
    }
    fn write(&mut self, index: &Self::Index, value: Self::Value) {
        match index {
            BV::Public(index) => match value {
                BV::Public(value) => {
                    let all_zeroes = boolector::BV::zero(self.btor.clone().into(), value.get_width());
                    haybale::backend::Memory::write(&mut self.shadow_mem, index, all_zeroes); // we are writing a public value to these bits
                    haybale::backend::Memory::write(&mut self.mem, index, value);
                },
                BV::Secret { btor, width, .. } => {
                    let all_ones = boolector::BV::ones(btor.clone().into(), width);
                    haybale::backend::Memory::write(&mut self.shadow_mem, index, all_ones); // we are writing a secret value to these bits
                    // we don't write anything to self.mem, because the value of its secret bits doesn't matter
                },
            },
            BV::Secret { btor, .. } => {
                btor.record_ct_violation(CTViolation::AddressCalculation);
            },
        }
    }
    fn change_solver(&mut self, new_solver: BtorRef) {
        self.mem.change_solver(new_solver.btor.clone());
        self.shadow_mem.change_solver(new_solver.btor.clone());
        self.btor = new_solver;
    }
}

#[derive(Clone, Debug)]
pub struct Backend {}

impl haybale::backend::Backend for Backend {
    type SolverRef = BtorRef;
    type BV = BV;
    type Memory = Memory;
}
