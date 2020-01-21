use haybale::{layout, Project};
use lazy_static::lazy_static;
use llvm_ir::Type;
use log::warn;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Mutex;

/// An abstract description of a value: if it is public or not, if it is a
/// pointer or not, does it point to data that is public/secret, maybe it's a
/// struct with some public and some secret fields, etc.
///
/// Unlike `AbstractData`, these may never be "underspecified" - that is, they
/// must be a complete description of the data structure.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum CompleteAbstractData {
    /// A public value, of the given size in bits. The `AbstractValue` is used to
    /// indicate whether the value should have a particular concrete value, be
    /// unconstrained, etc.
    ///
    /// This may be used for either a non-pointer value, or for a pointer value
    /// if you want to specify the exact numerical value of the pointer (e.g. `NULL`).
    PublicValue { bits: usize, value: AbstractValue },

    /// A secret value (pointer or non-pointer, doesn't matter) of the given size in bits
    Secret { bits: usize },

    /// A (first-class) array of values
    Array { element_type: Box<Self>, num_elements: usize },

    /// A (first-class) structure of values
    Struct { name: String, elements: Vec<Self> },

    /// A (public) pointer to something - another value, an array, etc
    PublicPointerTo {
        /// Description of the thing being pointed to
        pointee: Box<Self>,
        /// If `false`, the pointer must point to the pointee; if `true`,
        /// it may either point to the pointee or be `NULL`
        maybe_null: bool,
    },

    /// A (public) pointer to the LLVM `Function` with the given name
    PublicPointerToFunction(String),

    /// A (public) pointer to the _hook_ registered for the given name
    PublicPointerToHook(String),

    /// A (public) pointer to this struct itself. E.g., in the C code
    /// ```c
    /// struct Foo {
    ///     int x;
    ///     Foo* f;
    /// };
    /// ```
    /// you could use this for `Foo* f` to indicate it should point to
    /// this exact `Foo` itself.
    PublicPointerToSelf,

    /// A (public) pointer to this struct's parent. E.g., in the C code
    /// ```c
    /// struct Foo {
    ///     int x;
    ///     Bar* bar1;
    ///     Bar* bar2;
    ///     ...
    /// };
    ///
    /// struct Bar {
    ///     int y;
    ///     Foo* parent;  // pointer to the Foo containing this Bar
    /// };
    /// ```
    /// you could use this for `Foo* parent` to indicate it should point to the
    /// `Foo` containing this `Bar`.
    ///
    /// If the `Option` is `Some`, then if the parent is not the correct type
    /// (or if there is no parent, i.e., we are directly initializing this)
    /// then pointer to the given `CompleteAbstractData` instead
    PublicPointerToParentOr(Option<Box<CompleteAbstractData>>),

    /// When C code uses `void*`, this often becomes `i8*` in LLVM. However,
    /// within Pitchfork, we may want to specify some type other than `i8*` for
    /// the purposes of allocating and analyzing the data behind the `void*`.
    ///
    /// This says to use the provided `CompleteAbstractData` even though the LLVM
    /// type is `i8`.
    ///
    /// If the optional `llvm_struct_name` is included, it will lookup that struct's
    /// type and check against that.  Otherwise, no typechecking will be performed
    /// and the provided `CompleteAbstractData` will be assumed correct.
    VoidOverride { llvm_struct_name: Option<String>, data: Box<Self> },

    /// Use the given `data`, even though it may not match the LLVM type.
    /// It still needs to be the same size (number of bits) as the LLVM type.
    /// For instance, you could specify that some LLVM pointer-size integer
    /// should actually be initialized to have a pointer value and point to some
    /// specified data.
    ///
    /// To override a `void*` type, see `VoidOverride` - and this probably won't
    /// work for that anyways because of the same-size restriction. See comments
    /// on `VoidOverride`.
    SameSizeOverride { data: Box<Self> },

    /// Use the given `data`, but also (during initialization) add a watchpoint
    /// with the given `name` to the `State` covering the memory region it
    /// occupies.
    WithWatchpoint { name: String, data: Box<Self> },
}

impl CompleteAbstractData {
    /// an 8-bit public value
    pub fn pub_i8(value: AbstractValue) -> Self {
        Self::PublicValue { bits: 8, value }
    }

    /// a 16-bit public value
    pub fn pub_i16(value: AbstractValue) -> Self {
        Self::PublicValue { bits: 16, value }
    }

    /// a 32-bit public value
    pub fn pub_i32(value: AbstractValue) -> Self {
        Self::PublicValue { bits: 32, value }
    }

    /// a 64-bit public value
    pub fn pub_i64(value: AbstractValue) -> Self {
        Self::PublicValue { bits: 64, value }
    }

    /// a public value with the given number of bits
    pub fn pub_integer(bits: usize, value: AbstractValue) -> Self {
        Self::PublicValue { bits, value }
    }

    /// an 8-bit secret value
    pub fn sec_i8() -> Self {
        Self::Secret { bits: 8 }
    }

    /// a 16-bit secret value
    pub fn sec_i16() -> Self {
        Self::Secret { bits: 16 }
    }

    /// a 32-bit secret value
    pub fn sec_i32() -> Self {
        Self::Secret { bits: 32 }
    }

    /// a 64-bit secret value
    pub fn sec_i64() -> Self {
        Self::Secret { bits: 64 }
    }

    /// a secret value with the given number of bits
    pub fn sec_integer(bits: usize) -> Self {
        Self::Secret { bits }
    }

    /// a (public) pointer to something - another value, an array, etc
    pub fn pub_pointer_to(data: Self) -> Self {
        Self::PublicPointerTo { pointee: Box::new(data), maybe_null: false }
    }

    /// A (public) pointer which may either point to the given data or be `NULL`
    pub fn pub_maybe_null_pointer_to(data: Self) -> Self {
        Self::PublicPointerTo { pointee: Box::new(data), maybe_null: true }
    }

    /// a (public) pointer to the LLVM `Function` with the given name
    pub fn pub_pointer_to_func(funcname: impl Into<String>) -> Self {
        Self::PublicPointerToFunction(funcname.into())
    }

    /// a (public) pointer to the _hook_ registered for the given name
    pub fn pub_pointer_to_hook(funcname: impl Into<String>) -> Self {
        Self::PublicPointerToHook(funcname.into())
    }

