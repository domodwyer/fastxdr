use crate::ast::{Enum, Node, Struct, Typedef, Union};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AstType {
    Struct(Struct),
    Union(Union),
    Enum(Enum),
    Typedef(Typedef),
}

impl std::fmt::Display for AstType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AstType::Struct(s) => s.name(),
                AstType::Union(s) => s.name(),
                AstType::Enum(s) => s.name.as_str(),
                AstType::Typedef(s) => s.target.as_str(),
            }
        )
    }
}

#[derive(Debug)]
pub struct TypeIndex(pub BTreeMap<String, AstType>);

impl TypeIndex {
    pub(crate) fn new<'n>(ast: &'n Node<'n>) -> Self {
        // Resolved holds the <new name> -> <existing name> pairs for
        // successfully resolved typedef targets.
        let mut resolved: BTreeMap<String, AstType> = BTreeMap::new();

        if let Node::Root(r) = ast {
            for item in r.iter() {
                match item {
                    Node::Typedef(v) => {
                        let alias = v.alias.unwrap_array().as_str();
                        // Try and resolve the original type to a basic type
                        resolved.insert(alias.to_string(), AstType::Typedef(v.clone()));
                    }
                    Node::Struct(v) => {
                        resolved.insert(v.name().to_string(), AstType::Struct(v.clone()));
                    }
                    Node::Union(v) => {
                        resolved.insert(v.name().to_string(), AstType::Union(v.clone()));
                    }
                    Node::Enum(v) => {
                        // Record the names of all compound types
                        resolved.insert(v.name.to_string(), AstType::Enum(v.clone()));
                    }
                    _ => continue,
                };
            }
        }

