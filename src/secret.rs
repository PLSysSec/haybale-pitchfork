//! The `BV`, `Bool`, `Array`, `Solver`, and `Backend` in this module are
//! intended to be used qualified whenever there is a chance of confusing
//! them with `haybale::backend::{BV, Bool, Array, Solver, Backend}`,
//! `haybale::solver::Solver`, or `z3::ast::{BV, Bool, Array}`.

use std::cell::RefCell;
use std::rc::Rc;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BV<'ctx> {
    Public(z3::ast::BV<'ctx>),
    /// `Secret` values are opaque because we don't care about their actual value, only how they are used.
    /// The `u32` is the size (in bits) of the tainted value.
    Secret(u32),
}

impl<'ctx> BV<'ctx> {
    pub fn is_tainted(&self) -> bool {
        match self {
            BV::Public(_) => false,
            BV::Secret(_) => true,
        }
    }

    /// Gets the value out of a `BV::Public`, panicking if it is instead a `BV::Secret`
    pub fn as_public(&self) -> &z3::ast::BV<'ctx> {
        match self {
            BV::Public(bv) => bv,
            BV::Secret(_) => panic!("as_public on a BV::Secret"),
        }
    }
}

macro_rules! impl_unop_as_functor {
    ($f:ident) => {
        fn $f(&self) -> Self {
            match self {
                BV::Public(bv) => BV::Public(bv.$f()),
                BV::Secret(bits) => BV::Secret(*bits), // assume that unary ops don't change the bitwidth
            }
        }
    };
}

macro_rules! impl_binop_as_functor {
    ($f:ident) => {
        fn $f(&self, other: &Self) -> Self {
            match (self, other) {
                (BV::Public(bv), BV::Public(other)) => BV::Public(bv.$f(other)),
                _ => BV::Secret(self.get_size()),
            }
        }
    };
}

macro_rules! impl_binop_as_functor_return_bool {
    ($f:ident) => {
        fn $f(&self, other: &Self) -> Self::AssociatedBool {
            match (self, other) {
                (BV::Public(bv), BV::Public(other)) => Bool::Public(bv.$f(other)),
                _ => Bool::Secret,
            }
        }
    };
}

impl<'ctx> haybale::backend::BV<'ctx> for BV<'ctx> {
    type AssociatedBool = Bool<'ctx>;

    fn new(ctx: &'ctx z3::Context, name: impl Into<z3::Symbol>, size: u32) -> Self {
        BV::Public(z3::ast::BV::new(ctx, name, size))
    }
    fn from_i64(ctx: &'ctx z3::Context, i: i64, size: u32) -> Self {
        BV::Public(z3::ast::BV::from_i64(ctx, i, size))
    }
    fn from_u64(ctx: &'ctx z3::Context, u: u64, size: u32) -> Self {
        BV::Public(z3::ast::BV::from_u64(ctx, u, size))
    }
    fn as_i64(&self) -> Option<i64> {
        match self {
            BV::Public(bv) => bv.as_i64(),
            BV::Secret(_) => None,
        }
    }
    fn as_u64(&self) -> Option<u64> {
        match self {
            BV::Public(bv) => bv.as_u64(),
            BV::Secret(_) => None,
        }
    }
    fn get_size(&self) -> u32 {
        match self {
            BV::Public(bv) => bv.get_size(),
            BV::Secret(bits) => *bits,
        }
    }

    impl_unop_as_functor!(not);
    impl_unop_as_functor!(neg);
    impl_binop_as_functor!(and);
    impl_binop_as_functor!(or);
    impl_binop_as_functor!(xor);
    impl_binop_as_functor!(nand);
    impl_binop_as_functor!(nor);
    impl_binop_as_functor!(xnor);
    impl_unop_as_functor!(redand);
    impl_unop_as_functor!(redor);
    impl_binop_as_functor!(add);
    impl_binop_as_functor!(sub);
    impl_binop_as_functor!(mul);
    impl_binop_as_functor!(udiv);
    impl_binop_as_functor!(sdiv);
    impl_binop_as_functor!(urem);
    impl_binop_as_functor!(srem);
    impl_binop_as_functor!(smod);
    impl_binop_as_functor_return_bool!(ult);
    impl_binop_as_functor_return_bool!(slt);
    impl_binop_as_functor_return_bool!(ule);
    impl_binop_as_functor_return_bool!(sle);
    impl_binop_as_functor_return_bool!(uge);
    impl_binop_as_functor_return_bool!(sge);
    impl_binop_as_functor_return_bool!(ugt);
    impl_binop_as_functor_return_bool!(sgt);
    impl_binop_as_functor!(shl);
    impl_binop_as_functor!(lshr);
    impl_binop_as_functor!(ashr);
    impl_binop_as_functor!(rotl);
    impl_binop_as_functor!(rotr);
    impl_binop_as_functor!(concat);
    impl_binop_as_functor_return_bool!(_eq);
    impl_unop_as_functor!(simplify);

    fn extract(&self, high: u32, low: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.extract(high, low)),
            BV::Secret(_) => BV::Secret(high - low + 1),
        }
    }
    fn sign_ext(&self, i: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.sign_ext(i)),
            BV::Secret(bits) => BV::Secret(bits + i),
        }
    }
    fn zero_ext(&self, i: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.zero_ext(i)),
            BV::Secret(bits) => BV::Secret(bits + i),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Bool<'ctx> {
    Public(z3::ast::Bool<'ctx>),
    Secret, // `Secret` values are opaque because we don't care about their actual value, only how they are used
}

