//! The `BV`, `Memory`, and `Backend` in this module are
//! intended to be used qualified whenever there is a chance of confusing
//! them with `haybale::backend::{BV, Memory, Backend}`,
//! `haybale::memory::Memory`, or `boolector::BV`.

use boolector::{Btor, BVSolution};
use haybale::{Error, Result};
use log::warn;
use std::ops::Deref;
use std::rc::Rc;

/// This wrapper around `Rc<Btor>` exists simply so we can give it a different
/// implementation of `haybale::backend::SolverRef` than the one provided by
/// `haybale::backend`
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct BtorRef(pub(crate) Rc<Btor>);

impl Deref for BtorRef {
    type Target = Btor;

    fn deref(&self) -> &Btor {
        &self.0
    }
}

impl AsRef<Btor> for BtorRef {
    fn as_ref(&self) -> &Btor {
        &self.0
    }
}

impl haybale::backend::SolverRef for BtorRef {
    type BV = BV;
    type Array = boolector::Array<Rc<Btor>>;

    fn new() -> Self {
        Self(<Rc<Btor> as haybale::backend::SolverRef>::new())
    }

    fn duplicate(&self) -> Self {
        Self(self.0.duplicate())
    }

    fn match_bv(&self, bv: &BV) -> Option<BV> {
        match bv {
            BV::Public(bv) => self.0.match_bv(bv).map(BV::Public),
            BV::Secret { .. } => Some(bv.clone()),
            BV::PartiallySecret { secret_mask, data, symbol, } => {
                self.0.match_bv(data).map(|matched_data| BV::PartiallySecret {
                    secret_mask: secret_mask.clone(),
                    data: matched_data,
                    symbol: symbol.clone(),
                })
            }
        }
    }

    fn match_array(&self, array: &boolector::Array<Rc<Btor>>) -> Option<boolector::Array<Rc<Btor>>> {
        self.0.match_array(array)
    }
}

impl From<BtorRef> for Rc<Btor> {
    fn from(btor: BtorRef) -> Rc<Btor> {
        btor.0
    }
}

impl From<Rc<Btor>> for BtorRef {
    fn from(rc: Rc<Btor>) -> BtorRef {
        BtorRef(rc)
    }
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
    /// `PartiallySecret` values have some secret and some not-secret bits.
    /// Currently we make just a best-effort attempt to handle these; for now,
    /// some functions may just fall back on treating the entire value as
    /// `Secret`.
    PartiallySecret {
        /// A vector the length of the `PartiallySecret` value's bitwidth, where each
        /// `true` indicates that the corresponding position in the `PartiallySecret`
        /// is secret, and likewise `false` indicates public.
        /// `secret_mask[0]` corresponds to the bit at position 0, which is the
        /// rightmost (least-significant) bit.
        secret_mask: Vec<bool>,
        /// A `BV`, which must have bitwidth exactly equal to the length of
        /// `secret_mask`.
        /// In positions where `secret_mask` is `false`, this gives the value of
        /// the public bits.
        /// In positions where `secret_mask` is `true`, the data here is invalid
        /// and should not be used for anything.
        data: boolector::BV<Rc<Btor>>,
        symbol: Option<String>,
    }
}

impl BV {
    pub fn is_secret(&self) -> bool {
        match self {
            BV::Public(_) => false,
            BV::Secret { .. } => true,
            BV::PartiallySecret { .. } => true,  // overapproximate. See notes on `BV::PartiallySecret`
        }
    }

    /// Gets the value out of a `BV::Public`, panicking if it is instead a `BV::Secret`
    pub fn as_public(&self) -> &boolector::BV<Rc<Btor>> {
        match self {
            BV::Public(bv) => bv,
            BV::Secret { .. } => panic!("as_public on a BV::Secret"),
            BV::PartiallySecret { .. } => panic!("as_public on a BV::PartiallySecret"),
        }
    }
}

