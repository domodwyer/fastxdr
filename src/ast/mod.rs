mod basic_type;
pub use basic_type::*;

mod structure;
pub use structure::*;

mod union;
pub use union::*;

mod enumeration;
pub use enumeration::*;

mod node;
pub use node::*;

mod array;
pub use array::*;

mod typedef;
pub use typedef::*;

pub trait CompoundType {
    fn inner_types(&self) -> Vec<&ArrayType<BasicType>>;
    fn contains_opaque(&self) -> bool;
}

use crate::indexes::*;
use crate::Result;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::convert::TryFrom;

#[derive(Parser)]
#[grammar = "xdr.pest"]
pub(crate) struct XDRParser;

pub struct Ast {
    constant_index: ConstantIndex,
    generic_index: GenericIndex,
    type_index: TypeIndex,
}

impl Ast {
    pub fn new<'s>(xdr: &'s str) -> Result<Self> {
        // Tokenise the input
        let mut root = XDRParser::parse(Rule::item, xdr)?;
        // Parse into an AST
        let ast = walk(root.next().unwrap())?;

        // Build some helpful indexes to answer questions about types when
        // generating the Rust code.
        let constant_index = ConstantIndex::new(&ast);
        let generic_index = GenericIndex::new(&ast);
        let type_index = TypeIndex::new(&ast);

        Ok(Ast {
            constant_index,
            generic_index,
            type_index,
        })
    }

    pub fn constants(&self) -> &ConstantIndex {
        &self.constant_index
    }

    pub fn generics(&self) -> &GenericIndex {
        &self.generic_index
    }

    pub fn types(&self) -> &TypeIndex {
        &self.type_index
    }
}

pub(crate) fn walk(
    ast: Pair<Rule>,
) -> std::result::Result<Node, Box<dyn std::error::Error + 'static>> {
    fn collect_values(ast: Pair<Rule>) -> Vec<Node> {
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