impl<'ctx> Bool<'ctx> {
    pub fn is_tainted(&self) -> bool {
        match self {
            Bool::Public(_) => false,
            Bool::Secret => true,
        }
    }

    /// Gets the value out of a `Bool::Public`, panicking if it is instead a `Bool::Secret`
    pub fn as_public(&self) -> &z3::ast::Bool<'ctx> {
        match self {
            Bool::Public(b) => b,
            Bool::Secret => panic!("as_public on a Bool::Secret"),
        }
    }
}

impl<'ctx> haybale::backend::Bool<'ctx> for Bool<'ctx> {
    type AssociatedBV = BV<'ctx>;

    fn new(ctx: &'ctx z3::Context, name: impl Into<z3::Symbol>) -> Self {
        Bool::Public(z3::ast::Bool::new(ctx, name))
    }
    fn from_bool(ctx: &'ctx z3::Context, b: bool) -> Self {
        Bool::Public(z3::ast::Bool::from_bool(ctx, b))
    }
    fn bvite(&self, a: &Self::AssociatedBV, b: &Self::AssociatedBV) -> Self::AssociatedBV {
        use haybale::backend::BV; // need the trait in scope so that we can use its methods (`get_size()`)
                                    // unfortunately this means that the `BV` in this module must now be referred to as `self::BV`
        match (self, a, b) {
            (Bool::Public(c), self::BV::Public(a), self::BV::Public(b)) => self::BV::Public(c.bvite(a, b)),
            _ => self::BV::Secret(a.get_size()),
        }
    }
    fn boolite(&self, a: &Self, b: &Self) -> Self {
        match (self, a, b) {
            (Bool::Public(c), Bool::Public(a), Bool::Public(b)) => Bool::Public(c.boolite(a, b)),
            _ => Bool::Secret,
        }
    }
    fn and(&self, other: &[&Self]) -> Self {
        let mut maybe_publics = Some(vec![]);
        for b in other {
            if let (Some(ref mut v), Bool::Public(b)) = (maybe_publics.as_mut(), b) {
                v.push(b);
            }
        }
        match (self, maybe_publics) {
            (Bool::Public(b), Some(ref v)) => Bool::Public(b.and(&v)),
            _ => Bool::Secret,
        }
    }
    fn or(&self, other: &[&Self]) -> Self {
        let mut maybe_publics = Some(vec![]);
        for b in other {
            if let (Some(ref mut v), Bool::Public(b)) = (maybe_publics.as_mut(), b) {
                v.push(b);
            }
        }
        match (self, maybe_publics) {
            (Bool::Public(b), Some(ref v)) => Bool::Public(b.or(&v)),
            _ => Bool::Secret,
        }
    }
    fn xor(&self, other: &Self) -> Self {
        match (self, other) {
            (Bool::Public(x), Bool::Public(y)) => Bool::Public(x.xor(y)),
            _ => Bool::Secret,
        }
    }
    fn not(&self) -> Self {
        match self {
            Bool::Public(b) => Bool::Public(b.not()),
            _ => Bool::Secret,
        }
    }
    fn iff(&self, other: &Self) -> Self {
        match (self, other) {
            (Bool::Public(x), Bool::Public(y)) => Bool::Public(x.iff(y)),
            _ => Bool::Secret,
        }
    }
    fn implies(&self, other: &Self) -> Self {
        match (self, other) {
            (Bool::Public(x), Bool::Public(y)) => Bool::Public(x.implies(y)),
            _ => Bool::Secret,
        }
    }
    fn _eq(&self, other: &Self) -> Self {
        match (self, other) {
            (Bool::Public(x), Bool::Public(y)) => Bool::Public(x._eq(y)),
            _ => Bool::Secret,
        }
    }
    fn simplify(&self) -> Self {
        match self {
            Bool::Public(b) => Bool::Public(b.simplify()),
            _ => Bool::Secret,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct State {
    ct_violation_observed: bool,
}

impl State {
    pub fn ct_violation_observed(&self) -> bool {
        self.ct_violation_observed
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            ct_violation_observed: false,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Array<'ctx> {
    array: z3::ast::Array<'ctx>,
    indexbits: u32,
    valuebits: u32,
    backend_state: Rc<RefCell<State>>,
}

impl<'ctx> haybale::backend::Array<'ctx> for Array<'ctx> {
    type Index = BV<'ctx>;
    type Value = BV<'ctx>;
    type BackendState = State;

    fn new(ctx: &'ctx z3::Context, backend_state: Rc<RefCell<Self::BackendState>>, name: impl Into<z3::Symbol>, indexbits: u32, valuebits: u32) -> Self {
        Self {
            array: haybale::backend::Array::new(ctx, Rc::new(RefCell::new(())), name, indexbits, valuebits),
            indexbits,
            valuebits,
            backend_state,
        }
    }
    fn select(&self, index: Self::Index) -> Self::Value {
        match index {
            BV::Public(index) => BV::Public(haybale::backend::Array::select(&self.array, index)),
            BV::Secret(bits) => {
                self.backend_state.borrow_mut().ct_violation_observed = true;  // `Secret` values influencing an address calculation is a ct violation
                assert_eq!(bits, self.indexbits);
                BV::Secret(self.valuebits)
            }
        }
    }
    fn store(&self, index: Self::Index, value: Self::Value) -> Self {
        match (index, value) {
            (BV::Public(index), BV::Public(value)) => Self {
                array: haybale::backend::Array::store(&self.array, index, value),
                indexbits: self.indexbits,
                valuebits: self.valuebits,
                backend_state: self.backend_state.clone(),
            },
            _ => {
                self.backend_state.borrow_mut().ct_violation_observed = true;  // `Secret` values influencing an address calculation is a ct violation
                self.clone() // should be a cheap operation
            },
        }
    }
    fn simplify(&self) -> Self {
        Self {
            array: haybale::backend::Array::simplify(&self.array),
            indexbits: self.indexbits,
            valuebits: self.valuebits,
            backend_state: self.backend_state.clone(),
        }
    }
}

pub struct Solver<'ctx> {
    haybale_solver: haybale::solver::Solver<'ctx>,
    backend_state: Rc<RefCell<State>>,
}

impl<'ctx> haybale::backend::Solver<'ctx> for Solver<'ctx> {
    type Constraint = Bool<'ctx>;
    type Value = BV<'ctx>;
    type BackendState = State;

    fn new(ctx: &'ctx z3::Context, backend_state: Rc<RefCell<State>>) -> Self {
        Self {
            haybale_solver: haybale::backend::Solver::new(ctx, Rc::new(RefCell::new(()))),
            backend_state,
        }
    }
    fn assert(&mut self, constraint: &Self::Constraint) {
        match constraint {
            Bool::Public(c) => self.haybale_solver.assert(c),
            Bool::Secret => self.backend_state.borrow_mut().ct_violation_observed = true,  // `Secret` values influencing a path constraint is a ct violation
        };
    }
    fn check(&mut self) -> bool {
        self.haybale_solver.check()
    }
    fn check_with_extra_constraints<'a>(&'a mut self, constraints: impl Iterator<Item = &'a Self::Constraint>) -> bool {
        self.haybale_solver.check_with_extra_constraints(
            constraints
                .filter(|c| !c.is_tainted())
                .map(Bool::as_public),
        )
    }
    fn push(&mut self) {
        self.haybale_solver.push()
    }
    fn pop(&mut self, n: usize) {
        self.haybale_solver.pop(n)
    }
    fn get_a_solution_for_bv(&mut self, bv: &Self::Value) -> Option<u64> {
        match bv {
            BV::Public(bv) => self.haybale_solver.get_a_solution_for_bv(bv),
            BV::Secret(_) => None,
        }
    }
    fn get_a_solution_for_specified_bits_of_bv(&mut self, bv: &Self::Value, high: u32, low: u32) -> Option<u64> {
        match bv {
            BV::Public(bv) => self
                .haybale_solver
                .get_a_solution_for_specified_bits_of_bv(bv, high, low),
            BV::Secret(_) => None,
        }
    }
    fn get_a_solution_for_bool(&mut self, b: &Self::Constraint) -> Option<bool> {
        match b {
            Bool::Public(b) => self.haybale_solver.get_a_solution_for_bool(b),
            Bool::Secret => None,
        }
    }
    fn current_model_to_pretty_string(&self) -> String {
        self.haybale_solver.current_model_to_pretty_string()
    }
}

pub struct Backend<'ctx> {
    phantomdata: std::marker::PhantomData<&'ctx ()>,
}

impl<'ctx> haybale::backend::Backend<'ctx> for Backend<'ctx> {
    type BV = BV<'ctx>;
    type Bool = Bool<'ctx>;
    type Array = Array<'ctx>;
    type Solver = Solver<'ctx>;
    type State = State;
}
