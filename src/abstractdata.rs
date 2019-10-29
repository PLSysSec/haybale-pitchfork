use haybale::layout;
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
    /// A public value, of the given size in bits. If `value` is `Some`, then it
    /// is the actual concrete value; otherwise (if `value` is `None`) the value
    /// is unconstrained.
    ///
    /// This may be used for either a non-pointer value, or for a pointer value
    /// if you want to specify the exact numerical value of the pointer (e.g. NULL).
    PublicValue { bits: usize, value: AbstractValue },

    /// A (first-class) array of values
    Array { element_type: Box<Self>, num_elements: usize },

    /// A (first-class) structure of values
    Struct(Vec<Self>),

    /// A (public) pointer to something - another value, an array, etc
    PublicPointerTo(Box<Self>),

    /// A (public) pointer to the LLVM `Function` with the given name
    PublicPointerToFunction(String),

    /// A (public) pointer to the _hook_ registered for the given name
    PublicPointerToHook(String),

    /// A (public) pointer to unconstrained public data, which could be a public
    /// value, an array (of unconstrained size) of public values, or a public
    /// data structure
    PublicPointerToUnconstrainedPublic,

    /// A secret value (pointer or non-pointer, doesn't matter) of the given size in bits
    Secret { bits: usize },
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

    /// A (first-class) array of values
    pub fn array_of(element_type: Self, num_elements: usize) -> Self {
        Self::Array { element_type: Box::new(element_type), num_elements }
    }

    /// A (first-class) structure of values
    pub fn struct_of(elements: impl IntoIterator<Item = Self>) -> Self {
        Self::Struct(elements.into_iter().collect())
    }

    /// A (public) pointer which may point anywhere
    pub fn unconstrained_pointer() -> Self {
        Self::PublicPointerToUnconstrainedPublic
    }
}

impl CompleteAbstractData {
    pub const POINTER_SIZE_BITS: usize = 64;

    /// Get the size of the `CompleteAbstractData`, in bits
    pub fn size_in_bits(&self) -> usize {
        match self {
            Self::PublicValue { bits, .. } => *bits,
            Self::Array { element_type, num_elements } => element_type.size_in_bits() * num_elements,
            Self::Struct(elements) => elements.iter().map(Self::size_in_bits).sum(),
            Self::PublicPointerTo(_) => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToFunction(_) => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToHook(_) => Self::POINTER_SIZE_BITS,
            Self::PublicPointerToUnconstrainedPublic => Self::POINTER_SIZE_BITS,
            Self::Secret { bits } => *bits,
        }
    }

    /// Get the size of the nth (0-indexed) field/element of the `CompleteAbstractData`, in bits.
    /// The `CompleteAbstractData` must be a `Struct` or `Array`.
    pub fn field_size_in_bits(&self, n: usize) -> usize {
        match self {
            Self::Struct(elements) => Self::size_in_bits(&elements[n]),
            Self::Array { element_type, .. } => Self::size_in_bits(element_type),
            _ => panic!("field_size_in_bits called on {:?}", self),
        }
    }

