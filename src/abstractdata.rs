use haybale::{layout, Project};
use llvm_ir::Type;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

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
    /// if you want to specify the exact numerical value of the pointer (e.g. NULL).
    PublicValue { bits: usize, value: AbstractValue },

    /// A (first-class) array of values
    Array { element_type: Box<Self>, num_elements: usize },

    /// A (first-class) structure of values
    Struct { name: String, elements: Vec<Self> },

    /// A (public) pointer to something - another value, an array, etc
    PublicPointerTo(Box<Self>),

    /// A (public) pointer to the LLVM `Function` with the given name
    PublicPointerToFunction(String),

    /// A (public) pointer to the _hook_ registered for the given name
    PublicPointerToHook(String),

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
    PublicPointerToParent,

    /// A (public) pointer which may point anywhere
    PublicUnconstrainedPointer,

    /// A secret value (pointer or non-pointer, doesn't matter) of the given size in bits
    Secret { bits: usize },

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
        Self::PublicPointerTo(Box::new(data))
    }

    /// a (public) pointer to the LLVM `Function` with the given name
    pub fn pub_pointer_to_func(funcname: impl Into<String>) -> Self {
        Self::PublicPointerToFunction(funcname.into())
    }

    /// a (public) pointer to the _hook_ registered for the given name
    pub fn pub_pointer_to_hook(funcname: impl Into<String>) -> Self {
        Self::PublicPointerToHook(funcname.into())
    }

    /// a (public) pointer to this struct's parent; see comments on
    /// `CompleteAbstractData::PublicPointerToParent`
    pub fn pub_pointer_to_parent() -> Self {
        Self::PublicPointerToParent
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

    /// A (public) pointer which may point anywhere
    pub fn unconstrained_pointer() -> Self {
        Self::PublicUnconstrainedPointer
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
}

impl CompleteAbstractData {
    pub const POINTER_SIZE_BITS: usize = 64;

    /// Get the size of the `CompleteAbstractData`, in bits
    pub fn size_in_bits(&self) -> usize {
        match self {
            Self::PublicValue { bits, .. } => *bits,
            Self::Array { element_type, num_elements } => element_type.size_in_bits() * num_elements,
            Self::Struct { elements, .. } => elements.iter().map(Self::size_in_bits).sum(),
            Self::PublicPointerTo(_) => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToFunction(_) => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToHook(_) => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToParent => Self::POINTER_SIZE_BITS,
            Self::PublicUnconstrainedPointer => Self::POINTER_SIZE_BITS,
            Self::Secret { bits } => *bits,
            Self::VoidOverride { data, .. } => data.size_in_bits(),
        }
    }

    /// Get the size of the nth (0-indexed) field/element of the `CompleteAbstractData`, in bits.
    /// The `CompleteAbstractData` must be a `Struct` or `Array`.
    pub fn field_size_in_bits(&self, n: usize) -> usize {
        match self {
            Self::Struct { elements, .. } => Self::size_in_bits(&elements[n]),
            Self::Array { element_type, .. } => Self::size_in_bits(element_type),
            Self::VoidOverride { data, .. } => data.field_size_in_bits(n),
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
            _ => panic!("offset_in_bits called on {:?}", self),
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
    /// Just use the default structure based on the LLVM type.
    ///
    /// See [`AbstractData::to_complete`](enum.AbstractData.html#method.to_complete)
    Unspecified,

    /// Use the given `CompleteAbstractData`, which gives a complete description
    Complete(CompleteAbstractData),

    /// A (public) pointer to something underspecified
    PublicPointerTo(Box<AbstractData>),

    /// an array with underspecified elements
    Array { element_type: Box<AbstractData>, num_elements: usize },

    /// a struct with underspecified fields
    /// (for instance, some unspecified and some fully-specified fields)
    Struct { name: String, elements: Vec<AbstractData> },

    /// See notes on [`CompleteAbstractData::VoidOverride`](enum.CompleteAbstractData.html).
    ///
    /// If the optional `llvm_struct_name` is included, it will lookup that
    /// struct's type and use that both for any underspecified elements in the
    /// `AbstractData`, and for sanity typechecking. Otherwise, the
    /// `AbstractData` must be fully-specified, and no sanity typechecking will
    /// be performed (the `AbstractData` will be assumed correct).
    VoidOverride { llvm_struct_name: Option<String>, data: Box<AbstractData> },
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
        Self(UnderspecifiedAbstractData::PublicPointerTo(Box::new(data)))
    }

    /// a (public) pointer to the LLVM `Function` with the given name
    pub fn pub_pointer_to_func(funcname: impl Into<String>) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::PublicPointerToFunction(funcname.into())))
    }

    /// a (public) pointer to the _hook_ registered for the given name
    pub fn pub_pointer_to_hook(funcname: impl Into<String>) -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::PublicPointerToHook(funcname.into())))
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
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::PublicPointerToParent))
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

    /// A (public) pointer which may point anywhere
    pub fn unconstrained_pointer() -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::PublicUnconstrainedPointer))
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
}

