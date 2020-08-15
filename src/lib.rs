//! To generate Rust types from an XDR spec at build time, add `fastxdr` to your
//! `Cargo.toml`:
//!
//! ```toml
//! [build-dependencies]
//! # For the code generation
//! fastxdr = "1.0"
//!
//! [dependencies]
//! # Required dependencies of the generated code
//! thiserror = "1.0"
//! bytes = "0.5"
//! ```
//!
//! And add a `build.rs` at the crate root (not in `src`!):
//!
//! ```no_run
//! # std::env::set_var("OUT_DIR", "./");
//! fn main() {
//!     // Tell Cargo to regenerate the types if the XDR spec changes
//!     println!("cargo:rerun-if-changed=xdr_spec.x");
//!
//!     // Read from xdr_spec.x, writing the generated code to out.rs
//!     std::fs::write(
//!         std::path::Path::new(std::env::var("OUT_DIR").unwrap().as_str()).join("out.rs"),
//!         fastxdr::Generator::default()
//!             .generate(include_str!("xdr_spec.x"))
//!             .unwrap(),
//!     )
//!     .unwrap();
//! }
//! ```
//!
//! And then include the generated file in your application:
//!
//! ```compile_fail
//! // Where out.rs is the filename from above
//! include!(concat!(env!("OUT_DIR"), "/out.rs"));
//! use xdr::*;
//! ```
//!
//! To view the generated types, either export the generated types in your
//! application and use `cargo doc`, or use the CLI to produce the generated
//! code directly for reading.

#![allow(clippy::needless_doctest_main)]

pub mod ast;
pub mod impls;
pub mod indexes;

use crate::ast::*;
use crate::impls::{print_impl_from, print_impl_wire_size, print_types, template};
use crate::indexes::*;
use pest::Parser;
use pest_derive::Parser;
use std::fmt::Write;

#[derive(Parser)]
#[grammar = "xdr.pest"]
pub(crate) struct XDRParser;

pub const DEFAULT_DERIVE: &str = "#[derive(Debug, PartialEq)]";

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + 'static>>;

pub struct Generator {
    derive: String,
}

impl std::default::Default for Generator {
    fn default() -> Self {
        Generator {
            derive: DEFAULT_DERIVE.to_string(),
        }
    }
}

impl Generator {
    pub fn with_derive<D: AsRef<str>>(self, derive: D) -> Self {
        Self {
            derive: derive.as_ref().to_string(),
        }
    }

    pub fn generate<T: AsRef<str>>(&self, xdr: T) -> Result<String> {
        // Tokenise the input
        let mut root = crate::XDRParser::parse(Rule::item, xdr.as_ref())?;
        // Parse into an AST
        let ast = walk(root.next().unwrap())?;

        // Build some helpful indexes to answer questions about types when
        // generating the Rust code.
        let constant_index = ConstantIndex::new(&ast);
        let generic_index = GenericIndex::new(&ast);
        let type_index = TypeIndex::new(&ast);

        let mut out = String::new();

        // Print the file header
        writeln!(out, "{}", include_str!("header.rs"))?;

        // Generate the types
        print_types(&mut out, &ast, &generic_index, self.derive.as_str())?;

        // Write the two from traits, one for Bytes and one for &mut Bytes
        print_impl_from(
            &mut out,
            template::bytes::Bytes,
            &generic_index,
            &constant_index,
            &type_index,
        )?;
        print_impl_from(
            &mut out,
            template::bytes::RefMutBytes,
            &generic_index,
            &constant_index,
            &type_index,
        )?;

        // Write the wire_size() implementations
        print_impl_wire_size(
            &mut out,
            template::bytes::Bytes,
            &type_index,
            &generic_index,
        )?;

        // End the header.rs with a closing }
        writeln!(out, "}}")?;

        Ok(out)
    }
}
