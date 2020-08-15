pub mod bytes;
pub trait FromTemplate: Copy {
    fn type_name(&self) -> &'static str;
    fn try_from(&self) -> &'static str;
    fn ref_type(&self) -> ReferenceType;
}

/// ReferenceType defines how the generated code should pass type instances when
/// decoding nested data structures.
#[derive(Debug, Clone, Copy)]
pub enum ReferenceType {
    ByValue,
    ByRef,
}

impl std::fmt::Display for ReferenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ReferenceType::ByValue => "&mut *v",
                ReferenceType::ByRef => "&mut v",
            }
        )
    }
}