use haybale::layout;
use llvm_ir::Type;

/// An abstract description of a value: if it is public or not, if it is a
/// pointer or not, does it point to data that is public/secret, maybe it's a
/// struct with some public and some secret fields, etc.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AbstractData {
    /// A public value, of the given size in bits. If `value` is `Some`, then it
    /// is the actual concrete value; otherwise (if `value` is `None`) the value
    /// is unconstrained.
    ///
    /// This may be used for either a non-pointer value, or for a pointer value
    /// if you want to specify the exact numerical value of the pointer (e.g. NULL).
    PublicValue { bits: usize, value: AbstractValue },

    /// A (first-class) array of values
    Array { element_type: Box<AbstractData>, num_elements: usize },

    /// A (first-class) structure of values
    Struct(Vec<AbstractData>),

    /// A (public) pointer to something - another value, an array, etc
    PublicPointerTo(Box<AbstractData>),

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

impl AbstractData {
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

    /// a (public) pointer to something - another value, an array, etc
    pub fn pub_pointer_to(data: Self) -> Self {
        Self::PublicPointerTo(Box::new(data))
    }
}

impl AbstractData {
    pub const POINTER_SIZE_BITS: usize = 64;
    pub const DEFAULT_ARRAY_LENGTH: usize = 1024;

    /// Get the size of the `AbstractData`, in bits
    pub fn size_in_bits(&self) -> usize {
        match self {
            AbstractData::PublicValue { bits, .. } => *bits,
            AbstractData::Array { element_type, num_elements } => element_type.size_in_bits() * num_elements,
            AbstractData::Struct(elements) => elements.iter().map(AbstractData::size_in_bits).sum(),
            AbstractData::PublicPointerTo(_) => AbstractData::POINTER_SIZE_BITS,
            AbstractData::PublicPointerToFunction(_) => AbstractData::POINTER_SIZE_BITS,
            AbstractData::PublicPointerToHook(_) => AbstractData::POINTER_SIZE_BITS,
            AbstractData::PublicPointerToUnconstrainedPublic => AbstractData::POINTER_SIZE_BITS,
            AbstractData::Secret { bits } => *bits,
        }
    }

    /// Get the offset of the nth (0-indexed) field/element of the `AbstractData`, in bits.
    /// The `AbstractData` must be a `Struct` or `Array`.
    pub fn offset_in_bits(&self, n: usize) -> usize {
        match self {
            AbstractData::Struct(elements) => elements.iter().take(n).map(AbstractData::size_in_bits).sum(),
            AbstractData::Array { element_type, .. } => element_type.size_in_bits() * n,
            _ => panic!("offset_in_bits called on {:?}", self),
        }
    }
}

/// Like `AbstractData`, but includes options for parts of the value (or the
/// whole value) to be `Unspecified`, meaning to just use the default based on
/// the LLVM type.
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum UnderspecifiedAbstractData {
    /// Just use the default structure based on the LLVM type.
    ///
    /// See [`UnderspecifiedAbstractData::convert_to_fully_specified_as`](enum.UnderspecifiedAbstractData.html#method.convert_to_fully_specified_as)
    Unspecified,

    /// Use the given fully specified `AbstractData`
    FullySpecified(AbstractData),

    /// A (public) pointer to something underspecified
    PublicPointerTo(Box<UnderspecifiedAbstractData>),

    /// an array with underspecified elements
    Array { element_type: Box<UnderspecifiedAbstractData>, num_elements: usize },

    /// a struct with underspecified fields
    /// (for instance, some unspecified and some fully-specified fields)
    Struct(Vec<UnderspecifiedAbstractData>),
}

impl UnderspecifiedAbstractData {
    /// an 8-bit public value
    pub fn pub_i8(value: AbstractValue) -> Self {
        Self::FullySpecified(AbstractData::pub_i8(value))
    }

    /// a 16-bit public value
    pub fn pub_i16(value: AbstractValue) -> Self {
        Self::FullySpecified(AbstractData::pub_i16(value))
    }

    /// a 32-bit public value
    pub fn pub_i32(value: AbstractValue) -> Self {
        Self::FullySpecified(AbstractData::pub_i32(value))
    }

    /// a 64-bit public value
    pub fn pub_i64(value: AbstractValue) -> Self {
        Self::FullySpecified(AbstractData::pub_i64(value))
    }

    /// an 8-bit secret value
    pub fn sec_i8() -> Self {
        Self::FullySpecified(AbstractData::sec_i8())
    }

    /// a 16-bit secret value
    pub fn sec_i16() -> Self {
        Self::FullySpecified(AbstractData::sec_i16())
    }

    /// a 32-bit secret value
    pub fn sec_i32() -> Self {
        Self::FullySpecified(AbstractData::sec_i32())
    }

    /// a 64-bit secret value
    pub fn sec_i64() -> Self {
        Self::FullySpecified(AbstractData::sec_i64())
    }

    /// A (public) pointer to something fully specified - another value, an array, etc
    pub fn pub_pointer_to(data: AbstractData) -> Self {
        Self::FullySpecified(AbstractData::PublicPointerTo(Box::new(data)))
    }