/// A map from struct name to an `AbstractData` description of the struct
pub type StructDescriptions = HashMap<String, AbstractData>;

impl AbstractData {
    pub const DEFAULT_ARRAY_LENGTH: usize = 1024;
    pub const POINTER_SIZE_BITS: usize = CompleteAbstractData::POINTER_SIZE_BITS;

    /// Fill in the default `CompleteAbstractData` for any parts of the
    /// `AbstractData` which are marked `Default`, using the information in the
    /// [`StructDescriptions`](struct.StructDescriptions.html) and the given LLVM
    /// type.
    ///
    /// The default `CompleteAbstractData` based on the LLVM type is:
    ///
    /// - for LLVM integer type: public unconstrained value of the appropriate size
    /// - for LLVM pointer type (except function pointer): public concrete pointer value to allocated memory, depending on pointer type:
    ///   - pointee is an integer type: pointer to allocated array of DEFAULT_ARRAY_LENGTH pointees
    ///       (e.g., default for char* is pointer to array of 1024 chars)
    ///   - pointee is any other type: pointer to one of that other type
    ///   - (then in either case, apply these rules recursively to each pointee type)
    /// - for LLVM function pointer type: concrete function pointer value which, when called, will raise an error
    /// - for LLVM vector or array type: array of the appropriate length, containing public values
    ///   (unless the number of elements is 0, in which case, we default to DEFAULT_ARRAY_LENGTH elements)
    ///   - (in any case, apply these rules recursively to each element)
    /// - for LLVM structure type:
    ///   - if this struct is one of those named in `sd`, then use the appropriate struct description
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

    /// set of struct names we have encountered which were given
    /// `UnderspecifiedAbstractData::Unspecified` and don't appear in `sd`. We
    /// keep track of these only so we can detect infinite recursion and abort
    /// with an appropriate error message.
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
    /// See method description on [`AbstractData::to_complete`](enum.AbstractData.html#method.to_complete)
    pub fn to_complete(self, ty: &Type, proj: &Project, sd: &StructDescriptions) -> CompleteAbstractData {
        self.to_complete_rec(Some(ty), ToCompleteContext::new(proj, sd))
    }

