use super::*;
#[derive(Debug, PartialEq)]
pub(crate) enum Node<'a> {
    Ident(&'a str),
    Type(BasicType),
    Option(Vec<Node<'a>>),
    Struct(Struct),
    Union(Union),
    UnionCase(Vec<Node<'a>>),
    UnionDefault(Vec<Node<'a>>),
    UnionVoid,
    StructDataField(Vec<Node<'a>>),
    UnionDataField(Vec<Node<'a>>),
    Array(Vec<Node<'a>>),
    ArrayVariable(&'a str),
    ArrayFixed(&'a str),
    Typedef(Typedef),
    Constant(Vec<Node<'a>>),
    Enum(Enum),
    EnumVariant(Vec<Node<'a>>),
    Root(Vec<Node<'a>>),

    EOF,
}

impl<'a> Node<'a> {
    pub fn ident_str(&'a self) -> &'a str {
        match self {
            Node::Ident(v) => match v.trim() {
                "type" => "type_t",
                v => v,
            },
            Node::Type(v) => v.as_str(),
            Node::Option(v) => v[0].ident_str(),
            _ => panic!("not an ident"),
        }
    }

    #[cfg(test)]
    pub fn into_inner(self) -> Vec<Node<'a>> {
        match self {
            Self::Option(v) => v,
            Self::UnionCase(v) => v,
            Self::UnionDefault(v) => v,
            Self::StructDataField(v) => v,
            Self::UnionDataField(v) => v,
            Self::Array(v) => v,
            Self::Constant(v) => v,
            Self::EnumVariant(v) => v,
            Self::Root(v) => v,
            _ => panic!("no node inner"),
        }
    }
}
