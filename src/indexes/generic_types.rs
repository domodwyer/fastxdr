use crate::ast::{BasicType, CompoundType, Node};
use std::collections::HashSet;

#[derive(Debug)]
pub struct GenericIndex(pub HashSet<String>);

impl GenericIndex {
    pub(crate) fn new<'a>(ast: &'a Node<'a>) -> GenericIndex {
        // Define a recursive ast walker that visits all values in the tree, looking
        // for "opaque" types.
        //
        // Types containing opaque types, and types containing those types (and so
        // on) are added to index to build a full set of type names that require
        // generic bounds.
        fn recurse<'a>(v: &'a Node<'a>, index: &mut HashSet<String>) -> bool {
            // Get the type name to see if it is already marked.
            //
            // Only structs, unions and typedefs can contain sub-types that may be
            // generic.
            let name = match v {
                Node::Struct(v) => Some(v.name()),
                Node::Union(v) => Some(v.name()),
                Node::Typedef(v) => Some(v.alias.unwrap_array().as_str()),
                _ => None,
            };

            // Stop if the type is already marked as needing a generic bounds
            if let Some(name) = name {
                if index.contains(name) {
                    return true;
                }
            }

            // Otherwise recurse into children looking for an "opaque" data type
            let contains_opaque = match v {
                Node::Type(v) => match v {
                    BasicType::Opaque => true,
                    BasicType::Ident(i) => index.contains(i.as_str()),
                    _ => false,
                },

                // These Nodes can contain inner opaque types, or contain compound
                // types that themselves contain opaques.
                Node::Struct(v) => v.inner_types().iter().any(|t| match t.unwrap_array() {
                    BasicType::Opaque => true,
                    BasicType::Ident(i) => index.contains(i.as_str()),
                    _ => false,
                }),
                Node::Union(v) => v.inner_types().iter().any(|t| match t.unwrap_array() {
                    BasicType::Opaque => true,
                    BasicType::Ident(i) => index.contains(i.as_str()),
                    _ => false,
                }),

                Node::Typedef(v) => match v.target {
                    BasicType::Opaque => true,
                    _ => index.contains(v.target.as_str()),
                },

                Node::Root(v) => v.iter().fold(false, |mut acc, v| {
                    if recurse(v, index) {
                        acc = true;
                    }
                    acc
                }),

                // These Nodes will never contain an opaque/generic type.
                Node::EOF
                | Node::Enum(_)
                | Node::Constant(_)
                | Node::EnumVariant(_)
                | Node::ArrayVariable(_)
                | Node::ArrayFixed(_) => false,

                // These nodes are not reachable in the tree
                Node::Ident(_)
                | Node::UnionDefault(_)
                | Node::UnionCase(_)
                | Node::Option(_)
                | Node::UnionDataField(_)
                | Node::UnionVoid
                | Node::StructDataField(_)
                | Node::Array(_) => unreachable!("{:?}", &v),
            };

            // If there was a type name, and it contains an opaque type, add it to
            // the "needs a generic" type index and return.
            if let Some(name) = name {
                if contains_opaque {
                    index.insert(name.to_string());
                    return true;
                }
            }

            contains_opaque
        };

        let mut index: HashSet<String> = HashSet::new();
        let mut last_size: isize = -1;
        while last_size != index.len() as isize {
            last_size = index.len() as isize;
            recurse(ast, &mut index);
        }

        GenericIndex(index)
    }

    pub fn contains(&self, ident: &str) -> bool {
        self.0.contains(ident)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_pushup_structs() {
        let input = r#"
struct stateid4 {
        uint32_t        seqid;
        opaque          other[NFS4_OTHER_SIZE];
};
struct generic_field {
        stateid4        inner;
};
struct another {
        generic_field        inner;
};
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();
        let got = ast.generics();

        let mut want = HashSet::new();
        want.insert("stateid4");
        want.insert("generic_field");
        want.insert("another");

        let mut got: Vec<String> = got.0.iter().cloned().collect();
        let mut want: Vec<&str> = want.iter().cloned().collect();

        let got = got.as_mut_slice();
        let want = want.as_mut_slice();

        got.sort();
        want.sort();

        assert_eq!(got, want);
    }

    #[test]
    fn test_generic_pushup_structs_reversed() {
        let input = r#"
struct another {
        generic_field        inner;
};
struct generic_field {
        stateid4        inner;
};
struct stateid4 {
        uint32_t        seqid;
        opaque          other[NFS4_OTHER_SIZE];
};
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();
        let got = ast.generics();

        let mut want = HashSet::new();
        want.insert("stateid4");
        want.insert("generic_field");
        want.insert("another");

        let mut got: Vec<String> = got.0.iter().cloned().collect();
        let mut want: Vec<&str> = want.iter().cloned().collect();

        let got = got.as_mut_slice();
        let want = want.as_mut_slice();

        got.sort();
        want.sort();

        assert_eq!(got, want);
    }

    #[test]
    fn test_generic_pushup_typedef() {
        let input = r#"
typedef opaque  attrlist4<>;
typedef opaque  nfs_fh4<NFS4_FHSIZE>;
struct generic_field {
        attrlist4        inner;
};
struct another {
        nfs_fh4        inner;
};
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();
        let got = ast.generics();

        let mut want = HashSet::new();
        want.insert("attrlist4");
        want.insert("generic_field");

        want.insert("nfs_fh4");
        want.insert("another");

        let mut got: Vec<String> = got.0.iter().cloned().collect();
        let mut want: Vec<&str> = want.iter().cloned().collect();

        let got = got.as_mut_slice();
        let want = want.as_mut_slice();

        got.sort();
        want.sort();

        assert_eq!(got, want);
    }

    #[test]
    fn test_generic_pushup_union() {
        let input = r#"
struct READ4resok {
        bool            eof;
        opaque          data<>;
};

union READ4res switch (nfsstat4 status) {
 case NFS4_OK:
         READ4resok     resok4;
 default:
         void;
};
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();
        let got = ast.generics();

        let mut want = HashSet::new();

        want.insert("READ4resok");
        want.insert("READ4res");

        let mut got: Vec<String> = got.0.iter().cloned().collect();
        let mut want: Vec<&str> = want.iter().cloned().collect();

        let got = got.as_mut_slice();
        let want = want.as_mut_slice();

        got.sort();
        want.sort();

        assert_eq!(got, want);
    }
}
