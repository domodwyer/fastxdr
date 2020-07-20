use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Typedef<'a> {
    pub target: BasicType<'a>,
    pub alias: ArrayType<BasicType<'a>>,
}

impl<'a> Typedef<'a> {
    pub fn new(mut vs: Vec<Node<'a>>) -> Self {
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

        // Optionally, extract the array definition so long as the target is not
        // opaque.
        //
        // Typedefs to opaque types include a variable array identifier so the
        // caller knows to read the length prefix bytes. This is already handled
        // by the opaque reader however, and confuses type resolution.
        let alias = if vs.len() > 0 && !target.is_opaque() {
            match vs.remove(0) {
                Node::ArrayFixed(s) => ArrayType::FixedSize(alias, ArraySize::from(s)),
                Node::ArrayVariable(s) => ArrayType::VariableSize(
                    alias,
                    match s.trim() {
                        "" => None,
                        s => Some(ArraySize::from(s)),
                    },
                ),
                _ => unreachable!("incorrect type in typedef"),
            }
        } else {
            ArrayType::None(alias)
        };

        Self {
            target: target,
            alias: alias,
        }
    }
}