macro_rules! impl_unop_as_functor {
    ($f:ident) => {
        fn $f(&self) -> Self {
            match self {
                BV::Public(bv) => BV::Public(bv.$f()),
                BV::Secret { btor, width, .. } => BV::Secret { btor: btor.clone(), width: *width, symbol: None }, // use this macro only for unary ops which don't change the bitwidth
                BV::PartiallySecret { data, .. } => BV::Secret { btor: data.get_btor().into(), width: data.get_width(), symbol: None }, // in general, any bit of the result may have been influenced by any other bit; we have no way of knowing if any bits remain untainted. Consider the inc() operation.
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
                BV::PartiallySecret { data, .. } => BV::Secret { btor: data.get_btor().into(), width: 1, symbol: None }, // in general, the result was probably influenced by one or more secret bits. Consider the redand() operation.
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
                // if one operand was even partially secret, we have no way of knowing in general which bits of the result may have been influenced by the secret input bits, so mark the entire output secret
                (BV::PartiallySecret { data, .. }, _) => BV::Secret { btor: data.get_btor().into(), width: data.get_width(), symbol: None },
                (_, BV::PartiallySecret { data, .. }) => BV::Secret { btor: data.get_btor().into(), width: data.get_width(), symbol: None },
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
                // if one operand was even partially secret, we must assume that in general the result may have been influenced by the secret input bits, so mark the output secret
                (BV::PartiallySecret { data, .. }, _) => BV::Secret { btor: data.get_btor().into(), width: 1, symbol: None },
                (_, BV::PartiallySecret { data, .. }) => BV::Secret { btor: data.get_btor().into(), width: 1, symbol: None },
            }
        }
    };
}

impl haybale::backend::BV for BV {
    type SolverRef = BtorRef;

    fn new(btor: BtorRef, width: u32, name: Option<&str>) -> Self {
        BV::Public(boolector::BV::new(btor.0, width, name))
    }
    fn from_bool(btor: BtorRef, b: bool) -> Self {
        BV::Public(boolector::BV::from_bool(btor.0, b))
    }
    fn from_i32(btor: BtorRef, i: i32, width: u32) -> Self {
        BV::Public(boolector::BV::from_i32(btor.0, i, width))
    }
    fn from_u32(btor: BtorRef, u: u32, width: u32) -> Self {
        BV::Public(boolector::BV::from_u32(btor.0, u, width))
    }
    fn from_i64(btor: BtorRef, i: i64, width: u32) -> Self {
        BV::Public(boolector::BV::from_i64(btor.0, i, width))
    }
    fn from_u64(btor: BtorRef, u: u64, width: u32) -> Self {
        BV::Public(boolector::BV::from_u64(btor.0, u, width))
    }
    fn zero(btor: BtorRef, width: u32) -> Self {
        BV::Public(boolector::BV::zero(btor.0, width))
    }
    fn one(btor: BtorRef, width: u32) -> Self {
        BV::Public(boolector::BV::one(btor.0, width))
    }
    fn ones(btor: BtorRef, width: u32) -> Self {
        BV::Public(boolector::BV::ones(btor.0, width))
    }
    fn from_binary_str(btor: BtorRef, bits: &str) -> Self {
        BV::Public(boolector::BV::from_binary_str(btor.0, bits))
    }
    fn from_dec_str(btor: BtorRef, num: &str, width: u32) -> Self {
        BV::Public(boolector::BV::from_dec_str(btor.0, num, width))
    }
    fn from_hex_str(btor: BtorRef, num: &str, width: u32) -> Self {
        BV::Public(boolector::BV::from_hex_str(btor.0, num, width))
    }
    fn as_binary_str(&self) -> Option<String> {
        match self {
            BV::Public(bv) => bv.as_binary_str(),
            BV::Secret { .. } => None,
            BV::PartiallySecret { .. } => None,
        }
    }
    fn as_u64(&self) -> Option<u64> {
        match self {
            BV::Public(bv) => bv.as_u64(),
            BV::Secret { .. } => None,
            BV::PartiallySecret { .. } => None,
        }
    }
    fn as_bool(&self) -> Option<bool> {
        match self {
            BV::Public(bv) => bv.as_bool(),
            BV::Secret { .. } => None,
            BV::PartiallySecret { .. } => None,
        }
    }
    fn get_a_solution(&self) -> Result<BVSolution> {
        match self {
            BV::Public(bv) => Ok(bv.get_a_solution()),
            BV::Secret { .. } => Err(Error::OtherError("Possible constant-time violation: get_a_solution() on a Secret value".to_owned())),
            BV::PartiallySecret { .. } => Err(Error::OtherError("Possible constant-time violation: get_a_solution() on a PartiallySecret value".to_owned())),
        }
    }
    fn get_solver(&self) -> Self::SolverRef {
       match self {
           BV::Public(bv) => bv.get_solver().into(),
           BV::Secret { btor, .. } => btor.clone(),
           BV::PartiallySecret { data, .. } => data.get_solver().into(),
       }
    }
    fn get_id(&self) -> i32 {
        match self {
            BV::Public(bv) => bv.get_id(),
            BV::Secret { .. } => panic!("get_id() on a Secret value"),
            BV::PartiallySecret { .. } => panic!("get_id() on a PartiallySecret value"),
        }
    }
    fn get_width(&self) -> u32 {
        match self {
            BV::Public(bv) => bv.get_width(),
            BV::Secret { width, .. } => *width,
            BV::PartiallySecret { secret_mask, data, .. } => {
                let width = data.get_width();
                assert_eq!(width, secret_mask.len() as u32);
                width
            },
        }
    }
    fn get_symbol(&self) -> Option<&str> {
        match self {
            BV::Public(bv) => bv.get_symbol(),
            BV::Secret { symbol, .. } => symbol.as_deref(),
            BV::PartiallySecret { symbol, .. } => symbol.as_deref(),
        }
    }
    fn set_symbol(&mut self, symbol: Option<&str>) {
        match self {
            BV::Public(bv) => bv.set_symbol(symbol),
            BV::Secret { btor, width, .. } => {
                *self = BV::Secret {
                    btor: btor.clone(),
                    width: *width,
                    symbol: symbol.map(|s| s.into()),
                }
            },
            BV::PartiallySecret { secret_mask, data, .. } => {
                *self = BV::PartiallySecret {
                    secret_mask: secret_mask.clone(),
                    data: data.clone(),
                    symbol: symbol.map(|s| s.into()),
                }
            },
        }
    }
    fn is_const(&self) -> bool {
        match self {
            BV::Public(bv) => bv.is_const(),
            BV::Secret { .. } => false,
            BV::PartiallySecret { .. } => false,
        }
    }
    fn has_same_width(&self, other: &Self) -> bool {
        match (self, other) {
            (BV::Public(bv), BV::Public(other)) => bv.has_same_width(other),
            _ => self.get_width() == other.get_width(),
        }
    }
    fn assert(&self) -> Result<()> {
        match self {
            BV::Public(bv) => {
                bv.assert();
                Ok(())
            },
            BV::Secret { .. } | BV::PartiallySecret { .. } => {
                // `Secret` values influencing a path constraint means they influenced a control flow decision
                Err(Error::OtherError("Constant-time violation: control-flow may be influenced by secret data".to_owned()))
            },
        }
    }
    fn is_failed_assumption(&self) -> bool {
        match self {
            BV::Public(bv) => bv.is_failed_assumption(),
            BV::Secret { .. } => false,
            BV::PartiallySecret { .. } => false,
        }
    }