    /// a (public) pointer to this struct itself; see comments on
    /// `CompleteAbstractData::PublicPointerToSelf`
    pub fn pub_pointer_to_self() -> Self {
        Self::PublicPointerToSelf
    }

    /// A (public) pointer to this struct's parent. E.g., in the C code
    /// ```c
    /// struct Foo {
    ///     int x;
    ///     Bar* bar1;
    ///     Bar* bar2;
    ///     ...
    /// };
    ///
    /// struct Bar {
    ///     int y;
    ///     Foo* parent;  // pointer to the Foo containing this Bar
    /// };
    /// ```
    /// you could use this for `Foo* parent` to indicate it should point to the
    /// `Foo` containing this `Bar`.
    pub fn pub_pointer_to_parent() -> Self {
        Self::PublicPointerToParentOr(None)
    }

    /// Like `pub_pointer_to_parent()`, but if the parent is not the correct type
    /// (or if there is no parent, i.e., we are directly initializing this) then
    /// pointer to the given `CompleteAbstractData` instead
    pub fn pub_pointer_to_parent_or(data: Self) -> Self {
        Self::PublicPointerToParentOr(Some(Box::new(data)))
    }

    /// A (first-class) array of values
    pub fn array_of(element_type: Self, num_elements: usize) -> Self {
        Self::Array { element_type: Box::new(element_type), num_elements }
    }

    /// A (first-class) structure of values.  Name used only for debugging purposes, need not match the (mangled) LLVM struct name.
    ///
    /// (`_struct` used instead of `struct` to avoid collision with the Rust keyword)
    pub fn _struct(name: impl Into<String>, elements: impl IntoIterator<Item = Self>) -> Self {
        Self::Struct { name: name.into(), elements: elements.into_iter().collect() }
    }

    /// A (public) pointer which may point anywhere, including being `NULL`
    pub fn unconstrained_pointer() -> Self {
        Self::PublicValue { bits: Self::POINTER_SIZE_BITS, value: AbstractValue::Unconstrained }
    }

    /// When C code uses `void*`, this often becomes `i8*` in LLVM. However,
    /// within Pitchfork, we may want to specify some type other than `i8*` for
    /// the purposes of allocating and analyzing the data behind the `void*`.
    ///
    /// This says to use the provided `CompleteAbstractData` even though the LLVM
    /// type is `i8`.
    ///
    /// If the optional `llvm_struct_name` is included, it will lookup that struct's
    /// type and check against that.  Otherwise, no typechecking will be performed
    /// and the provided `CompleteAbstractData` will be assumed correct.
    pub fn void_override(llvm_struct_name: Option<&str>, data: Self) -> Self {
        Self::VoidOverride { llvm_struct_name: llvm_struct_name.map(Into::into), data: Box::new(data) }
    }

    /// Use the given `data`, even though it may not match the LLVM type.
    /// It still needs to be the same size (number of bits) as the LLVM type.
    /// For instance, you could specify that some LLVM pointer-size integer
    /// should actually be initialized to have a pointer value and point to some
    /// specified data.
    ///
    /// To override a `void*` type, see `void_override` - and this probably won't
    /// work for that anyways because of the same-size restriction. See comments
    /// on `void_override`.
    pub fn same_size_override(data: Self) -> Self {
        Self::SameSizeOverride { data: Box::new(data) }
    }

    /// Use the given `data`, but also (during initialization) add a watchpoint
    /// with the given `name` to the `State` covering the memory region it
    /// occupies.
    pub fn with_watchpoint(name: impl Into<String>, data: Self) -> Self {
        Self::WithWatchpoint { name: name.into(), data: Box::new(data) }
    }
}

impl CompleteAbstractData {
    pub const POINTER_SIZE_BITS: usize = 64;

    /// Get the size of the `CompleteAbstractData`, in bits
    pub fn size_in_bits(&self) -> usize {
        match self {
            Self::PublicValue { bits, .. } => *bits,
            Self::Array { element_type, num_elements } => element_type.size_in_bits() * num_elements,
            Self::Struct { elements, .. } => elements.iter().map(Self::size_in_bits).sum(),
            Self::PublicPointerTo { .. } => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToFunction(_) => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToHook(_) => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToSelf => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToParentOr(_) => Self::POINTER_SIZE_BITS,
            Self::Secret { bits } => *bits,
            Self::VoidOverride { data, .. } => data.size_in_bits(),
            Self::SameSizeOverride { data, .. } => data.size_in_bits(),
            Self::WithWatchpoint { data, .. } => data.size_in_bits(),
        }
    }

    /// Get the size of the nth (0-indexed) field/element of the `CompleteAbstractData`, in bits.
    /// The `CompleteAbstractData` must be a `Struct` or `Array`.
    pub fn field_size_in_bits(&self, n: usize) -> usize {
        match self {
            Self::Struct { elements, .. } => Self::size_in_bits(&elements[n]),
            Self::Array { element_type, .. } => Self::size_in_bits(element_type),
            Self::VoidOverride { data, .. } => data.field_size_in_bits(n),
            Self::SameSizeOverride { data, .. } => data.field_size_in_bits(n),
            Self::WithWatchpoint { data, .. } => data.field_size_in_bits(n),
            _ => panic!("field_size_in_bits called on {:?}", self),
        }
    }

    /// Get the offset of the nth (0-indexed) field/element of the `CompleteAbstractData`, in bits.
    /// The `CompleteAbstractData` must be a `Struct` or `Array`.
    pub fn offset_in_bits(&self, n: usize) -> usize {
        match self {
            Self::Struct { elements, .. } => elements.iter().take(n).map(Self::size_in_bits).sum(),
            Self::Array { element_type, .. } => element_type.size_in_bits() * n,
            Self::VoidOverride { data, .. } => data.offset_in_bits(n),
            Self::SameSizeOverride { data, .. } => data.offset_in_bits(n),
            Self::WithWatchpoint { data, .. } => data.offset_in_bits(n),
            _ => panic!("offset_in_bits called on {:?}", self),
        }
    }

    /// Does the `CompleteAbstractData` represent a pointer of some kind?
    pub fn is_pointer(&self) -> bool {
        match self {
            Self::PublicValue { .. } => false,
            Self::Secret { .. } => panic!("is_pointer on a Secret"),
            Self::Array { .. } => false,
            Self::Struct { .. } => false,
            Self::PublicPointerTo { .. } => true,
            Self::PublicPointerToFunction(_) => true,
            Self::PublicPointerToHook(_) => true,
            Self::PublicPointerToSelf => true,
            Self::PublicPointerToParentOr(_) => true,
            Self::VoidOverride { data, .. } => data.is_pointer(),
            Self::SameSizeOverride { data, .. } => data.is_pointer(),
            Self::WithWatchpoint { data, .. } => data.is_pointer(),
        }
    }

