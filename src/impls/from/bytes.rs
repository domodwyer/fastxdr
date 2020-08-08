use super::{FromTemplate, ReferenceType};

#[derive(Debug, Clone, Copy)]
pub struct Bytes;

impl FromTemplate for Bytes {
    fn type_name(&self) -> &'static str {
        "Bytes"
    }

    fn try_from(&self) -> &'static str {
        "Bytes"
    }

    fn ref_type(&self) -> ReferenceType {
        ReferenceType::ByRef
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RefMutBytes;

impl FromTemplate for RefMutBytes {
    fn type_name(&self) -> &'static str {
        "Bytes"
    }

    fn try_from(&self) -> &'static str {
        "&mut Bytes"
    }

    fn ref_type(&self) -> ReferenceType {
        ReferenceType::ByValue
    }
}
