use crate::ast::{BasicType, Enum, Node, Struct, Typedef, Union};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AstType<'a> {
    Struct(&'a Struct<'a>),
    Union(&'a Union<'a>),
    Enum(&'a Enum),
    Basic(BasicType<'a>),
    Typedef(Typedef<'a>),
}

pub struct TypeIndex<'a>(HashMap<&'a str, AstType<'a>>);

impl<'a> TypeIndex<'a> {
    pub fn new(ast: &'a Node) -> Self {
        // Resolved holds the <new name> -> <existing name> pairs for
        // successfully resolved typedef targets.
        let mut resolved: HashMap<&'a str, AstType> = HashMap::new();

        if let Node::Root(r) = ast {
            for item in r.iter() {
                match item {
                    Node::Typedef(v) => {
                        let alias = v.alias.as_str();
                        // Try and resolve the original type to a basic type
                        resolved.insert(alias, AstType::Typedef(v.clone()));
                    }
                    Node::Struct(v) => {
                        resolved.insert(v.name(), AstType::Struct(v));
                    }
                    Node::Union(v) => {
                        resolved.insert(v.name(), AstType::Union(v));
                    }
                    Node::Enum(v) => {
                        // Record the names of all compound types
                        resolved.insert(&v.name, AstType::Enum(v));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;

    #[test]
    fn test_typedef_unresolvable() {
        let input = r#"
            typedef old new;
        "#;

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        let index = TypeIndex::new(&ast);
        assert_eq!(index.get("old"), None);
        assert_eq!(
            index.get("new").unwrap().clone(),
            AstType::Typedef(Typedef {
                target: BasicType::Ident("old".into()),
                alias: BasicType::Ident("new".into()),
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

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        let r = match &ast {
            Node::Root(r) => r,
            _ => panic!("not root"),
        };

        let mut want = HashMap::new();
        want.insert("s", AstType::Struct(&r[0].unwrap_struct()));
        want.insert("u", AstType::Union(&r[1].unwrap_union()));
        want.insert("e", AstType::Enum(&r[2].unwrap_enum()));

        let got = TypeIndex::new(&ast);

        assert_eq!(got.0, want);
    }

    #[test]
    fn test_typedef_single_level() {
        let input = r#"
            typedef uint32_t A;
            typedef uint64_t B;
            typedef unsigned int C;
        "#;

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        let mut want = HashMap::new();
        want.insert(
            "A",
            AstType::Typedef(Typedef {
                target: BasicType::U32,
                alias: BasicType::Ident("A".into()),
            }),
        );
        want.insert(
            "B",
            AstType::Typedef(Typedef {
                target: BasicType::U64,
                alias: BasicType::Ident("B".into()),
            }),
        );
        want.insert(
            "C",
            AstType::Typedef(Typedef {
                target: BasicType::U32,
                alias: BasicType::Ident("C".into()),
            }),
        );

        let got = TypeIndex::new(&ast);

        assert_eq!(got.0, want);
    }

    #[test]
    fn test_typedef_chain() {
        let input = r#"
            typedef uint32_t A;
            typedef A B;
        "#;

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        let typedef = AstType::Typedef(Typedef {
            target: BasicType::Ident("A".into()),
            alias: BasicType::Ident("B".into()),
        });

        let mut want = HashMap::new();
        want.insert(
            "A",
            AstType::Typedef(Typedef {
                target: BasicType::U32,
                alias: BasicType::Ident("A".into()),
            }),
        );
        want.insert("B", typedef.clone());

        let got = TypeIndex::new(&ast);
        assert_eq!(got.0, want);

        assert_eq!(got.get("B"), Some(&typedef));
        assert_eq!(
            got.get("A").unwrap().clone(),
            AstType::Typedef(Typedef {
                target: BasicType::U32,
                alias: BasicType::Ident("A".into()),
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

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        let got = TypeIndex::new(&ast);

        let r = match &ast {
            Node::Root(r) => r,
            _ => panic!("not root"),
        };

        let mut want = HashMap::new();
        want.insert(
            "A",
            AstType::Typedef(Typedef {
                target: BasicType::Ident("thing".into()),
                alias: BasicType::Ident("A".into()),
            }),
        );
        want.insert(
            "B",
            AstType::Typedef(Typedef {
                target: BasicType::Ident("A".into()),
                alias: BasicType::Ident("B".into()),
            }),
        );
        want.insert("thing", AstType::Struct(&r[0].unwrap_struct()));

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

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        let got = TypeIndex::new(&ast);

        let r = match &ast {
            Node::Root(r) => r,
            _ => panic!("not root"),
        };

        let mut want = HashMap::new();
        want.insert(
            "B",
            AstType::Typedef(Typedef {
                target: BasicType::Ident("A".into()),
                alias: BasicType::Ident("B".into()),
            }),
        );
        want.insert(
            "A",
            AstType::Typedef(Typedef {
                target: BasicType::Ident("thing".into()),
                alias: BasicType::Ident("A".into()),
            }),
        );
        want.insert("thing", AstType::Struct(&r[2].unwrap_struct()));

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

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        let got = TypeIndex::new(&ast);

        let r = match &ast {
            Node::Root(r) => r,
            _ => panic!("not root"),
        };

        let mut want = HashMap::new();
        want.insert(
            "B",
            AstType::Typedef(Typedef {
                target: BasicType::Ident("A".into()),
                alias: BasicType::Ident("B".into()),
            }),
        );
        want.insert(
            "A",
            AstType::Typedef(Typedef {
                target: BasicType::Ident("thing".into()),
                alias: BasicType::Ident("A".into()),
            }),
        );
        want.insert("thing", AstType::Enum(&r[2].unwrap_enum()));

        assert_eq!(got.0, want);
    }
}