    /// Get the size of the data this `CompleteAbstractData` _points to_.
    ///
    /// Panics if `self` is not a pointer of some kind.
    pub fn pointee_size_in_bits(&self) -> usize {
        match self {
            Self::PublicValue { .. } => panic!("pointee_size_in_bits() on a non-pointer: {:?}", self),
            Self::Array { .. } => panic!("pointee_size_in_bits() on a non-pointer: {:?}", self),
            Self::Struct { .. } => panic!("pointee_size_in_bits() on a non-pointer: {:?}", self),
            Self::PublicPointerTo { pointee, .. } => pointee.size_in_bits(),
            Self::PublicPointerToFunction(_) => 64,  // as of this writing, haybale allocates 64 bits for functions; see State::new()
            Self::PublicPointerToHook(_) => 64,  // as of this writing, haybale allocates 64 bits for hooks; see State::new()
            Self::PublicPointerToSelf => unimplemented!("pointee_size_in_bits() on PublicPointerToSelf"),
            Self::PublicPointerToParentOr(None) => unimplemented!("pointee_size_in_bits() on PublicPointerToParent"),
            Self::PublicPointerToParentOr(Some(data)) => data.size_in_bits(),  // assume that if the parent typechecks, it's the same size
            Self::Secret { .. } => panic!("pointee_size_in_bits() on a Secret"),
            Self::VoidOverride { data, .. } => data.pointee_size_in_bits(),
            Self::SameSizeOverride { data, .. } => data.pointee_size_in_bits(),
            Self::WithWatchpoint { data, .. } => data.pointee_size_in_bits(),
        }
    }

    /// for internal use: could this `CompleteAbstractData` be valid for describing a struct of one element?
    pub(crate) fn could_describe_a_struct_of_one_element(&self) -> bool {
        match self {
            Self::Struct { elements, .. } => elements.len() == 1,  // compatible iff the number of elements is 1
            Self::Secret { .. } => true,  // could be compatible with the struct-of-one-element type
            Self::VoidOverride { .. } => true,  // could be compatible with the struct-of-one-element type
            Self::SameSizeOverride { .. } => true,  // could be compatible with the struct-of-one-element type
            Self::WithWatchpoint { .. } => true,  // could be compatible with the struct-of-one-element type
            _ => false,
        }
    }
}

/// This `Display` is not meant to completely replace the derived `Debug`
/// representation, but rather be a much more concise pretty representation
/// (omitting a lot of the data in some cases)
impl fmt::Display for CompleteAbstractData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::PublicValue { bits, .. } => write!(f, "a {}-bit public value", bits),
            Self::Secret { bits, .. } => write!(f, "a {}-bit secret value", bits),
            Self::Array { num_elements, .. } => write!(f, "an array of {} elements", num_elements),
            Self::Struct { name, elements } => write!(f, "a struct named {} with {} elements", name, elements.len()),
            Self::PublicPointerTo { pointee, .. } => {
                write!(f, "a pointer to ")?;
                pointee.fmt(f)?;
                Ok(())
            },
            Self::PublicPointerToFunction(funcname) => write!(f, "a pointer to a function named {}", funcname),
            Self::PublicPointerToHook(funcname) => write!(f, "a pointer to the active hook for a function named {}", funcname),
            Self::PublicPointerToSelf => write!(f, "a pointer to this struct itself"),
            Self::PublicPointerToParentOr(opt) => match opt {
                Some(_) => write!(f, "a pointer to this struct's parent, with a backup"),
                None => write!(f, "a pointer to this struct's parent, with no backup"),
            },
            Self::VoidOverride { data, .. } => {
                write!(f, "a void override containing ")?;
                data.fmt(f)?;
                Ok(())
            },
            Self::SameSizeOverride { data, .. } => {
                write!(f, "a same-size override containing ")?;
                data.fmt(f)?;
                Ok(())
            },
            Self::WithWatchpoint { name, data } => {
                data.fmt(f)?;
                write!(f, ", with a watchpoint named {}", name)?;
                Ok(())
            },
        }
    }
}

/// An abstract description of a value: if it is public or not, if it is a
/// pointer or not, does it point to data that is public/secret, maybe it's a
/// struct with some public and some secret fields, etc.
///
/// Unlike `CompleteAbstractData`, these may be "underspecified": parts of the
/// value (or the whole value) may be `Unspecified`, meaning to just use the
/// default based on the LLVM type.
#[derive(PartialEq, Eq, Clone, Debug)]
// we wrap the actual enum so that external users can't rely on the actual enum
// variants, and only see the (nicer and more stable) function constructors
pub struct AbstractData(pub(crate) UnderspecifiedAbstractData);

/// Enum which backs `AbstractData`; see comments there
#[derive(PartialEq, Eq, Clone, Debug)]
pub(crate) enum UnderspecifiedAbstractData {
    /// Just use the default structure based on the LLVM type, making all
    /// contents public.
    ///
    /// See [`AbstractData::to_complete`](enum.AbstractData.html#method.to_complete)
    Unspecified,

    /// Just fill with the appropriate number of unconstrained public bytes based
    /// on the LLVM type
    Unconstrained,

    /// Fill with the appropriate number of secret bytes based on the LLVM type
    Secret,

    /// Use the given `CompleteAbstractData`, which gives a complete description
    Complete(CompleteAbstractData),

    /// A (public) pointer to something underspecified
    PublicPointerTo {
        /// Description of the thing being pointed to
        pointee: Box<AbstractData>,
        /// If `false`, the pointer must point to the pointee; if `true`,
        /// it may either point to the pointee or be `NULL`
        maybe_null: bool,
    },

    /// Like `CompleteAbstractData::PublicPointerToParentOr`, but the `Or` part
    /// can be an `AbstractData` instead of a `CompleteAbstractData`.
    /// Also, the `Or` part isn't an `Option` - if you don't want the `Or` part,
    /// use `Complete` with `CompleteAbstractData::PublicPointerToParentOr(None)`
    PublicPointerToParentOr(Box<AbstractData>),

