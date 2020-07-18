use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Typedef<'a> {
    pub target: BasicType<'a>,
    pub alias: BasicType<'a>,
}

impl<'a> Typedef<'a> {
    pub fn new(mut vs: Vec<Node<'a>>) -> Self {
        // TODO: handle arrays in typedefs
        Self {
            target: match vs.remove(0) {
                Node::Type(t) => t,
                _ => unreachable!("incorrect type in typedef"),
            },
            alias: match vs.remove(0) {
                Node::Type(t) => t,
                _ => unreachable!("incorrect type in typedef"),
            },
        }
    }
}
