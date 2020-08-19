use crate::ast::Node;
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum ConstantType {
    ConstValue(String),
    EnumValue { enum_name: String, variant: String },
}

impl std::fmt::Display for ConstantType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConstValue(s) => write!(f, "{}", s),
            Self::EnumValue { enum_name, variant } => write!(f, "{}::{}", enum_name, variant),
        }
    }
}

#[derive(Debug)]
pub struct ConstantIndex(pub BTreeMap<String, ConstantType>);

impl ConstantIndex {
    /// Build an index of all consts / enums for use in the union switches.
    pub(crate) fn new<'a>(ast: &'a Node<'a>) -> ConstantIndex {
        let mut case_values = BTreeMap::new();
        if let Node::Root(r) = ast {
            for item in r.iter() {
                match item {
                    Node::Constant(vs) => {
                        // Map constants to themselves, they do not require namespacing.
                        if case_values
                            .insert(
                                vs[0].ident_str().to_string(),
                                ConstantType::ConstValue(vs[1].ident_str().to_string()),
                            )
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
                                .insert(
                                    v.name.as_str().to_string(),
                                    ConstantType::EnumValue {
                                        enum_name: e.name.to_string(),
                                        variant: v.name.as_str().to_string(),
                                    },
                                )
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
    pub fn get<T: AsRef<str>>(&self, name: T) -> Option<&ConstantType> {
        self.0.get(name.as_ref())
    }

    /// Iterates over the types in the constant index.
    pub fn iter(&self) -> impl std::iter::Iterator<Item = (&String, &ConstantType)> {
        self.0.iter()
    }
}
