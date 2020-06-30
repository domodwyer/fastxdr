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

pub trait CompoundType {
    fn inner_types(&self) -> Vec<&ArrayType<BasicType>>;
    fn contains_opaque(&self) -> bool;
}
