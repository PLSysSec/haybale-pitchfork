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

    pub fn size(&self) -> usize {
        match self {
            AbstractData::PublicValue { bits, .. } => *bits,
            AbstractData::Array { element_type, num_elements } => element_type.size() * num_elements,
            AbstractData::Struct(elements) => elements.iter().map(AbstractData::size).sum(),
            AbstractData::PublicPointerTo(_) => AbstractData::POINTER_SIZE_BITS,
            AbstractData::PublicPointerToFunction(_) => AbstractData::POINTER_SIZE_BITS,
            AbstractData::PublicPointerToHook(_) => AbstractData::POINTER_SIZE_BITS,
            AbstractData::PublicPointerToUnconstrainedPublic => AbstractData::POINTER_SIZE_BITS,
            AbstractData::Secret { bits } => *bits,
        }
    }
}
