mod ast;
mod header;
mod impls;
mod indexes;

use crate::ast::*;
use crate::impls::from::bytes::{Bytes, RefMutBytes};
use crate::impls::*;
use crate::indexes::*;
use pest::Parser;
use pest_derive::Parser;

// The derive comment to be added to types.
const DERIVE: &'static str = "#[derive(Debug, PartialEq)]";

// The trait bound for T on structs/enums.
const TRAIT_BOUNDS: &'static str = "<T> where T: AsRef<[u8]> + Debug";

#[derive(Parser)]
#[grammar = "xdr.pest"]
pub struct XDRParser;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + 'static>>;

fn main() -> Result<()> {
    let mut ast = XDRParser::parse(Rule::item, include_str!("nfs.x"))?;

    // Print the file header
    println!("{}", include_str!("header.rs"));

    let ast = walk(ast.next().unwrap()).unwrap();

    let case_index = build_constant_index(&ast);
    let generic_index = build_generic_index(&ast);
    let type_index = TypeIndex::new(&ast);

    // dbg!(case_index);
    // dbg!(generic_index);
    // dbg!(type_index);
    // return Ok(());

    let mut out = String::new();
    print_types(&mut out, &ast, &generic_index)?;
    from::print_impl_from(&mut out, Bytes, &generic_index, &case_index, &type_index)?;
    from::print_impl_from(
        &mut out,
        RefMutBytes,
        &generic_index,
        &case_index,
        &type_index,
    )?;
    println!("{}", out);

    Ok(())
}