    /// an array with underspecified elements
    Array { element_type: Box<AbstractData>, num_elements: usize },

    /// a struct with underspecified fields
    /// (for instance, some unspecified and some fully-specified fields)
    Struct { name: String, elements: Vec<AbstractData> },

    /// Use the default structure for the given LLVM struct name.
    ///
    /// If we are not in the middle of an override, this struct name must match
    /// the actual LLVM type's struct name.
    ///
    /// If we are in the middle of an override and therefore don't have an
    /// LLVM type at the moment, this will act like `Unspecified` with the
    /// LLVM type being the one for the given LLVM struct name.
    DefaultForLLVMStructName { llvm_struct_name: String },

    /// See notes on [`CompleteAbstractData::VoidOverride`](enum.CompleteAbstractData.html).
    ///
    /// If the optional `llvm_struct_name` is included, it will lookup that
    /// struct's type and use that both for any underspecified elements in the
    /// `AbstractData`, and for sanity typechecking. Otherwise, the
    /// `AbstractData` must be fully-specified, and no sanity typechecking will
    /// be performed (the `AbstractData` will be assumed correct).
    VoidOverride { llvm_struct_name: Option<String>, data: Box<AbstractData> },

    /// See notes on [`CompleteAbstractData::SameSizeOverride`](enum.CompleteAbstractData.html).
    SameSizeOverride { data: Box<AbstractData> },

    /// Use the given `data`, but also (during initialization) add a watchpoint
    /// with the given `name` to the `State` covering the memory region it
    /// occupies.
    WithWatchpoint { name: String, data: Box<AbstractData> },
}

impl AbstractData {
    /// an 8-bit public value
    pub fn pub_i8(value: AbstractValue) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::pub_i8(value)))
    }

    /// a 16-bit public value
    pub fn pub_i16(value: AbstractValue) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::pub_i16(value)))
    }

    /// a 32-bit public value
    pub fn pub_i32(value: AbstractValue) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::pub_i32(value)))
    }

    /// a 64-bit public value
    pub fn pub_i64(value: AbstractValue) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::pub_i64(value)))
    }

    /// a public value with the given number of bits
    pub fn pub_integer(bits: usize, value: AbstractValue) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::pub_integer(bits, value)))
    }

    /// an 8-bit secret value
    pub fn sec_i8() -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::sec_i8()))
    }

    /// a 16-bit secret value
    pub fn sec_i16() -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::sec_i16()))
    }

    /// a 32-bit secret value
    pub fn sec_i32() -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::sec_i32()))
    }

    /// a 64-bit secret value
    pub fn sec_i64() -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::sec_i64()))
    }

    /// a secret value with the given number of bits
    pub fn sec_integer(bits: usize) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::sec_integer(bits)))
    }

    /// A (public) pointer to something - another value, an array, etc
    pub fn pub_pointer_to(data: Self) -> Self {
        Self(UnderspecifiedAbstractData::PublicPointerTo { pointee: Box::new(data), maybe_null: false })
    }

    /// A (public) pointer which may either point to the given data or be `NULL`
    pub fn pub_maybe_null_pointer_to(data: Self) -> Self {
        Self(UnderspecifiedAbstractData::PublicPointerTo { pointee: Box::new(data), maybe_null: true })
    }

    /// a (public) pointer to the LLVM `Function` with the given name
    pub fn pub_pointer_to_func(funcname: impl Into<String>) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::pub_pointer_to_func(funcname)))
    }

    /// a (public) pointer to the _hook_ registered for the given name
    pub fn pub_pointer_to_hook(funcname: impl Into<String>) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::pub_pointer_to_hook(funcname)))
    }

    /// A (public) pointer to this struct itself. E.g., in the C code
    /// ```c
    /// struct Foo {
    ///     int x;
    ///     Foo* f;
    /// };
    /// ```
    /// you could use this for `Foo* f` to indicate it should point to
    /// this exact `Foo` itself.
    pub fn pub_pointer_to_self() -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::pub_pointer_to_self()))
    }

    /// A (public) pointer to this struct's parent. E.g., in the C code
    /// ```c
    /// struct Foo {
    ///     int x;
    ///     Bar* bar1;
    ///     Bar* bar2;
    ///     ...
    /// };
    ///
    /// struct Bar {
    ///     int y;
    ///     Foo* parent;  // pointer to the Foo containing this Bar
    /// };
    /// ```
    /// you could use this for `Foo* parent` to indicate it should point to the
    /// `Foo` containing this `Bar`.
    pub fn pub_pointer_to_parent() -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::pub_pointer_to_parent()))
    }

    /// Like `pub_pointer_to_parent()`, but if the parent is not the correct type
    /// (or if there is no parent, i.e., we are directly initializing this)
    /// then pointer to the given `AbstractData` instead
    pub fn pub_pointer_to_parent_or(data: Self) -> Self {
        Self(UnderspecifiedAbstractData::PublicPointerToParentOr(Box::new(data)))
    }

    /// A (first-class) array of values
    pub fn array_of(element_type: Self, num_elements: usize) -> Self {
        Self(UnderspecifiedAbstractData::Array { element_type: Box::new(element_type), num_elements })
    }

    /// A (first-class) structure of values
    ///
    /// (`_struct` used instead of `struct` to avoid collision with the Rust keyword)
    pub fn _struct(name: impl Into<String>, elements: impl IntoIterator<Item = Self>) -> Self {
        Self(UnderspecifiedAbstractData::Struct { name: name.into(), elements: elements.into_iter().collect() })
    }

    /// Just use the default structure based on the LLVM type and/or the `StructDescriptions`.
    ///
    /// See [`AbstractData::to_complete`](struct.AbstractData.html#method.to_complete)
    pub fn default() -> Self {
        Self(UnderspecifiedAbstractData::Unspecified)
    }

    /// Use the default structure for the given LLVM struct name.
    ///
    /// If we are not in the middle of an override, this struct name must match
    /// the actual LLVM type's struct name.
    ///
    /// If we are in the middle of an override and therefore don't have an
    /// LLVM type at the moment, this will act like `default()` with the
    /// LLVM type being the one for the given LLVM struct name.
    pub fn default_for_llvm_struct_name(llvm_struct_name: impl Into<String>) -> Self {
        Self(UnderspecifiedAbstractData::DefaultForLLVMStructName { llvm_struct_name: llvm_struct_name.into() })
    }

    /// A (public) pointer which may point anywhere, including being `NULL`
    pub fn unconstrained_pointer() -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::unconstrained_pointer()))
    }

    /// Just fill with the appropriate number of unconstrained bytes based on the LLVM type
    pub fn unconstrained() -> Self {
        Self(UnderspecifiedAbstractData::Unconstrained)
    }

    /// Fill with the appropriate number of secret bytes based on the LLVM type
    pub fn secret() -> Self {
        Self(UnderspecifiedAbstractData::Secret)
    }

    /// See notes on [`CompleteAbstractData::void_override`](enum.CompleteAbstractData.html#method.void_override).
    ///
    /// Note that the `AbstractData` here must actually be fully specified,
    /// perhaps with the help of `StructDescriptions`. If it's not, `to_complete`
    /// will panic.
    ///
    /// If the optional `llvm_struct_name` is included, it will lookup that
    /// struct's type and use that both for any underspecified elements in the
    /// `AbstractData`, and for sanity typechecking. Otherwise, the
    /// `AbstractData` must be fully-specified, and no sanity typechecking will
    /// be performed (the `AbstractData` will be assumed correct).
    pub fn void_override(llvm_struct_name: Option<&str>, data: AbstractData) -> Self {
        Self(UnderspecifiedAbstractData::VoidOverride { llvm_struct_name: llvm_struct_name.map(Into::into), data: Box::new(data) })
    }

    /// See notes on [`CompleteAbstractData::same_size_override`](enum.CompleteAbstractData.html#method.same_size_override).
    ///
    /// Note that the `AbstractData` here must actually be fully specified,
    /// perhaps with the help of `StructDescriptions`. If it's not, `to_complete`
    /// will panic.
    pub fn same_size_override(data: AbstractData) -> Self {
        Self(UnderspecifiedAbstractData::SameSizeOverride { data: Box::new(data) })
    }

    /// Use the given `data`, but also (during initialization) add a watchpoint
    /// with the given `name` to the `State` covering the memory region it
    /// occupies.
    pub fn with_watchpoint(name: impl Into<String>, data: Self) -> Self {
        Self(UnderspecifiedAbstractData::WithWatchpoint { name: name.into(), data: Box::new(data) })
    }
}

