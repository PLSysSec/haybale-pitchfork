use haybale::layout;
use llvm_ir::Type;

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
    /// Get the default `AbstractData` for a value of the given LLVM type.
    /// Those defaults are:
    ///
    /// Integer type: public unconstrained value of the appropriate size
    /// Pointer type (except function pointer): public concrete pointer value to allocated memory, depending on pointer type:
    ///   pointee is an integer type: pointer to allocated array of DEFAULT_ARRAY_LENGTH pointees (e.g., default for char* is pointer to array of 1024 chars)
    ///   pointee is any other type: pointer to one of that other type
    ///   (then in either case, apply these rules recursively to each pointee type)
    /// Function pointer type: concrete function pointer value which, when called, will raise an error
    /// Vector or array type: array of the appropriate length, containing public values
    ///   (unless the number of elements is 0, in which case, we default to DEFAULT_ARRAY_LENGTH elements)
    ///   (in any case, apply these rules recursively to each element)
    /// Structure type: apply these rules recursively to each field
    pub fn default_for(ty: &Type) -> Self {
        match ty {
            Type::IntegerType { .. } => AbstractData::PublicValue { bits: layout::size(ty), value: AbstractValue::Unconstrained },
            Type::PointerType { pointee_type, .. } => match &**pointee_type {
                Type::FuncType { .. } =>
                    AbstractData::PublicPointerToHook("hook_uninitialized_function_pointer".to_owned()),
                Type::IntegerType { bits } =>
                    AbstractData::PublicPointerTo(Box::new(AbstractData::Array {
                        element_type: Box::new(AbstractData::PublicValue { bits: *bits as usize, value: AbstractValue::Unconstrained}),
                        num_elements: AbstractData::DEFAULT_ARRAY_LENGTH,
                    })),
                ty => AbstractData::PublicPointerTo(Box::new(Self::default_for(ty))),
            },
            Type::VectorType { element_type, num_elements } | Type::ArrayType { element_type, num_elements } =>
                AbstractData::Array {
                    element_type: Box::new(Self::default_for(element_type)),
                    num_elements: if *num_elements == 0 { AbstractData::DEFAULT_ARRAY_LENGTH } else { *num_elements },
                },
            Type::StructType { element_types, .. } => AbstractData::Struct(element_types.iter().map(Self::default_for).collect()),
            Type::NamedStructType { ty, .. } => Self::default_for(
                &ty.as_ref()
                .expect("Can't get default AbstractData for an opaque struct type")
                .upgrade()
                .expect("Failed to upgrade weak reference")
                .read()
                .unwrap()
            ),
            ty => unimplemented!("AbstractData::default_for {:?}", ty),
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