        TypeIndex(resolved)
    }

    /// Returns the `AstType` for `name`.
    pub fn get<T: AsRef<str>>(&self, name: T) -> Option<&AstType> {
        self.0.get(name.as_ref())
    }

    /// Returns the type aliased by `name` if `name` is a typedef.
    pub fn typedef_target<T: AsRef<str>>(&self, name: T) -> Option<&Typedef> {
        match self.0.get(name.as_ref()) {
            Some(AstType::Typedef(t)) => Some(t),
            _ => None,
        }
    }

    /// Iterates over the types in the type index.
    pub fn iter(&self) -> impl std::iter::Iterator<Item = &AstType> {
        self.0.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{ArrayType, BasicType};

    #[test]
    fn test_typedef_unresolvable() {
        let input = r#"
            typedef old new;
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();
        let index = ast.types();

        assert_eq!(index.get("old"), None);
        assert_eq!(
            index.get("new").unwrap().clone(),
            AstType::Typedef(Typedef {
                target: BasicType::Ident("old".into()),
                alias: ArrayType::None(BasicType::Ident("new".into())),
            })
        );
    }

    #[test]
    fn test_compound_types_in_index() {
        let input = r#"
            struct s {
                u32 a;
            };
            
            union u switch (var_type var_name) {
            case 1:
                    case_type       case_var;
            default:
                    void;
            };

            enum e {
                YES = 1,
                NO = 2
            };
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();

        let mut want = BTreeMap::new();
        want.insert("s".to_string(), ast.types().get("s").unwrap().clone());
        want.insert("u".to_string(), ast.types().get("u").unwrap().clone());
        want.insert("e".to_string(), ast.types().get("e").unwrap().clone());

        let got = ast.types();

        assert_eq!(got.0, want);
    }

    #[test]
    fn test_iter() {
        let input = r#"
            struct s {
                u32 a;
            };
            
            union u switch (var_type var_name) {
            case 1:
                    case_type       case_var;
            default:
                    void;
            };

            enum e {
                YES = 1,
                NO = 2
            };

            typedef uint32_t A;
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();
        let got = ast.types();

        let mut iter = got.iter();

        assert!(matches!(iter.next(), Some(&AstType::Typedef(_))));
        assert!(matches!(iter.next(), Some(&AstType::Enum(_))));
        assert!(matches!(iter.next(), Some(&AstType::Struct(_))));
        assert!(matches!(iter.next(), Some(&AstType::Union(_))));
    }

    #[test]
    fn test_typedef_single_level() {
        let input = r#"
            typedef uint32_t A;
            typedef uint64_t B;
            typedef unsigned int C;
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();

        let mut want = BTreeMap::new();
        want.insert(
            "A".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::U32,
                alias: ArrayType::None(BasicType::Ident("A".into())),
            }),
        );
        want.insert(
            "B".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::U64,
                alias: ArrayType::None(BasicType::Ident("B".into())),
            }),
        );
        want.insert(
            "C".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::U32,
                alias: ArrayType::None(BasicType::Ident("C".into())),
            }),
        );

        let got = ast.types();

        assert_eq!(got.0, want);
    }

    #[test]
    fn test_typedef_chain() {
        let input = r#"
            typedef uint32_t A;
            typedef A B;
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();

        let typedef = AstType::Typedef(Typedef {
            target: BasicType::Ident("A".into()),
            alias: ArrayType::None(BasicType::Ident("B".into())),
        });

        let mut want = BTreeMap::new();
        want.insert(
            "A".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::U32,
                alias: ArrayType::None(BasicType::Ident("A".into())),
            }),
        );
        want.insert("B".to_string(), typedef.clone());

        let got = ast.types();
        assert_eq!(got.0, want);

        assert_eq!(got.get("B"), Some(&typedef));
        assert_eq!(
            got.get("A").unwrap().clone(),
            AstType::Typedef(Typedef {
                target: BasicType::U32,
                alias: ArrayType::None(BasicType::Ident("A".into())),
            })
        );
    }

    #[test]
    fn test_compound_typedef_chain() {
        let input = r#"
            struct thing {
                u32 a;
            };
            typedef thing A;
            typedef A B;
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();

        let got = ast.types();

        let mut want = BTreeMap::new();
        want.insert(
            "A".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::Ident("thing".into()),
                alias: ArrayType::None(BasicType::Ident("A".into())),
            }),
        );
        want.insert(
            "B".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::Ident("A".into()),
                alias: ArrayType::None(BasicType::Ident("B".into())),
            }),
        );
        want.insert(
            "thing".to_string(),
            ast.types().get("thing").unwrap().clone(),
        );

        assert_eq!(got.0, want);
    }

    #[test]
    fn test_compound_typedef_chain_reversed() {
        let input = r#"
            typedef A B;
            typedef thing A;
            struct thing {
                u32 a;
            };
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();
        let got = ast.types();

        let mut want = BTreeMap::new();
        want.insert(
            "B".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::Ident("A".into()),
                alias: ArrayType::None(BasicType::Ident("B".into())),
            }),
        );
        want.insert(
            "A".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::Ident("thing".into()),
                alias: ArrayType::None(BasicType::Ident("A".into())),
            }),
        );
        want.insert(
            "thing".to_string(),
            ast.types().get("thing").unwrap().clone(),
        );

        assert_eq!(got.0, want);
    }

    #[test]
    fn test_typedef_compound_enum() {
        let input = r#"
            typedef A B;
            typedef thing A;
            enum thing {
                A = 1
            };
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();
        let got = ast.types();

        let mut want = BTreeMap::new();
        want.insert(
            "B".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::Ident("A".into()),
                alias: ArrayType::None(BasicType::Ident("B".into())),
            }),
        );
        want.insert(
            "A".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::Ident("thing".into()),
                alias: ArrayType::None(BasicType::Ident("A".into())),
            }),
        );
        want.insert(
            "thing".to_string(),
            ast.types().get("thing").unwrap().clone(),
        );

        assert_eq!(got.0, want);
    }

    #[test]
    fn test_typedef_verifier4() {
        use crate::ast::ArraySize;

        let input = r#"
            const NFS4_VERIFIER_SIZE        = 8;
            typedef opaque  verifier4[NFS4_VERIFIER_SIZE];
        "#;

        let ast = crate::ast::Ast::new(input).unwrap();
        let got = ast.types();

        let mut want = BTreeMap::new();
        want.insert(
            "verifier4".to_string(),
            AstType::Typedef(Typedef {
                target: BasicType::Opaque,
                alias: ArrayType::FixedSize(
                    BasicType::Ident("verifier4".into()),
                    ArraySize::Constant("NFS4_VERIFIER_SIZE".into()),
                ),
            }),
        );

        assert_eq!(got.0, want);
    }
}
