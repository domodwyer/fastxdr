use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Typedef {
    pub target: BasicType,
    pub alias: ArrayType<BasicType>,
}

impl Typedef {
    pub(crate) fn new<'a>(mut vs: Vec<Node<'a>>) -> Self {
        // Extract the target type
        let target = match vs.remove(0) {
            Node::Type(t) => t,
            _ => unreachable!("incorrect type in typedef"),
        };

        // Extract the defined alias
        let alias = match vs.remove(0) {
            Node::Type(t) => t,
            _ => unreachable!("incorrect type in typedef"),
        };

        // Optionally, extract the array definition
        let alias = if !vs.is_empty() {
            match vs.remove(0) {
                Node::ArrayFixed(s) => ArrayType::FixedSize(alias, ArraySize::from(s)),

                // Typedefs to opaque types include a variable array identifier so the
                // caller knows to read the length prefix bytes. This is already handled
                // by the opaque reader however, so map this to a "no array" wrapper.
                Node::ArrayVariable(_) if target.is_opaque() => ArrayType::None(alias),

                Node::ArrayVariable(s) => ArrayType::VariableSize(
                    alias,
                    match s.trim() {
                        "" => None,
                        s => Some(ArraySize::from(s)),
                    },
                ),
                t => unreachable!("incorrect type in typedef {:?}", t),
            }
        } else {
            ArrayType::None(alias)
        };

        Self { target, alias }
    }
}
