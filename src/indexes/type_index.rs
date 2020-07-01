use crate::ast::{BasicType, Enum, Node, Struct, Union};
use std::collections::HashMap;

pub struct TypeIndex<'a>(HashMap<&'a str, ConcreteType<'a>>);

#[derive(Debug, Clone, PartialEq)]
pub enum ConcreteType<'a> {
    Struct(&'a Struct<'a>),
    Union(&'a Union<'a>),
    Enum(&'a Enum),
    Basic(BasicType<'a>),
}

impl<'a> TypeIndex<'a> {
    pub fn new(ast: &'a Node) -> Self {
        // Resolved holds the <new name> -> <existing name> pairs for sucessfuly
        // resolved typedef targets.
        let mut resolved: HashMap<&'a str, ConcreteType> = HashMap::new();

        // Not all targets can be resolved in the first pass - some are
        // typedefs-of-typedefs, and some are typedefs to compound (struct) types.
        let mut unresolved: HashMap<&'a str, &'a str> = HashMap::new();

        if let Node::Root(r) = ast {
            for item in r.iter() {
                match item {
                    Node::Typedef(v) => {
                        let new_type = v[1].ident_str();
                        // Try and resolve the original type to a basic type
                        match &v[0] {
                            Node::Type(BasicType::Ident(c)) => {
                                // Idents need resolving later
                                unresolved.insert(new_type, c.as_ref());
                            }
                            Node::Type(ref t) => {
                                resolved.insert(new_type, ConcreteType::Basic(t.to_owned()));
                            }
                            _ => {
                                // Otherwise retry resolving it later.
                                unresolved.insert(new_type, v[0].ident_str());
                            }
                        }
                    }
                    Node::Struct(v) => {
                        resolved.insert(v.name(), ConcreteType::Struct(v));
                    }
                    Node::Union(v) => {
                        resolved.insert(v.name(), ConcreteType::Union(v));
                    }
                    Node::Enum(v) => {
                        // Record the names of all compound types
                        resolved.insert(&v.name, ConcreteType::Enum(v));
                    }
                    _ => continue,
                };
            }
        }

        // Keep resolving types until there is no difference, flattening typedef
        // chains and resolving aliases of compound types.
        let mut last_len = 0;
        while unresolved.len() > 0 {
            unresolved.retain(|new, old| {
                let got = {
                    if let Some(res) = resolved.get(old) {
                        Some(res.clone())
                    } else {
                        None
                    }
                };

                if let Some(got) = got {
                    resolved.insert(new, got);
                    return false;
                }

                true
            });

            if last_len == unresolved.len() && unresolved.len() != 0 {
                panic!("unable to resolve all typedefs")
            }
            last_len = unresolved.len();
        }

        TypeIndex(resolved)
    }

    /// Returns the concrete type for `name`, chasing any typedef chains to the
    /// terminating type.
    pub fn get_concrete<T: AsRef<str>>(&self, name: T) -> Option<&ConcreteType> {
        self.0.get(name.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;

    #[test]
    #[should_panic(expected = "unable to resolve all typedefs")]
    fn test_typedef_unresolveable() {
        let input = r#"
            typedef old new;
        "#;

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        TypeIndex::new(&ast);
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
        want.insert("s", ConcreteType::Struct(&r[0].unwrap_struct()));
        want.insert("u", ConcreteType::Union(&r[1].unwrap_union()));
        want.insert("e", ConcreteType::Enum(&r[2].unwrap_enum()));

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
        want.insert("A", ConcreteType::Basic(BasicType::U32));
        want.insert("B", ConcreteType::Basic(BasicType::U64));
        want.insert("C", ConcreteType::Basic(BasicType::U32));

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

        let mut want = HashMap::new();
        want.insert("A", ConcreteType::Basic(BasicType::U32));
        want.insert("B", ConcreteType::Basic(BasicType::U32));

        let got = TypeIndex::new(&ast);
        assert_eq!(got.0, want);
    }

    #[test]
    fn test_compound_typedef_chain() {
        let input = r#"
            struct mything {
                u32 a;
            };
            typedef mything A;
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
        want.insert("A", ConcreteType::Struct(&r[0].unwrap_struct()));
        want.insert("B", ConcreteType::Struct(&r[0].unwrap_struct()));
        want.insert("mything", ConcreteType::Struct(&r[0].unwrap_struct()));

        assert_eq!(got.0, want);
    }

    #[test]
    fn test_compound_typedef_chain_reversed() {
        let input = r#"
            typedef A B;
            typedef mything A;
            struct mything {
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
        want.insert("B", ConcreteType::Struct(&r[2].unwrap_struct()));
        want.insert("A", ConcreteType::Struct(&r[2].unwrap_struct()));
        want.insert("mything", ConcreteType::Struct(&r[2].unwrap_struct()));

        assert_eq!(got.0, want);
    }

    #[test]
    fn test_typedef_compound_enum() {
        let input = r#"
            typedef A B;
            typedef mything A;
            enum mything {
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
        want.insert("B", ConcreteType::Enum(&r[2].unwrap_enum()));
        want.insert("A", ConcreteType::Enum(&r[2].unwrap_enum()));
        want.insert("mything", ConcreteType::Enum(&r[2].unwrap_enum()));

        assert_eq!(got.0, want);
    }
}