/// This `Display` is not meant to completely replace the derived `Debug`
/// representation, but rather be a much more concise pretty representation
/// (omitting a lot of the data in some cases)
impl fmt::Display for AbstractData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// This `Display` is not meant to completely replace the derived `Debug`
/// representation, but rather be a much more concise pretty representation
/// (omitting a lot of the data in some cases)
impl fmt::Display for UnderspecifiedAbstractData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnderspecifiedAbstractData::Unspecified => write!(f, "an unspecified value"),
            UnderspecifiedAbstractData::Unconstrained => write!(f, "an unconstrained value"),
            UnderspecifiedAbstractData::Secret => write!(f, "a secret value"),
            UnderspecifiedAbstractData::Complete(cad) => {
                write!(f, "a complete value: ")?;
                cad.fmt(f)?;
                Ok(())
            },
            UnderspecifiedAbstractData::PublicPointerTo { pointee, .. } => {
                write!(f, "a pointer to ")?;
                pointee.fmt(f)?;
                Ok(())
            },
            UnderspecifiedAbstractData::PublicPointerToParentOr(_) => write!(f, "a public pointer to parent, with a backup"),
            UnderspecifiedAbstractData::Array { num_elements, .. } => write!(f, "an array of {} elements", num_elements),
            UnderspecifiedAbstractData::Struct { name, elements } => write!(f, "a struct named {} with {} elements", name, elements.len()),
            UnderspecifiedAbstractData::DefaultForLLVMStructName { llvm_struct_name } => write!(f, "the default for the LLVM struct {}", llvm_struct_name),
            UnderspecifiedAbstractData::VoidOverride { data, .. } => {
                write!(f, "a void override with data ")?;
                data.fmt(f)?;
                Ok(())
            },
            UnderspecifiedAbstractData::SameSizeOverride { data, .. } => {
                write!(f, "a same-size override with data ")?;
                data.fmt(f)?;
                Ok(())
            },
            UnderspecifiedAbstractData::WithWatchpoint { name, data } => {
                data.fmt(f)?;
                write!(f, " with a watchpoint named {}", name)?;
                Ok(())
            },
        }
    }
}

/// A map from struct name to an `AbstractData` description of the struct
pub type StructDescriptions = HashMap<String, AbstractData>;

impl AbstractData {
    pub const DEFAULT_ARRAY_LENGTH: usize = 1024;
    pub const POINTER_SIZE_BITS: usize = CompleteAbstractData::POINTER_SIZE_BITS;
    pub const OPAQUE_STRUCT_SIZE_BYTES: usize = 1024 * 64;

    /// Fill in the default `CompleteAbstractData` for any parts of the
    /// `AbstractData` which are marked `Default`, using the information in the
    /// [`StructDescriptions`](struct.StructDescriptions.html) and the given LLVM
    /// type.
    ///
    /// The default `CompleteAbstractData` based on the LLVM type is:
    ///
    /// - for LLVM integer type: public unconstrained value of the appropriate size
    /// - for LLVM pointer type (except function pointer): public concrete pointer value to allocated memory, depending on pointer type:
    ///   - pointee is an integer type: pointer to allocated array of `DEFAULT_ARRAY_LENGTH` pointees
    ///       (e.g., default for `char*` is pointer to array of 1024 chars)
    ///   - pointee is an array type with 0 elements: pointer to allocated array of `DEFAULT_ARRAY_LENGTH` elements
    ///   - pointee is any other type: pointer to one of that other type
    ///   - (then in any case, apply these rules recursively to each pointee type)
    /// - for LLVM function pointer type: concrete function pointer value which, when called, will raise an error
    /// - for LLVM vector or array type: array of the appropriate length, containing public values
    ///   - (then apply these rules recursively to each element)
    /// - for LLVM structure type:
    ///   - if this struct is one of those named in `sd`, then use the appropriate struct description
    ///   - if the structure type is entirely opaque (no definition anywhere in the `Project`), then allocate
    ///       `OPAQUE_STRUCT_SIZE_BYTES` unconstrained bytes for it and assume that's enough
    ///       (probably most of that memory will go unused, but that's fine)
    ///   - else, apply these rules recursively to each field
    pub fn to_complete(self, ty: &Type, proj: &Project, sd: &StructDescriptions) -> CompleteAbstractData {
        self.0.to_complete(ty, proj, sd)
    }