    /// A (public) pointer to something underspecified - another value, an array, etc
    pub fn pub_pointer_to_underspec(data: UnderspecifiedAbstractData) -> Self {
        Self::PublicPointerTo(Box::new(data))
    }
}

impl UnderspecifiedAbstractData {
    /// Fill in the default `AbstractData` for any parts of the
    /// `UnderspecifiedAbstractData` which are unspecified, using the information
    /// in the given LLVM type.
    ///
    /// The default `AbstractData` based on the LLVM type is:
    ///
    /// - for LLVM integer type: public unconstrained value of the appropriate size
    /// - for LLVM pointer type (except function pointer): public concrete pointer value to allocated memory, depending on pointer type:
    ///   - pointee is an integer type: pointer to allocated array of DEFAULT_ARRAY_LENGTH pointees (e.g., default for char* is pointer to array of 1024 chars)
    ///   - pointee is any other type: pointer to one of that other type
    ///   - (then in either case, apply these rules recursively to each pointee type)
    /// - for LLVM function pointer type: concrete function pointer value which, when called, will raise an error
    /// - for LLVM vector or array type: array of the appropriate length, containing public values
    ///   (unless the number of elements is 0, in which case, we default to DEFAULT_ARRAY_LENGTH elements)
    ///   - (in any case, apply these rules recursively to each element)
    /// - for LLVM structure type: apply these rules recursively to each field
    pub fn convert_to_fully_specified_as(self, ty: &Type) -> AbstractData {
        if let Type::NamedStructType { ty, .. } = ty {
            self.convert_to_fully_specified_as(
                &ty.as_ref()
                .expect("Can't convert to fully specified as an opaque struct type")
                .upgrade()
                .expect("Failed to upgrade weak reference")
                .read()
                .unwrap()
            )
        } else {
            match self {
                Self::FullySpecified(abstractdata) => abstractdata,
                Self::PublicPointerTo(uad) => match ty {
                    Type::PointerType { pointee_type, .. } =>
                        AbstractData::PublicPointerTo(Box::new(uad.convert_to_fully_specified_as(&**pointee_type))),
                    _ => panic!("Type mismatch: UnderspecifiedAbstractData::PublicPointerTo but LLVM type is {:?}", ty),
                },
                Self::Array { element_type, num_elements } => match ty {
                    Type::ArrayType { element_type: llvm_element_type, num_elements: llvm_num_elements } => {
                        if *llvm_num_elements != 0 {
                            assert_eq!(num_elements, *llvm_num_elements, "Type mismatch: AbstractData specifies an array with {} elements, but found an array with {} elements", num_elements, llvm_num_elements);
                        }
                        AbstractData::Array { element_type: Box::new(element_type.convert_to_fully_specified_as(&**llvm_element_type)), num_elements }
                    },
                    Type::PointerType { pointee_type, .. } =>
                        AbstractData::Array { element_type: Box::new(element_type.convert_to_fully_specified_as(&**pointee_type)), num_elements },
                    _ => panic!("Type mismatch: UnderspecifiedAbstractData::Array but LLVM type is {:?}", ty),
                }
                Self::Struct(v) => match ty {
                    Type::StructType { element_types, .. } => AbstractData::Struct(
                        v.into_iter()
                        .zip(element_types)
                        .map(|(el_data, el_type)| el_data.convert_to_fully_specified_as(el_type))
                        .collect()
                    ),
                    Type::NamedStructType { .. } => panic!("This case should have been already handled above"),
                    _ => panic!("Type mismatch: UnderspecifiedAbstractData::Struct but LLVM type is {:?}", ty),
                },
                Self::Unspecified => match ty {
                    Type::IntegerType { .. } =>
                        AbstractData::PublicValue { bits: layout::size(ty), value: AbstractValue::Unconstrained },
                    Type::PointerType { pointee_type, .. } => match &**pointee_type {
                        Type::FuncType { .. } =>
                            AbstractData::PublicPointerToHook("hook_uninitialized_function_pointer".to_owned()),
                        Type::IntegerType { bits } =>
                            AbstractData::PublicPointerTo(Box::new(AbstractData::Array {
                                element_type: Box::new(AbstractData::PublicValue { bits: *bits as usize, value: AbstractValue::Unconstrained}),
                                num_elements: AbstractData::DEFAULT_ARRAY_LENGTH,
                            })),
                        ty => AbstractData::PublicPointerTo(Box::new(Self::Unspecified.convert_to_fully_specified_as(ty))),
                    },
                    Type::VectorType { element_type, num_elements } | Type::ArrayType { element_type, num_elements } =>
                        AbstractData::Array {
                            element_type: Box::new(Self::Unspecified.convert_to_fully_specified_as(element_type)),
                            num_elements: if *num_elements == 0 { AbstractData::DEFAULT_ARRAY_LENGTH } else { *num_elements },
                        },
                    Type::StructType { element_types, .. } => AbstractData::Struct(
                        element_types.iter()
                        .map(|el_type| Self::Unspecified.convert_to_fully_specified_as(el_type))
                        .collect()
                    ),
                    _ => unimplemented!("UnderspecifiedAbstractData::convert_to_fully_specified_as {:?}", ty),
                },
            }
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