    /// Get the offset of the nth (0-indexed) field/element of the `CompleteAbstractData`, in bits.
    /// The `CompleteAbstractData` must be a `Struct` or `Array`.
    pub fn offset_in_bits(&self, n: usize) -> usize {
        match self {
            Self::Struct(elements) => elements.iter().take(n).map(Self::size_in_bits).sum(),
            Self::Array { element_type, .. } => element_type.size_in_bits() * n,
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
    Struct(Vec<AbstractData>),
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

    /// A (first-class) array of values
    pub fn array_of(element_type: Self, num_elements: usize) -> Self {
        Self(UnderspecifiedAbstractData::Array { element_type: Box::new(element_type), num_elements })
    }

    /// A (first-class) structure of values
    pub fn struct_of(elements: impl IntoIterator<Item = Self>) -> Self {
        Self(UnderspecifiedAbstractData::Struct(elements.into_iter().collect()))
    }

    /// Just use the default structure based on the LLVM type and/or the `StructDescriptions`.
    ///
    /// See [`AbstractData::to_complete`](struct.AbstractData.html#method.to_complete)
    pub fn default() -> Self {
        Self(UnderspecifiedAbstractData::Unspecified)
    }

    /// A (public) pointer which may point anywhere
    pub fn unconstrained_pointer() -> Self {
        Self(UnderspecifiedAbstractData::Complete(CompleteAbstractData::PublicPointerToUnconstrainedPublic))
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
    pub fn to_complete(self, ty: &Type, sd: &StructDescriptions) -> CompleteAbstractData {
        self.0.to_complete(ty, sd)
    }

    /// `unspecified_named_structs`: set of struct names we have encountered
    /// which were given `UnderspecifiedAbstractData::Unspecified` and don't
    /// appear in `sd`. We keep track of these only so we can detect infinite
    /// recursion and abort with an appropriate error message.
    fn to_complete_rec<'a>(self, ty: &'a Type, sd: &StructDescriptions, unspecified_named_structs: HashSet<&'a String>) -> CompleteAbstractData {
        self.0.to_complete_rec(ty, sd, unspecified_named_structs)
    }
}

impl UnderspecifiedAbstractData {
    /// See method description on [`AbstractData::to_complete`](enum.AbstractData.html#method.to_complete)
    pub fn to_complete(self, ty: &Type, sd: &StructDescriptions) -> CompleteAbstractData {
        self.to_complete_rec(ty, sd, HashSet::new())
    }

    /// See method description on [`AbstractData::to_complete_rec`](enum.AbstractData.html#method.to_complete_rec)
    fn to_complete_rec<'a>(self, ty: &'a Type, sd: &StructDescriptions, mut unspecified_named_structs: HashSet<&'a String>) -> CompleteAbstractData {
        match self {
            Self::Complete(abstractdata) => abstractdata,
            Self::PublicPointerTo(ad) => match ty {
                Type::PointerType { pointee_type, .. } =>
                    CompleteAbstractData::PublicPointerTo(Box::new(match &ad.0 {
                        Self::Array { num_elements, .. } => {
                            // AbstractData is pointer-to-array, but LLVM type may be pointer-to-scalar
                            match &**pointee_type {
                                ty@Type::ArrayType { .. } | ty@Type::VectorType { .. } => {
                                    ad.to_complete_rec(ty, sd, unspecified_named_structs)  // LLVM type is array or vector as well, it matches
                                },
                                ty => {
                                    // LLVM type is scalar, but AbstractData is array, so it's actually pointer-to-array
                                    let num_elements = *num_elements;
                                    ad.to_complete_rec(&Type::ArrayType { element_type: Box::new(ty.clone()), num_elements }, sd, unspecified_named_structs)
                                },
                            }
                        },
                        _ => {
                            // AbstractData is pointer-to-something-else, just let the recursive call handle it
                            ad.to_complete_rec(&**pointee_type, sd, unspecified_named_structs)
                        },
                    })),
                Type::ArrayType { num_elements: 1, element_type } | Type::VectorType { num_elements: 1, element_type } => {
                    // auto-unwrap LLVM type if it is array or vector of one element
                    Self::PublicPointerTo(ad).to_complete_rec(&**element_type, sd, unspecified_named_structs)
                },
                _ => panic!("Type mismatch: AbstractData::PublicPointerTo but LLVM type is {:?}", ty),
            },
            Self::Array { element_type, num_elements } => match ty {
                Type::ArrayType { element_type: llvm_element_type, num_elements: llvm_num_elements }
                | Type::VectorType { element_type: llvm_element_type, num_elements: llvm_num_elements } => {
                    if *llvm_num_elements != 0 {
                        assert_eq!(num_elements, *llvm_num_elements, "Type mismatch: AbstractData specifies an array with {} elements, but found an array with {} elements", num_elements, llvm_num_elements);
                    }
                    CompleteAbstractData::Array { element_type: Box::new(element_type.to_complete_rec(&**llvm_element_type, sd, unspecified_named_structs)), num_elements }
                },
                _ => panic!("Type mismatch: AbstractData::Array with {} elements, but LLVM type is {:?}", num_elements, ty),
            }
            Self::Struct(v) => match ty {
                Type::NamedStructType { ty, .. } => Self::Struct(v).to_complete_rec(
                    &ty.as_ref()
                        .expect("Can't convert to complete with an opaque struct type")
                        .upgrade()
                        .expect("Failed to upgrade weak reference")
                        .read()
                        .unwrap(),
                    sd,
                    unspecified_named_structs,
                ),
                Type::StructType { element_types, .. } => CompleteAbstractData::Struct(
                    v.into_iter()
                    .zip(element_types)
                    .map(|(el_data, el_type)| el_data.to_complete_rec(el_type, sd, unspecified_named_structs.clone()))
                    .collect()
                ),
                Type::ArrayType { num_elements: 1, element_type } | Type::VectorType { num_elements: 1, element_type } => {
                    // auto-unwrap LLVM type if it is array or vector of one element
                    Self::Struct(v).to_complete_rec(&**element_type, sd, unspecified_named_structs)
                },
                _ => panic!("Type mismatch: AbstractData::Struct with {} elements, but LLVM type is {:?}", v.len(), ty),
            },
            Self::Unspecified => match ty {
                Type::IntegerType { .. } =>
                    CompleteAbstractData::PublicValue { bits: layout::size(ty), value: AbstractValue::Unconstrained },
                Type::PointerType { pointee_type, .. } => match &**pointee_type {
                    Type::FuncType { .. } =>
                        CompleteAbstractData::PublicPointerToHook("hook_uninitialized_function_pointer".to_owned()),
                    Type::IntegerType { bits } =>
                        CompleteAbstractData::PublicPointerTo(Box::new(CompleteAbstractData::Array {
                            element_type: Box::new(CompleteAbstractData::PublicValue { bits: *bits as usize, value: AbstractValue::Unconstrained}),
                            num_elements: AbstractData::DEFAULT_ARRAY_LENGTH,
                        })),
                    ty => CompleteAbstractData::PublicPointerTo(Box::new(Self::Unspecified.to_complete_rec(ty, sd, unspecified_named_structs))),
                },
                Type::VectorType { element_type, num_elements } | Type::ArrayType { element_type, num_elements } =>
                    CompleteAbstractData::Array {
                        element_type: Box::new(Self::Unspecified.to_complete_rec(element_type, sd, unspecified_named_structs)),
                        num_elements: if *num_elements == 0 { AbstractData::DEFAULT_ARRAY_LENGTH } else { *num_elements },
                    },
                Type::NamedStructType { ty, name } => {
                    let arc: Arc<RwLock<Type>> = ty.as_ref()
                        .unwrap_or_else(|| panic!("Can't convert to complete with an opaque struct type {:?}", name))
                        .upgrade()
                        .expect("Failed to upgrade weak reference");
                    let inner_ty: &Type = &arc.read().unwrap();
                    match sd.get(name) {
                        Some(abstractdata) => abstractdata.clone().to_complete_rec(inner_ty, sd, unspecified_named_structs),
                        None => {
                            if unspecified_named_structs.insert(name) {
                                self.to_complete_rec(inner_ty, sd, unspecified_named_structs)
                            } else {
                                panic!("AbstractData::default() applied to recursive struct {:?}", name)
                            }
                        },
                    }
                },
                Type::StructType { element_types, .. } => CompleteAbstractData::Struct(
                    element_types.iter()
                    .map(|el_type| Self::Unspecified.to_complete_rec(el_type, sd, unspecified_named_structs.clone()))
                    .collect()
                ),
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
}