    fn to_complete_rec<'a>(self, ty: Option<&'a Type>, ctx: ToCompleteContext<'a, '_>) -> CompleteAbstractData {
        self.0.to_complete_rec(ty, ctx)
    }
}

/// Struct containing information we need to carry around during recursive calls to to_complete_rec()
#[derive(Clone)]
struct ToCompleteContext<'a, 'p> {
    proj: &'p Project,

    /// `StructDescriptions` which we are working with
    sd: &'p StructDescriptions,

    /// set of struct names we are within which were given
    /// `UnderspecifiedAbstractData::Unspecified` (whether they appear in `sd` or
    /// not). We keep track of these only so we can detect infinite recursion.
    unspecified_named_structs: HashSet<&'a String>,

    /// Name of the struct we are currently within (and the struct that is
    /// within, etc), purely for debugging purposes. First in the vec is the
    /// top-level struct, last is the most immediate struct.
    within_structs: Vec<String>,
}

impl<'a, 'p> ToCompleteContext<'a, 'p> {
    fn new(proj: &'p Project, sd: &'p StructDescriptions) -> Self {
        Self {
            proj,
            sd,
            unspecified_named_structs: HashSet::new(),
            within_structs: Vec::new(),
        }
    }

    fn error_backtrace(&self) {
        eprintln!();
        for w in self.within_structs.iter() {
            eprintln!("within struct {}:", w);
        }
    }
}

impl UnderspecifiedAbstractData {
    /// for internal use: could this `UnderspecifiedAbstractData` be valid for describing a struct of one element?
    pub(crate) fn could_describe_a_struct_of_one_element(&self) -> bool {
        match self {
            Self::Unspecified => true,  // compatible with the struct-of-one-element type
            Self::Unconstrained => true,  // compatible with the struct-of-one-element type
            Self::Secret => true,  // compatible with the struct-of-one-element type
            Self::Struct { elements, .. } => elements.len() == 1,  // compatible iff the number of elements is 1
            Self::Complete(CompleteAbstractData::Struct { elements, .. }) => elements.len() == 1,  // compatible iff the number of elements is 1
            Self::VoidOverride { .. } => true,  // could be compatible with the struct-of-one-element type
            Self::SameSizeOverride { .. } => true,  // could be compatible with the struct-of-one-element type
            Self::WithWatchpoint { .. } => true,  // could be compatible with the struct-of-one-element type
            _ => false,
        }
    }

    /// See method description on [`AbstractData::to_complete`](enum.AbstractData.html#method.to_complete)
    pub fn to_complete(self, ty: &Type, proj: &Project, sd: &StructDescriptions) -> CompleteAbstractData {
        self.to_complete_rec(Some(ty), ToCompleteContext::new(proj, sd))
    }

