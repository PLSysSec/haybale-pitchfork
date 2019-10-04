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
