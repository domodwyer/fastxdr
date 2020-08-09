use crate::ast::Node;
use std::collections::BTreeMap;

pub struct ConstantIndex<'a>(pub BTreeMap<&'a str, String>);

impl<'a> ConstantIndex<'a> {
    /// Build an index of all consts / enums for use in the union switches.
    pub(crate) fn new(ast: &'a Node) -> ConstantIndex<'a> {
        let mut case_values = BTreeMap::new();
        if let Node::Root(r) = ast {
            for item in r.iter() {
                match item {
                    Node::Constant(vs) => {
                        // Map constants to themselves, they do not require namespacing.
                        if case_values
                            .insert(vs[0].ident_str(), vs[1].ident_str().to_string())
                            .is_some()
                        {
                            panic!("duplicate case keys {}", vs[0].ident_str());
                        }
                    }
                    Node::Enum(e) => {
                        // Enums require namespacing.
                        //
                        // `NFS_OK` defined in a hypothetical `Status` enum must
                        // become `Status::NFS_OK`.
                        for v in e.variants.iter() {
                            if case_values
                                .insert(v.name.as_str(), format!("{}::{}", e.name, v.name.as_str()))
                                .is_some()
                            {
                                panic!("duplicate case keys {}", v.name.as_str());
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }

        ConstantIndex(case_values)
    }

    /// Returns the constant value as a string for `name`.
    pub fn get<T: AsRef<str>>(&self, name: T) -> Option<&String> {
        self.0.get(name.as_ref())
    }

    /// Iterates over the types in the constant index.
    pub fn iter(&self) -> impl std::iter::Iterator<Item = (&&str, &String)> {
        self.0.iter()
    }
}
