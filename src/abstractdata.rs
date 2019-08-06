pub enum AbstractData {
    /// A public non-pointer value, of the given size in bits. If `value` is
    /// `Some`, then it is the actual concrete value; otherwise (if `value` is
    /// `None`) the value is unconstrained.
    PublicNonPointer { bits: usize, value: Option<u64> },
    /// A (first-class) array of values
    Array { element_type: Box<AbstractData>, num_elements: usize },
    /// A (first-class) structure of values
    Struct(Vec<AbstractData>),
    /// A (public) pointer to something - another value, an array, etc
    PublicPointer(Box<AbstractData>),
    /// A (public) pointer to unconstrained public data, which could be a public
    /// value, an array (of unconstrained size) of public values, or a public
    /// data structure
    PublicPointerToUnconstrainedPublic,
    /// A secret value (pointer or non-pointer, doesn't matter) of the given size in bits
    Secret { bits: usize },
}

impl AbstractData {
    const POINTER_SIZE_BITS: usize = 64;

    pub fn size(&self) -> usize {
        match self {
            AbstractData::PublicNonPointer { bits, .. } => *bits,
            AbstractData::Array { element_type, num_elements } => element_type.size() * num_elements,
            AbstractData::Struct(elements) => elements.iter().map(AbstractData::size).sum(),
            AbstractData::PublicPointer(_) => AbstractData::POINTER_SIZE_BITS,
            AbstractData::PublicPointerToUnconstrainedPublic => AbstractData::POINTER_SIZE_BITS,
            AbstractData::Secret { bits } => *bits,
        }
    }
}