    /// If `ty` is `None`, this indicates that we are explicitly overriding the LLVM type via `VoidOverride`
    fn to_complete_rec<'a>(self, ty: Option<&'a Type>, mut ctx: ToCompleteContext<'a, '_>) -> CompleteAbstractData {
        match self {
            Self::Complete(abstractdata) => abstractdata,
            Self::VoidOverride { llvm_struct_name, data } => match llvm_struct_name {
                None => CompleteAbstractData::VoidOverride { llvm_struct_name: None, data: Box::new(data.to_complete_rec(None, ctx)) },
                Some(llvm_struct_name) => {
                    let (llvm_ty, _) = ctx.proj.get_named_struct_type_by_name(&llvm_struct_name)
                        .unwrap_or_else(|| panic!("VoidOverride: llvm_struct_name {:?} not found in Project", llvm_struct_name));
                    let arc = llvm_ty.as_ref().unwrap_or_else(|| panic!("VoidOverride: llvm_struct_name {:?} is an opaque type", llvm_struct_name));
                    let ty = &arc.read().unwrap();
                    CompleteAbstractData::VoidOverride { llvm_struct_name: Some(llvm_struct_name), data: Box::new(data.to_complete_rec(Some(ty), ctx)) }
                },
            }
            Self::PublicPointerTo(ad) => match ty {
                Some(Type::PointerType { pointee_type, .. }) =>
                    CompleteAbstractData::PublicPointerTo(Box::new(match &ad.0 {
                        Self::Array { num_elements, .. } => {
                            // AbstractData is pointer-to-array, but LLVM type may be pointer-to-scalar
                            match &**pointee_type {
                                ty@Type::ArrayType { .. } | ty@Type::VectorType { .. } => {
                                    ad.to_complete_rec(Some(ty), ctx)  // LLVM type is array or vector as well, it matches
                                },
                                ty => {
                                    // LLVM type is scalar, but AbstractData is array, so it's actually pointer-to-array
                                    let num_elements = *num_elements;
                                    ad.to_complete_rec(Some(&Type::ArrayType { element_type: Box::new(ty.clone()), num_elements }), ctx)
                                },
                            }
                        },
                        _ => {
                            // AbstractData is pointer-to-something-else, just let the recursive call handle it
                            ad.to_complete_rec(Some(&**pointee_type), ctx)
                        },
                    })),
                Some(Type::ArrayType { num_elements: 1, element_type }) | Some(Type::VectorType { num_elements: 1, element_type }) => {
                    // auto-unwrap LLVM type if it is array or vector of one element
                    Self::PublicPointerTo(ad).to_complete_rec(Some(&**element_type), ctx)
                },
                None => ad.to_complete_rec(None, ctx),
                _ => {
                    ctx.error_backtrace();
                    panic!("Type mismatch: AbstractData::PublicPointerTo but LLVM type is {:?}", ty);
                },
            },
            Self::Array { element_type, num_elements } => match ty {
                Some(Type::ArrayType { element_type: llvm_element_type, num_elements: llvm_num_elements })
                | Some(Type::VectorType { element_type: llvm_element_type, num_elements: llvm_num_elements }) => {
                    if *llvm_num_elements != 0 && *llvm_num_elements != num_elements {
                        ctx.error_backtrace();
                        panic!("Type mismatch: AbstractData specifies an array with {} elements, but found an array with {} elements", num_elements, llvm_num_elements);
                    }
                    CompleteAbstractData::Array { element_type: Box::new(element_type.to_complete_rec(Some(&**llvm_element_type), ctx.clone())), num_elements }
                },
                None => CompleteAbstractData::Array { element_type: Box::new(element_type.to_complete_rec(None, ctx.clone())), num_elements },
                _ => {
                    ctx.error_backtrace();
                    panic!("Type mismatch: AbstractData::Array with {} elements, but LLVM type is {:?}", num_elements, ty);
                },
            }
            Self::Struct { elements, name } => match ty {
                Some(Type::NamedStructType { name: llvm_struct_name, ty }) => {
                    let arc: Arc<RwLock<Type>> = match &ty.as_ref() {
                        Some(ty) => ty.upgrade().expect("Failed to upgrade weak reference"),
                        None => {
                            // This is an opaque struct definition. Try to find a non-opaque definition for the same struct.
                            let (ty, _) = ctx.proj.get_named_struct_type_by_name(&llvm_struct_name).unwrap_or_else(|| panic!("Struct name {:?} (LLVM name {:?}) not found in the project", name, llvm_struct_name));
                            ty.as_ref()
                                .unwrap_or_else(|| panic!("Can't convert struct named {:?} (LLVM name {:?}) to complete: it has only opaque definitions in this project", name, llvm_struct_name))
                                .clone()
                        },
                    };
                    let actual_ty: &Type = &arc.read().unwrap();
                    Self::Struct { elements, name }.to_complete_rec(Some(actual_ty), ctx)
                },
                Some(Type::StructType { element_types, .. }) => {
                    ctx.within_structs.push(name.clone());
                    if elements.len() != element_types.len() {
                        ctx.error_backtrace();
                        panic!("Type mismatch: AbstractData::Struct with {} elements, but LLVM type has {} elements", elements.len(), element_types.len());
                    }
                    CompleteAbstractData::Struct { name, elements:
                        elements.into_iter()
                        .zip(element_types)
                        .map(|(el_data, el_type)| el_data.to_complete_rec(Some(el_type), ctx.clone()))
                        .collect()
                    }
                },
                Some(Type::ArrayType { num_elements: 1, element_type }) | Some(Type::VectorType { num_elements: 1, element_type }) => {
                    // auto-unwrap LLVM type if it is array or vector of one element
                    Self::Struct { elements, name }.to_complete_rec(Some(&**element_type), ctx.clone())
                },
                None => {
                    ctx.within_structs.push(name.clone());
                    CompleteAbstractData::Struct { name, elements:
                        elements.into_iter().map(|el_data| el_data.to_complete_rec(None, ctx.clone())).collect()
                    }
                }
                _ => {
                    ctx.error_backtrace();
                    panic!("Type mismatch: AbstractData::Struct {}, but LLVM type is {:?}", name, ty);
                },
            },
            Self::Unspecified => match ty.expect("void_override requires a fully-specified structure") {
                ty@Type::IntegerType { .. } =>
                    CompleteAbstractData::PublicValue { bits: layout::size(ty), value: AbstractValue::Unconstrained },
                Type::PointerType { pointee_type, .. } => match &**pointee_type {
                    Type::FuncType { .. } =>
                        CompleteAbstractData::PublicPointerToHook("hook_uninitialized_function_pointer".to_owned()),
                    Type::IntegerType { bits } =>
                        CompleteAbstractData::PublicPointerTo(Box::new(CompleteAbstractData::Array {
                            element_type: Box::new(CompleteAbstractData::PublicValue { bits: *bits as usize, value: AbstractValue::Unconstrained}),
                            num_elements: AbstractData::DEFAULT_ARRAY_LENGTH,
                        })),
                    ty => CompleteAbstractData::PublicPointerTo(Box::new(Self::Unspecified.to_complete_rec(Some(ty), ctx))),
                },
                Type::VectorType { element_type, num_elements } | Type::ArrayType { element_type, num_elements } =>
                    CompleteAbstractData::Array {
                        element_type: Box::new(Self::Unspecified.to_complete_rec(Some(element_type), ctx)),
                        num_elements: if *num_elements == 0 { AbstractData::DEFAULT_ARRAY_LENGTH } else { *num_elements },
                    },
                Type::NamedStructType { ty, name } => {
                    let arc: Arc<RwLock<Type>> = match &ty.as_ref() {
                        Some(ty) => ty.upgrade().expect("Failed to upgrade weak reference"),
                        None => {
                            // This is an opaque struct definition. Try to find a non-opaque definition for the same struct.
                            let (ty, _) = ctx.proj.get_named_struct_type_by_name(&name).unwrap_or_else(|| panic!("Struct name {:?} not found in the project", name));
                            ty.as_ref()
                                .unwrap_or_else(|| panic!("Can't convert struct named {:?} to complete: it has only opaque definitions in this project", name))
                                .clone()
                        },
                    };
                    let inner_ty: &Type = &arc.read().unwrap();
                    match ctx.sd.get(name) {
                        Some(abstractdata) => {
                            ctx.within_structs.push(name.clone());
                            abstractdata.clone().to_complete_rec(Some(inner_ty), ctx)
                        },
                        None => {
                            if ctx.unspecified_named_structs.insert(name) {
                                ctx.within_structs.push(name.clone());
                                self.to_complete_rec(Some(inner_ty), ctx)
                            } else {
                                ctx.error_backtrace();
                                panic!("AbstractData::default() applied to recursive struct {:?}", name)
                            }
                        },
                    }
                },
                Type::StructType { element_types, .. } => CompleteAbstractData::Struct { name: "unspecified_struct".to_owned(), elements:
                    element_types.iter()
                    .map(|el_type| Self::Unspecified.to_complete_rec(Some(el_type), ctx.clone()))
                    .collect()
                },
                _ => unimplemented!("AbstractData::to_complete with {:?}", ty),
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
    /// A value with a (unique) name, so that it can be referenced in a `Equal`, `SignedLessThan`, `SignedGreaterThan`, etc
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
