use crate::ast::Node;
use std::collections::BTreeMap;

/// Build an index of all consts / enums for use in the union switches.
pub fn build_constant_index<'a>(ast: &'a Node) -> BTreeMap<&'a str, String> {
    let mut case_values = BTreeMap::new();
    if let Node::Root(r) = ast {
        for item in r.iter() {
            match item {
                Node::Constant(vs) => {
                    // Map constants to themselves, they do not require namespacing.
                    if case_values
                        .insert(vs[0].ident_str(), vs[0].ident_str().to_string())
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

    case_values
}
