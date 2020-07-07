use super::*;
use crate::Rule;
use pest::iterators::Pair;
use std::convert::TryFrom;

#[derive(Debug, PartialEq)]
pub enum Node<'a> {
    Ident(&'a str),
    Type(BasicType<'a>),
    Option(Vec<Node<'a>>),
    Struct(Struct<'a>),
    Union(Union<'a>),
    UnionCase(Vec<Node<'a>>),
    UnionDefault(Vec<Node<'a>>),
    UnionVoid,
    StructDataField(Vec<Node<'a>>),
    UnionDataField(Vec<Node<'a>>),
    Array(Vec<Node<'a>>),
    ArrayVariable(&'a str),
    ArrayFixed(&'a str),
    Typedef(Typedef<'a>),
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

    #[cfg(test)]
    pub fn unwrap_struct(&'a self) -> &Struct<'a> {
        if let Self::Struct(s) = self {
            return s;
        }
        panic!("unwrap_struct not a struct")
    }

    #[cfg(test)]
    pub fn unwrap_union(&'a self) -> &Union<'a> {
        if let Self::Union(s) = self {
            return s;
        }
        panic!("unwrap_union not a union")
    }

    #[cfg(test)]
    pub fn unwrap_enum(&self) -> &Enum {
        if let Self::Enum(s) = self {
            return s;
        }
        panic!("unwrap_enum not a enum")
    }
}

pub(crate) fn walk<'a>(ast: Pair<'a, Rule>) -> Result<Node, Box<dyn std::error::Error + 'static>> {
    fn collect_values<'a>(ast: Pair<'a, Rule>) -> Vec<Node> {
        ast.into_inner().map(|v| walk(v).unwrap()).collect()
    }

    let x = match ast.as_rule() {
        Rule::item => Node::Root(collect_values(ast)),
        Rule::typedef => Node::Typedef(Typedef::new(collect_values(ast))),
        Rule::constant => Node::Constant(collect_values(ast)),
        Rule::ident | Rule::ident_const | Rule::ident_value => {
            if let Ok(t) = BasicType::try_from(ast.as_str()) {
                Node::Type(t)
            } else {
                Node::Ident(ast.as_str())
            }
        }
        Rule::enum_type => Node::Enum(Enum::new(collect_values(ast))),
        Rule::enum_variant => Node::EnumVariant(collect_values(ast)),
        Rule::array => Node::Array(collect_values(ast)),
        Rule::array_variable => Node::ArrayVariable(ast.into_inner().as_str()),
        Rule::array_fixed => Node::ArrayFixed(ast.into_inner().as_str()),
        Rule::struct_type => Node::Struct(Struct::new(collect_values(ast))),
        Rule::struct_data_field => Node::StructDataField(collect_values(ast)),
        Rule::union_data_field => Node::UnionDataField(collect_values(ast)),
        Rule::union => Node::Union(Union::new(collect_values(ast))),
        Rule::union_case => Node::UnionCase(collect_values(ast)),
        Rule::union_default => Node::UnionDefault(collect_values(ast)),
        Rule::union_void => Node::UnionVoid,
        Rule::option => Node::Option(collect_values(ast)),
        Rule::basic_type => {
            Node::Type(BasicType::try_from(ast.as_str()).expect("unrecognised type"))
        }
        Rule::EOI => Node::EOF,
        e => unimplemented!("{:?}", e),
    };

    Ok(x)
}