    impl_binop_as_functor_return_bool!(_eq);
    impl_binop_as_functor_return_bool!(_ne);
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
    impl_binop_as_functor_return_bool!(uaddo);
    impl_binop_as_functor_return_bool!(saddo);
    impl_binop_as_functor_return_bool!(usubo);
    impl_binop_as_functor_return_bool!(ssubo);
    impl_binop_as_functor_return_bool!(umulo);
    impl_binop_as_functor_return_bool!(smulo);
    impl_binop_as_functor_return_bool!(sdivo);
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

    // we could just use the default implementations for these
    // saturating-arithmetic operations, but the functor implementation will be
    // slightly more efficient
    impl_binop_as_functor!(uadds);
    impl_binop_as_functor!(sadds);
    impl_binop_as_functor!(usubs);
    impl_binop_as_functor!(ssubs);

    fn zext(&self, i: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.zext(i)),
            BV::Secret { btor, width, .. } => BV::PartiallySecret {
                secret_mask: itertools::repeat_n(true, *width as usize).chain(itertools::repeat_n(false, i as usize)).collect(),
                data: boolector::BV::zero(btor.clone().into(), *width + i),
                symbol: None,
            },
            BV::PartiallySecret { secret_mask, data, .. } => BV::PartiallySecret {
                secret_mask: {
                    let mut new_mask = secret_mask.clone();
                    new_mask.resize(secret_mask.len() + (i as usize), false);
                    new_mask
                },
                data: data.zext(i),
                symbol: None,
            },
        }
    }
    fn sext(&self, i: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.sext(i)),
            BV::Secret { btor, width, .. } => BV::Secret { btor: btor.clone(), width: *width + i, symbol: None },
            BV::PartiallySecret { secret_mask, data, .. } => BV::PartiallySecret {
                secret_mask: {
                    let mut new_mask = secret_mask.clone();
                    let new_length = secret_mask.len() + (i as usize);
                    match secret_mask.last() {
                        Some(true) => {
                            // sign bit is secret, so extend with secret bits
                            new_mask.resize(new_length, true);
                        },
                        Some(false) => {
                            // sign bit is public, so extend with public bits
                            new_mask.resize(new_length, false);
                        },
                        None => {
                            panic!("sign-extension operation on a bitvector with width 0");
                        },
                    }
                    new_mask
                },
                data: data.sext(i),
                symbol: None,
            },
        }
    }
    fn slice(&self, high: u32, low: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.slice(high, low)),
            BV::Secret { btor, .. } => BV::Secret { btor: btor.clone(), width: high - low + 1, symbol: None },
            BV::PartiallySecret { secret_mask, data, .. } => {
                let new_mask = secret_mask[low as usize ..= high as usize].to_vec();
                if new_mask.iter().all(|b| *b) {
                    // result is entirely secret
                    BV::Secret { btor: data.get_btor().into(), width: high - low + 1, symbol: None }
                } else if !new_mask.iter().any(|b| *b) {
                    // result is entirely public
                    BV::Public(data.slice(high, low))
                } else {
                    // result is still mixed public and secret
                    BV::PartiallySecret {
                        secret_mask: new_mask,
                        data: data.slice(high, low),
                        symbol: None,
                    }
                }
            },
        }
    }
    fn concat(&self, other: &Self) -> Self {
        match (self, other) {
            (BV::Public(bv), BV::Public(other)) => BV::Public(bv.concat(other)),
            (BV::Secret { btor, width, .. }, BV::Public(public)) => BV::PartiallySecret {
                // secret in the high-order bits, public in the low-order bits
                secret_mask: itertools::repeat_n(false, public.get_width() as usize)
                    .chain(itertools::repeat_n(true, *width as usize))
                    .collect(),
                data: boolector::BV::zero(btor.clone().into(), *width).concat(public),
                symbol: None,
            },
            (BV::Public(public), BV::Secret { btor, width, .. }) => BV::PartiallySecret {
                // secret in the low-order bits, public in the high-order bits
                secret_mask: itertools::repeat_n(true, *width as usize)
                    .chain(itertools::repeat_n(false, public.get_width() as usize))
                    .collect(),
                data: public.concat(&boolector::BV::zero(btor.clone().into(), *width)),
                symbol: None,
            },
            (BV::Secret { btor: self_btor, width: self_width, .. }, BV::Secret { btor: other_btor, width: other_width, .. }) => {
                assert_eq!(self_btor, other_btor);
                BV::Secret {
                    btor: self_btor.clone(),
                    width: self_width + other_width,
                    symbol: None,
                }
            },
            (BV::PartiallySecret { secret_mask, data, .. }, BV::Public(public)) => BV::PartiallySecret {
                secret_mask: {
                    let mut mask = Vec::with_capacity(secret_mask.len() + public.get_width() as usize);
                    mask.extend(itertools::repeat_n(false, public.get_width() as usize));
                    mask.extend_from_slice(&secret_mask[..]);
                    mask
                },
                data: data.concat(public),
                symbol: None,
            },
            (BV::Public(public), BV::PartiallySecret { secret_mask, data, .. }) => BV::PartiallySecret {
                secret_mask: {
                    let mut mask = Vec::with_capacity(secret_mask.len() + public.get_width() as usize);
                    mask.extend_from_slice(&secret_mask[..]);
                    mask.extend(itertools::repeat_n(false, public.get_width() as usize));
                    mask
                },
                data: public.concat(data),
                symbol: None,
            },
            (BV::PartiallySecret { secret_mask, data, .. }, BV::Secret { btor, width, .. }) => BV::PartiallySecret {
                secret_mask: {
                    let mut mask = Vec::with_capacity(secret_mask.len() + *width as usize);
                    mask.extend(itertools::repeat_n(true, *width as usize));
                    mask.extend_from_slice(&secret_mask[..]);
                    mask
                },
                data: data.concat(&boolector::BV::zero(btor.clone().into(), *width)),
                symbol: None,
            },
            (BV::Secret { btor, width, .. }, BV::PartiallySecret { secret_mask, data, .. }) => BV::PartiallySecret {
                secret_mask: {
                    let mut mask = Vec::with_capacity(secret_mask.len() + *width as usize);
                    mask.extend_from_slice(&secret_mask[..]);
                    mask.extend(itertools::repeat_n(true, *width as usize));
                    mask
                },
                data: boolector::BV::zero(btor.clone().into(), *width).concat(&data),
                symbol: None,
            },
            (BV::PartiallySecret { secret_mask: self_mask, data: self_data, .. }, BV::PartiallySecret { secret_mask: other_mask, data: other_data, .. } ) => BV::PartiallySecret {
                secret_mask: {
                    let mut mask = other_mask.clone();
                    mask.extend_from_slice(&self_mask[..]);
                    mask
                },
                data: self_data.concat(&other_data),
                symbol: None,
            },
        }
    }
    fn repeat(&self, n: u32) -> Self {
        match self {
            BV::Public(bv) => BV::Public(bv.repeat(n)),
            BV::Secret { btor, width, .. } => BV::Secret { btor: btor.clone(), width: width * n, symbol: None },
            BV::PartiallySecret { secret_mask, data, .. } => BV::PartiallySecret {
                secret_mask: secret_mask.repeat(n as usize),
                data: data.repeat(n),
                symbol: None,
            }
        }
    }

    impl_binop_as_functor!(iff);
    impl_binop_as_functor!(implies);

    fn cond_bv(&self, truebv: &Self, falsebv: &Self) -> Self {
        let dest_width = {
            let width = truebv.get_width();
            assert_eq!(width, falsebv.get_width());
            width
        };
        if self.is_secret() {
            warn!("'select' operation with a secret condition and {}-bit operands. This may not be constant-time, depending on the target architecture and other factors.", dest_width);
        }
        match (self, truebv, falsebv) {
            (BV::Public(bv), BV::Public(truebv), BV::Public(falsebv))
                => BV::Public(bv.cond_bv(truebv, falsebv)),
            (BV::Secret { btor, .. }, _, _)
                => BV::Secret { btor: btor.clone(), width: truebv.get_width(), symbol: None },
            (_, BV::Secret { btor, width, .. }, _)
                => BV::Secret { btor: btor.clone(), width: *width, symbol: None },
            (_, _, BV::Secret { btor, width, .. })
                => BV::Secret { btor: btor.clone(), width: *width, symbol: None },
            (BV::PartiallySecret { data, .. }, _, _)
                => BV::Secret { btor: data.get_btor().clone().into(), width: truebv.get_width(), symbol: None },
            (BV::Public(cond), BV::PartiallySecret { secret_mask, data, .. }, BV::Public(falsebv))
                => BV::PartiallySecret { secret_mask: secret_mask.clone(), data: cond.cond_bv(data, falsebv), symbol: None },
            (BV::Public(cond), BV::Public(truebv), BV::PartiallySecret { secret_mask, data, .. })
                => BV::PartiallySecret { secret_mask: secret_mask.clone(), data: cond.cond_bv(truebv, data), symbol: None },
            (BV::Public(cond), BV::PartiallySecret { secret_mask: true_mask, data: true_data, .. }, BV::PartiallySecret { secret_mask: false_mask, data: false_data, .. })
                => BV::PartiallySecret {
                    secret_mask: true_mask.iter().zip(false_mask.iter()).map(|(a,b)| *a || *b).collect(),
                    data: cond.cond_bv(true_data, false_data),
                    symbol: None,
                },
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Memory {
    btor: BtorRef,
    /// This memory holds the actual data
    mem: haybale::simple_memory::Memory,
    /// This memory is a bitmap, with each bit indicating if the corresponding bit of `mem` is secret or not (1 for secret, 0 for public)
    shadow_mem: haybale::simple_memory::Memory,
}
// note on the above: we use `haybale::simple_memory` over `haybale::memory`
// because, at least in one relevant case, it can speed up an analysis from
// the order of hours to under a minute.
//
// The speedup is due to better ability to get "true constants" from memory
// reads, and thus avoiding painful model generation calls, e.g. when needing to
// resolve constant function pointers.
//
// A more general performance comparison across a wide variety of typical
// workloads is probably called for.

impl haybale::backend::Memory for Memory {
    type SolverRef = BtorRef;
    type Index = BV;
    type Value = BV;

    fn new_uninitialized(btor: BtorRef, null_detection: bool, name: Option<&str>) -> Self {
        assert_ne!(name, Some("shadow_mem"), "can't use {:?} as a name for a secret::Memory, as we reserve that name", name);
        Self {
            mem: haybale::backend::Memory::new_uninitialized(btor.0.clone(), null_detection, name),
            shadow_mem: haybale::backend::Memory::new_zero_initialized(btor.0.clone(), null_detection, Some("shadow_mem")), // shadow bits are zero-initialized (all public) even though the memory contents are uninitialized
            btor,  // out of order so it can be used above but moved in here
        }
    }
    fn new_zero_initialized(btor: BtorRef, null_detection: bool, name: Option<&str>) -> Self {
        assert_ne!(name, Some("shadow_mem"), "can't use {:?} as a name for a secret::Memory, as we reserve that name", name);
        Self {
            mem: haybale::backend::Memory::new_zero_initialized(btor.0.clone(), null_detection, name),
            shadow_mem: haybale::backend::Memory::new_zero_initialized(btor.0.clone(), null_detection, Some("shadow_mem")), // initialize to all public zeroes
            btor,  // out of order so it can be used above but moved in here
        }
    }
    fn read(&self, index: &Self::Index, bits: u32) -> Result<Self::Value> {
        match index {
            BV::Public(index) => {
                use haybale::solver_utils::{bvs_must_be_equal, bvs_can_be_equal, max_possible_solution_for_bv_as_binary_str};
                let shadow_cell = haybale::backend::Memory::read(&self.shadow_mem, index, bits)?;
                // In Boolector, (at least when this comment was originally written) reads
                // on a constant array that return the default value are nonetheless not
                // constant (they are merely constrained to be equal to the default value).
                // So, we actually need to do a solve here.
                //
                // However, in the common case the shadow value is either all zeroes (all
                // public) or all ones (all secret). This means that usually we can get away
                // with using faster `bvs_must_be_equal` / `bvs_can_be_equal` checks rather
                // than a slow `get_possible_solutions_for_bv()` check.
                let rc: Rc<Btor> = self.btor.clone().into();
                let all_zeroes = boolector::BV::zero(rc.clone(), shadow_cell.get_width());
                let all_ones = boolector::BV::ones(rc.clone(), shadow_cell.get_width());
                if bvs_must_be_equal(&rc, &shadow_cell, &all_zeroes)? {
                    // the bits are all public
                    haybale::backend::Memory::read(&self.mem, index, bits).map(BV::Public)
                } else if bvs_can_be_equal(&rc, &shadow_cell, &all_ones)? {
                    // the bits all _can_ be secret. And any bit that _can_ be
                    // secret, we mark as secret (following the worst case).
                    // (Non-constant secrecy bits means that the bits could be
                    // secret or not, depending on the values of other variables.
                    // This can happen, e.g., when reading from a symbolic address
                    // that could point to either secret or public data.)
                    Ok(BV::Secret { btor: self.btor.clone(), width: bits, symbol: None })
                } else {
                    // Some of the bits are secret, others are public.
                    // We get a mask of which can be secret by finding the
                    // (unsigned) maximum value of the shadow cell; this will
                    // have 1s everywhere possible.
                    // (We assume that the secrecy of each bit is independent;
                    // that is, that there is not a situation where a bit could
                    // be secret, but only if some other bit isn't.)
                    // Any bits that have 0s in that mask must be public.
                    let secret_mask_as_str = max_possible_solution_for_bv_as_binary_str(rc, &shadow_cell)?.ok_or(Error::Unsat)?;
                    let secret_mask = secret_mask_as_str.chars().rev().map(|c| c == '1').collect();
                    Ok(BV::PartiallySecret {
                        secret_mask,
                        data: haybale::backend::Memory::read(&self.mem, index, bits)?,
                        symbol: None,
                    })
                }
            },
            BV::Secret { .. } | BV::PartiallySecret { .. } => {
                Err(Error::OtherError("Constant-time violation: memory read on an address which can be influenced by secret data".to_owned()))
            }
        }
    }
    fn write(&mut self, index: &Self::Index, value: Self::Value) -> Result<()> {
        match index {
            BV::Public(index) => {
                if !index.is_const() {
                    warn!("Memory write with a non-constant address {:?}", index);
                }
                match value {
                    BV::Public(value) => {
                        let all_zeroes = boolector::BV::zero(self.btor.clone().into(), value.get_width());
                        haybale::backend::Memory::write(&mut self.shadow_mem, index, all_zeroes)?; // we are writing a public value to these bits
                        haybale::backend::Memory::write(&mut self.mem, index, value)?;
                        Ok(())
                    },
                    BV::Secret { btor, width, .. } => {
                        let all_ones = boolector::BV::ones(btor.clone().into(), width);
                        haybale::backend::Memory::write(&mut self.shadow_mem, index, all_ones)?; // we are writing a secret value to these bits
                        // we don't write anything to self.mem, because the value of its secret bits doesn't matter
                        Ok(())
                    },
                    BV::PartiallySecret { secret_mask, data, .. } => {
                        let shadow_mem_string: String = secret_mask.iter().map(|b| if *b { "1" } else { "0" }).rev().collect();
                        let shadow_mem_bv = boolector::BV::from_binary_str(self.btor.clone().into(), &shadow_mem_string);
                        haybale::backend::Memory::write(&mut self.shadow_mem, index, shadow_mem_bv)?;
                        haybale::backend::Memory::write(&mut self.mem, index, data)?;
                        Ok(())
                    },
                }
            },
            BV::Secret { .. } | BV::PartiallySecret { .. } => {
                Err(Error::OtherError("Constant-time violation: memory write on an address which can be influenced by secret data".to_owned()))
            },
        }
    }
    fn get_solver(&self) -> BtorRef {
        self.btor.clone()
    }
    fn change_solver(&mut self, new_solver: BtorRef) {
        self.mem.change_solver(new_solver.0.clone());
        self.shadow_mem.change_solver(new_solver.0.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use haybale::backend::{BV, Memory, SolverRef};
    use haybale::solver_utils::bvs_must_be_equal;

    #[test]
    fn arithmetic() {
        let btor = BtorRef::new();
        let public = super::BV::new(btor.clone(), 32, Some("public"));
        let public_const = super::BV::from_u32(btor.clone(), 13, 32);
        let secret = super::BV::Secret { btor: btor.clone(), width: 32, symbol: None };

        // unary ops, exemplified by inc()
        assert!(!public.inc().is_secret());
        assert!(secret.inc().is_secret());

        // binary ops, exemplified by add()
        assert!(!public.add(&public_const).is_secret());
        assert!(public.add(&secret).is_secret());
        assert!(secret.add(&public).is_secret());
        assert!(secret.add(&secret).is_secret());

        // comparison ops, exemplified by ugt()
        assert!(!public.ugt(&public_const).is_secret());
        assert!(public.ugt(&secret).is_secret());
        assert!(secret.ugt(&public).is_secret());
        assert!(secret.ugt(&secret).is_secret());
    }

    #[test]
    fn slice_and_concat() {
        let btor = BtorRef::new();
        let public = super::BV::new(btor.clone(), 32, Some("public"));
        let public_const = super::BV::from_u32(btor.clone(), 13, 32);
        let secret = super::BV::Secret { btor: btor.clone(), width: 32, symbol: None };

        assert!(!public.slice(31, 0).is_secret());
        assert!(!public.slice(17, 12).is_secret());
        assert!(secret.slice(31, 0).is_secret());
        assert!(secret.slice(17, 12).is_secret());

        assert!(!public.concat(&public_const).is_secret());
        assert!(secret.concat(&public_const).is_secret());
        assert!(public.concat(&secret).is_secret());
        assert!(secret.concat(&secret).is_secret());

        let secret_low = public.concat(&secret);
        let secret_high = secret.concat(&public);
        assert!(secret_low.is_secret());
        assert!(secret_high.is_secret());
        assert!(!secret_low.slice(63, 32).is_secret());
        assert!(secret_low.slice(31, 0).is_secret());
        assert!(secret_high.slice(63, 32).is_secret());
        assert!(!secret_high.slice(31, 0).is_secret());
        assert!(!secret_high.slice(0, 0).is_secret());
        assert!(secret_low.slice(0, 0).is_secret());
        assert!(secret_low.slice(40, 20).is_secret());
        assert!(secret_high.slice(40, 20).is_secret());

        let secret_middle = public.concat(&secret_high);
        assert!(secret_middle.is_secret());
        assert!(!secret_middle.slice(10, 0).is_secret());
        assert!(!secret_middle.slice(90, 80).is_secret());
        assert!(secret_middle.slice(50, 40).is_secret());

        let secret_ends = secret_high.concat(&secret);
        assert!(secret_ends.is_secret());
        assert!(secret_ends.slice(10, 0).is_secret());
        assert!(secret_ends.slice(90, 80).is_secret());
        assert!(!secret_ends.slice(50, 40).is_secret());

        // concatenate two PartiallySecret bvs
        let secret_ends = secret_high.concat(&secret_low);
        assert!(secret_ends.is_secret());
        assert!(secret_ends.slice(10, 0).is_secret());
        assert!(secret_ends.slice(120, 110).is_secret());
        assert!(!secret_ends.slice(90, 80).is_secret());
        assert!(!secret_ends.slice(50, 40).is_secret());
    }

    #[test]
    fn extensions() {
        let btor = BtorRef::new();
        let public = super::BV::new(btor.clone(), 32, Some("public"));
        let secret = super::BV::Secret { btor: btor.clone(), width: 32, symbol: None };

        assert!(!public.zext(16).is_secret());
        assert!(secret.zext(16).is_secret());
        assert!(!public.sext(16).is_secret());
        assert!(secret.sext(16).is_secret());

        // zero-extending a secret, the zeroes are public
        assert!(!secret.zext(16).slice(47, 32).is_secret());
        // sign-extending a secret, the extension bits are secret bc they depend on a secret bit
        assert!(secret.sext(16).slice(47, 32).is_secret());

        let secret_high = secret.concat(&public);
        let secret_low = public.concat(&secret);

        // zero-extending a mixed value, the zeroes are public
        assert!(!secret_low.zext(16).slice(79, 64).is_secret());
        assert!(!secret_high.zext(16).slice(79, 64).is_secret());
        // sign-extending a mixed value, extension bits are secret iff sign bit is secret
        assert!(!secret_low.sext(16).slice(79, 64).is_secret());
        assert!(secret_high.sext(16).slice(79, 64).is_secret());
    }

    #[test]
    fn read_and_write() {
        let btor = BtorRef::new();
        let mut mem = super::Memory::new_uninitialized(btor.clone(), false, Some("mem"));
        let initialized_mem = super::Memory::new_zero_initialized(btor.clone(), false, Some("init_mem"));
        let addr = super::BV::from_u64(btor.clone(), 0x1000, 64);
        let addr_plus_two = addr.add(&super::BV::from_u32(btor.clone(), 2, 64));
        let secret = super::BV::Secret { btor: btor.clone(), width: 64, symbol: Some("secret".into()) };
        let secret_32bits = super::BV::Secret { btor: btor.clone(), width: 32, symbol: Some("smaller_secret".into()) };
        let mixed = super::BV::from_u32(btor.clone(), 321, 64).concat(&secret_32bits);

        // uninitialized values are public
        let data = mem.read(&addr, 64).expect("Reading memory at a constant address shouldn't be a violation");
        assert!(!data.is_secret());

        // initialized values are public
        let data = initialized_mem.read(&addr, 64).expect("Reading memory at a constant address shouldn't be a violation");
        assert!(!data.is_secret());

        // read at secret or mixed address is a violation
        assert!(mem.read(&secret, 64).is_err());
        assert!(mem.read(&mixed, 64).is_err());

        // write a public value, get a public value back
        let value = super::BV::from_u32(btor.clone(), 577, 64);
        mem.write(&addr, value.clone()).expect("Writing memory at a constant address shouldn't be a violation");
        let data = mem.read(&addr, 64).expect("Reading memory at a constant address shouldn't be a violation");
        assert!(!data.is_secret());
        assert!(bvs_must_be_equal(&btor, &value, &data).unwrap());

        // write a secret value, get a secret value back
        mem.write(&addr, secret.clone()).expect("Writing memory at a constant address shouldn't be a violation");
        let data = mem.read(&addr, 64).expect("Reading memory at a constant address shouldn't be a violation");
        assert!(data.is_secret());

        // write a mixed value, get a mixed value back
        mem.write(&addr, mixed.clone()).expect("Writing memory at a constant address shouldn't be a violation");
        let data = mem.read(&addr, 64).expect("Reading memory at a constant address shouldn't be a violation");
        assert!(!data.slice(63, 32).is_secret());
        assert!(data.slice(31, 0).is_secret());

        // overwrite a large public value with a few secret bits, get the appropriate mixed value back
        mem.write(&addr, value.clone()).expect("Writing memory at a constant address shouldn't be a violation");
        mem.write(&addr_plus_two, secret_32bits.clone()).expect("Writing memory at a constant address shouldn't be a violation");
        let data = mem.read(&addr, 64).expect("Reading memory at a constant address shouldn't be a violation");
        assert!(data.is_secret());
        assert!(data.slice(30, 30).is_secret());
        assert!(data.slice(40, 40).is_secret());
        assert!(!data.slice(60, 60).is_secret());
        assert!(!data.slice(2, 2).is_secret());

        // overwrite a large secret value with a few public bits, get the appropriate mixed value back
        mem.write(&addr, secret.clone()).expect("Writing memory at a constant address shouldn't be a violation");
        let public_32bits = super::BV::from_u32(btor.clone(), 4678, 32);
        mem.write(&addr_plus_two, public_32bits.clone()).expect("Writing memory at a constant address shouldn't be a violation");
        let data = mem.read(&addr, 64).expect("Reading memory at a constant address shouldn't be a violation");
        assert!(data.is_secret());
        assert!(!data.slice(30, 30).is_secret());
        assert!(!data.slice(40, 40).is_secret());
        assert!(data.slice(60, 60).is_secret());
        assert!(data.slice(2, 2).is_secret());

        // read at a fully symbolic addr, should result in a secret as there are some secret bits in the mem right now
        let symbolic = super::BV::new(btor.clone(), 64, Some("symbolic"));
        let data = mem.read(&symbolic, 8).expect("Reading memory at a public address shouldn't be a violation");
        assert!(data.is_secret());

        // write a small secret to some address in a range; reading an element of that range gives secret
        let range_bottom = super::BV::from_u64(btor.clone(), 0x20000, 64);
        let range_top = range_bottom.add(&super::BV::from_u64(btor.clone(), 0x1000, 64));
        let secret_8bits = super::BV::Secret { btor: btor.clone(), width: 8, symbol: Some("secret_8bits".into()) };
        let symbolic = super::BV::new(btor.clone(), 64, Some("symbolic_range"));
        symbolic.ugte(&range_bottom).assert().unwrap();
        symbolic.ult(&range_top).assert().unwrap();
        mem.write(&symbolic, secret_8bits.clone()).expect("Writing memory at a public address shouldn't be a violation");
        let somewhere_in_middle_of_range = range_bottom.add(&super::BV::from_u64(btor.clone(), 100, 64));
        let data = mem.read(&somewhere_in_middle_of_range, 8).expect("Reading memory at a public address shouldn't be a violation");
        assert!(data.is_secret());
    }
}