    /// If `ty` is `None`, this indicates that either:
    ///   (1) we are explicitly overriding the LLVM type via `VoidOverride` or `SameSizeOverride`, or
    ///   (2) we are initializing a struct via the `StructDescriptions` that we don't have an LLVM type for because it's opaque
    fn to_complete_rec<'a>(self, ty: Option<&'a Type>, mut ctx: ToCompleteContext<'a, '_>) -> CompleteAbstractData {
        // Set of struct names which have been detected to have infinite recursion,
        // and which we have already warned about. We won't warn again for the same
        // struct names.
        lazy_static! {
            static ref WARNED_STRUCTS: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
        }

        // If LLVM type is a struct of one element, and UAD is specified as
        // something else, unwrap the LLVM struct and try again
        match ty {
            Some(Type::StructType { element_types, .. }) if element_types.len() == 1 => {
                if !self.could_describe_a_struct_of_one_element() {
                    // `self` specifies some incompatible type.  Unwrap the LLVM struct and try again.
                    return self.to_complete_rec(Some(&element_types[0]), ctx);
                }
            },
            Some(ty@Type::NamedStructType { .. }) => {
                match ctx.proj.get_inner_struct_type_from_named(ty) {
                    None => {},  // we're looking for where LLVM type is a struct of one element. Opaque struct type is a different problem.
                    Some(arc) => {
                        let actual_ty: &Type = &arc.read().unwrap();
                        if let Type::StructType { element_types, .. } = actual_ty {
                            if element_types.len() == 1 {
                                // the LLVM type is struct of one element.  Proceed as in the above case
                                if !self.could_describe_a_struct_of_one_element() {
                                    // `self` specifies some incompatible type.  Unwrap the LLVM struct and try again.
                                    // we could consider pushing the named struct name to within_structs here
                                    return self.to_complete_rec(Some(&element_types[0]), ctx);
                                }
                            }
                        }
                    }
                }
            },
            _ => {},  // LLVM type isn't struct of one element.  Continue.
        }

        // Otherwise, on to the normal processing
        match self {
            Self::Complete(abstractdata) => abstractdata,
            Self::Unconstrained => match ty {
                Some(ty) => CompleteAbstractData::PublicValue { bits: layout::size(ty), value: AbstractValue::Unconstrained },
                None => {
                    ctx.error_backtrace();
                    panic!("Encountered an AbstractData::unconstrained() but don't have an LLVM type to use");
                },
            },
            Self::Secret => match ty {
                Some(ty) => CompleteAbstractData::Secret { bits: layout::size(ty) },
                None => {
                    ctx.error_backtrace();
                    panic!("Encountered an AbstractData::secret() but don't have an LLVM type to use");
                },
            },
            Self::WithWatchpoint { name, data } => CompleteAbstractData::with_watchpoint(name, data.to_complete_rec(ty, ctx)),
            Self::VoidOverride { llvm_struct_name, data } => match llvm_struct_name {
                None => CompleteAbstractData::void_override(None, data.to_complete_rec(None, ctx)),
                Some(llvm_struct_name) => {
                    let (llvm_ty, _) = ctx.proj.get_named_struct_type_by_name(&llvm_struct_name)
                        .unwrap_or_else(|| { ctx.error_backtrace(); panic!("VoidOverride: llvm_struct_name {:?} not found in Project", llvm_struct_name) });
                    let arc = llvm_ty.as_ref().unwrap_or_else(|| { ctx.error_backtrace(); panic!("VoidOverride: llvm_struct_name {:?} is an opaque type", llvm_struct_name) });
                    let ty = &arc.read().unwrap();
                    CompleteAbstractData::void_override(Some(&llvm_struct_name), data.to_complete_rec(Some(ty), ctx))
                },
            }
            Self::SameSizeOverride { data } => CompleteAbstractData::same_size_override(data.to_complete_rec(None, ctx)),
            Self::PublicPointerTo { pointee, maybe_null } => match ty {
                Some(Type::PointerType { pointee_type, .. }) =>
                    CompleteAbstractData::PublicPointerTo { pointee: Box::new(match &pointee.0 {
                        Self::Array { num_elements, .. } => {
                            // AbstractData is pointer-to-array, but LLVM type may be pointer-to-scalar
                            match &**pointee_type {
                                ty@Type::ArrayType { .. } | ty@Type::VectorType { .. } => {
                                    pointee.to_complete_rec(Some(ty), ctx)  // LLVM type is array or vector as well, it matches
                                },
                                ty => {
                                    // LLVM type is scalar, but AbstractData is array, so it's actually pointer-to-array
                                    let num_elements = *num_elements;
                                    pointee.to_complete_rec(Some(&Type::ArrayType { element_type: Box::new(ty.clone()), num_elements }), ctx)
                                },
                            }
                        },
                        _ => {
                            // AbstractData is pointer-to-something-else, just let the recursive call handle it
                            pointee.to_complete_rec(Some(&**pointee_type), ctx)
                        },
                    }), maybe_null },
                Some(Type::ArrayType { num_elements: 1, element_type }) | Some(Type::VectorType { num_elements: 1, element_type }) => {
                    // auto-unwrap LLVM type if it is array or vector of one element
                    Self::PublicPointerTo { pointee, maybe_null }.to_complete_rec(Some(&**element_type), ctx)
                },
                None => CompleteAbstractData::PublicPointerTo { pointee: Box::new(pointee.to_complete_rec(None, ctx)), maybe_null },
                _ => {
                    ctx.error_backtrace();
                    panic!("Type mismatch: AbstractData::PublicPointerTo but LLVM type is {:?}", ty);
                },
            },
            Self::PublicPointerToParentOr(ad) => {
                let pointee_ty: Option<&Type> = match ty {
                    Some(Type::PointerType { pointee_type, .. }) => Some(pointee_type),
                    Some(ty) => {
                        ctx.error_backtrace();
                        panic!("Type mismatch: AbstractData::PublicPointerToParentOr but LLVM type is not a pointer: {:?}", ty);
                    },
                    None => None,
                };
                CompleteAbstractData::pub_pointer_to_parent_or(ad.to_complete_rec(pointee_ty, ctx))
            },
            Self::Array { element_type, num_elements } => match ty {
                Some(Type::ArrayType { element_type: llvm_element_type, num_elements: llvm_num_elements })
                | Some(Type::VectorType { element_type: llvm_element_type, num_elements: llvm_num_elements }) => {
                    if *llvm_num_elements != 0 && *llvm_num_elements != num_elements {
                        ctx.error_backtrace();
                        panic!("Type mismatch: AbstractData specifies an array with {} elements, but found an array with {} elements", num_elements, llvm_num_elements);
                    }
                    CompleteAbstractData::array_of(element_type.to_complete_rec(Some(&**llvm_element_type), ctx.clone()), num_elements)
                },
                None => CompleteAbstractData::array_of(element_type.to_complete_rec(None, ctx.clone()), num_elements),
                _ => {
                    ctx.error_backtrace();
                    panic!("Type mismatch: AbstractData::Array with {} elements, but LLVM type is {:?}", num_elements, ty);
                },
            }
            Self::Struct { elements, name } => match ty {
                Some(ty@Type::NamedStructType { .. }) => {
                    match ctx.proj.get_inner_struct_type_from_named(ty) {
                        Some(arc) => {
                            let actual_ty: &Type = &arc.read().unwrap();
                            Self::Struct { elements, name }.to_complete_rec(Some(actual_ty), ctx)
                        },
                        None => Self::Struct { elements, name }.to_complete_rec(None, ctx),
                    }
                },
                Some(Type::StructType { element_types, .. }) => {
                    ctx.within_structs.push(name.clone());
                    if elements.len() != element_types.len() {
                        ctx.error_backtrace();
                        panic!("Type mismatch: AbstractData::Struct with {} elements, but LLVM type has {} elements: {:?}", elements.len(), element_types.len(), element_types);
                    }
                    CompleteAbstractData::_struct(name, elements
                        .into_iter()
                        .zip(element_types)
                        .map(|(el_data, el_type)| el_data.to_complete_rec(Some(el_type), ctx.clone()))
                    )
                },
                Some(Type::ArrayType { num_elements: 1, element_type }) | Some(Type::VectorType { num_elements: 1, element_type }) => {
                    // auto-unwrap LLVM type if it is array or vector of one element
                    Self::Struct { elements, name }.to_complete_rec(Some(&**element_type), ctx.clone())
                },
                None => {
                    ctx.within_structs.push(name.clone());
                    CompleteAbstractData::_struct(name, elements.into_iter().map(|el_data| el_data.to_complete_rec(None, ctx.clone())))
                }
                _ => {
                    ctx.error_backtrace();
                    panic!("Type mismatch: AbstractData::Struct {}, but LLVM type is {:?}", name, ty);
                },
            },
            Self::DefaultForLLVMStructName { llvm_struct_name } => match ty {
                Some(Type::NamedStructType { name, .. }) => {
                    if name == &llvm_struct_name {
                        // all's normal, just treat this as an Unspecified
                        Self::Unspecified.to_complete_rec(ty, ctx)
                    } else {
                        ctx.error_backtrace();
                        panic!("default_for_llvm_struct_name {:?}, but LLVM type is a struct named {:?}", llvm_struct_name, name)
                    }
                },
                Some(Type::StructType { .. }) => {
                    // just treat this as an Unspecified and try to proceed.
                    // If the struct types don't match, we'll get the type error later
                    Self::Unspecified.to_complete_rec(ty, ctx)
                },
                Some(ty) => {
                    ctx.error_backtrace();
                    panic!("default_for_llvm_struct_name {:?}, but LLVM type is not a structure type: {:?}", llvm_struct_name, ty)
                },
                None => {
                    // working as intended - use this `llvm_struct_name` as the type from here on out
                    let (llvm_struct_ty_arc, _) = ctx.proj.get_named_struct_type_by_name(&llvm_struct_name)
                        .unwrap_or_else(|| { ctx.error_backtrace(); panic!("default_for_llvm_struct_name: struct name {:?} not found in the Project", llvm_struct_name); });
                    let llvm_struct_ty_arc = llvm_struct_ty_arc
                        .as_ref()
                        .unwrap_or_else(|| { ctx.error_backtrace(); panic!("default_for_llvm_struct_name: struct name {:?} is entirely opaque in this Project", llvm_struct_name); })
                        .clone();
                    let llvm_struct_ty: &Type = &llvm_struct_ty_arc.read().unwrap();
                    Self::Unspecified.to_complete_rec(Some(llvm_struct_ty), ctx)
                },
            },
            Self::Unspecified => match ty {
                None => {
                    ctx.error_backtrace();
                    panic!("Encountered an AbstractData::default() but don't have an LLVM type to use; this is either because:\n  (1) either same_size_override or void_override with llvm_struct_name == None were used, but the specified AbstractData contained a default() somewhere; or\n  (2) a struct in the StructDescriptions is opaque in this Project, but the specified AbstractData contained a default() somewhere");
                },
                Some(ty) => match ty {
                    ty@Type::IntegerType { .. } =>
                        CompleteAbstractData::pub_integer(layout::size(ty), AbstractValue::Unconstrained),
                    Type::PointerType { pointee_type, .. } => match &**pointee_type {
                        Type::FuncType { .. } =>
                            CompleteAbstractData::pub_pointer_to_hook("hook_uninitialized_function_pointer"),
                        Type::IntegerType { bits } =>
                            CompleteAbstractData::pub_pointer_to(CompleteAbstractData::array_of(
                                CompleteAbstractData::pub_integer(*bits as usize, AbstractValue::Unconstrained),
                                AbstractData::DEFAULT_ARRAY_LENGTH,
                            )),
                        Type::ArrayType { num_elements: 0, element_type } =>
                            CompleteAbstractData::pub_pointer_to(CompleteAbstractData::array_of(
                                Self::Unspecified.to_complete_rec(Some(element_type), ctx),
                                AbstractData::DEFAULT_ARRAY_LENGTH,
                            )),
                        ty => CompleteAbstractData::pub_pointer_to(Self::Unspecified.to_complete_rec(Some(ty), ctx)),
                    },
                    Type::VectorType { element_type, num_elements } | Type::ArrayType { element_type, num_elements } =>
                        CompleteAbstractData::array_of(
                            Self::Unspecified.to_complete_rec(Some(element_type), ctx),
                            *num_elements,
                        ),
                    Type::NamedStructType { name, .. } => {
                        let arc = ctx.proj.get_inner_struct_type_from_named(ty);
                        if !ctx.unspecified_named_structs.insert(name) {
                            match arc {
                                Some(arc) => {
                                    if WARNED_STRUCTS.lock().unwrap().insert(name.clone()) {
                                        warn!("Setting the contents of a {:?} to unconstrained in order to avoid infinite recursion. We will not warn again for infinite recursion on a {:?}", name, name);
                                    }
                                    let inner_ty: &Type = &arc.read().unwrap();
                                    return CompleteAbstractData::PublicValue { bits: layout::size(inner_ty), value: AbstractValue::Unconstrained };
                                },
                                None => {
                                    ctx.error_backtrace();
                                    panic!("Encountered infinite recursion in struct {:?}, which is opaque; this should be impossible");
                                },
                            }
                        }
                        match ctx.sd.get(name) {
                            Some(abstractdata) => {
                                // This is in the StructDescriptions, so use the description there
                                ctx.within_structs.push(name.clone());
                                match arc {
                                    Some(arc) => {
                                        let inner_ty: &Type = &arc.read().unwrap();
                                        abstractdata.clone().to_complete_rec(Some(inner_ty), ctx)
                                    },
                                    None => abstractdata.clone().to_complete_rec(None, ctx),
                                }
                            },
                            None => match arc {
                                Some(arc) => {
                                    // We have an LLVM struct definition, so use that
                                    ctx.within_structs.push(name.clone());
                                    let inner_ty: &Type = &arc.read().unwrap();
                                    match self.to_complete_rec(Some(inner_ty), ctx) {
                                        CompleteAbstractData::Struct { elements, .. } => CompleteAbstractData::_struct(name.clone(), elements),  // put in the correct struct name
                                        cad => panic!("Expected to end up with a Struct from this call, but got {:?}", cad),
                                    }
                                },
                                None => {
                                    // all definitions of the struct in the project are opaque, and it isn't in the StructDescriptions
                                    // allocate OPAQUE_STRUCT_SIZE_BYTES unconstrained bytes and call it good
                                    CompleteAbstractData::array_of(CompleteAbstractData::pub_i8(AbstractValue::Unconstrained), AbstractData::OPAQUE_STRUCT_SIZE_BYTES)
                                },
                            },
                        }
                    },
                    Type::StructType { element_types, .. } => CompleteAbstractData::_struct("unspecified_struct", element_types
                        .iter()
                        .map(|el_type| Self::Unspecified.to_complete_rec(Some(el_type), ctx.clone()))
                    ),
                    _ => unimplemented!("AbstractData::to_complete with {:?}", ty),
                },
            },
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AbstractValue {
    /// This exact numerical value
    ExactValue(u64),
    /// Any numerical value in the range (inclusive)
    Range(u64, u64),
    /// Any value whatsoever
    Unconstrained,
    /// A value with a (unique) name, so that it can be referenced in a `Equal`, `SignedLessThan`, `SignedGreaterThan`, etc.
    ///
    /// If more than one `AbstractValue` is given the same name, they will implicitly be set equal to each other.
    Named {
        name: String,
        value: Box<AbstractValue>,
    },
    /// A value equal to the value with the given name
    EqualTo(String),
    /// A value signed-less-than the value with the given name
    SignedLessThan(String),
    /// A value signed-greater-than the value with the given name
    SignedGreaterThan(String),
    /// A value unsigned-less-than the value with the given name
    UnsignedLessThan(String),
    /// A value unsigned-greater-than the value with the given name
    UnsignedGreaterThan(String),
}

impl AbstractValue {
    pub fn named(name: &str, value: AbstractValue) -> Self {
        Self::Named {
            name: name.to_owned(),
            value: Box::new(value),
        }
    }
}
