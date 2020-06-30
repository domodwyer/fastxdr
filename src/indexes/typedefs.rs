use crate::ast::{BasicType, Node};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TypedefTarget<'a> {
    /// Type holds a basic type, like `u32` or `String`
    Type(BasicType<'a>),

    /// Compound holds the name of a struct/union/enum/etc.
    Compound(&'a Node<'a>),
}

pub fn build_typedef_index<'a>(ast: &'a Node) -> HashMap<&'a str, TypedefTarget<'a>> {
    // Resolved holds the <new name> -> <existing name> pairs for sucessfuly
    // resolved typedef targets.
    let mut resolved: HashMap<&'a str, TypedefTarget> = HashMap::new();

    // Not all targets can be resolved in the first pass - some are
    // typedefs-of-typedefs, and some are typedefs to compound (struct) types.
    let mut unresolved: HashMap<&'a str, &'a str> = HashMap::new();
    let mut compounds: HashMap<&'a str, &Node<'a>> = HashMap::new();

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
                            resolved.insert(new_type, TypedefTarget::Type(t.to_owned()));
                        }
                        _ => {
                            // Otherwise retry resolving it later.
                            unresolved.insert(new_type, v[0].ident_str());
                        }
                    }
                }
                Node::Struct(v) => {
                    compounds.insert(v.name(), item);
                }
                Node::Union(v) => {
                    compounds.insert(v.name(), item);
                }
                Node::Enum(v) => {
                    // Record the names of all compound types
                    compounds.insert(&v.name, item);
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
                } else if let Some(res) = compounds.get(old) {
                    Some(TypedefTarget::Compound(res))
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

    resolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;

    #[test]
    #[should_panic(expected = "unable to resolve all typedefs")]
    fn test_unresolveable() {
        let input = r#"
typedef old new;
        "#;

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        build_typedef_index(&ast);
    }

    #[test]
    fn test_single_level() {
        let input = r#"
typedef uint32_t A;
typedef uint64_t B;
typedef unsigned int C;
        "#;

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();

        let mut want = HashMap::new();
        want.insert("A", TypedefTarget::Type(BasicType::U32));
        want.insert("B", TypedefTarget::Type(BasicType::U64));
        want.insert("C", TypedefTarget::Type(BasicType::U32));

        let got = build_typedef_index(&ast);

        assert_eq!(got, want);
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
        want.insert("A", TypedefTarget::Type(BasicType::U32));
        want.insert("B", TypedefTarget::Type(BasicType::U32));

        let got = build_typedef_index(&ast);
        assert_eq!(got, want);
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

        let got = build_typedef_index(&ast);

        let r = match &ast {
            Node::Root(r) => r,
            _ => panic!("not root"),
        };

        let mut want = HashMap::new();
        want.insert("A", TypedefTarget::Compound(&r[0]));
        want.insert("B", TypedefTarget::Compound(&r[0]));

        assert_eq!(got, want);
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

        let got = build_typedef_index(&ast);

        let r = match &ast {
            Node::Root(r) => r,
            _ => panic!("not root"),
        };

        let mut want = HashMap::new();
        want.insert("B", TypedefTarget::Compound(&r[2]));
        want.insert("A", TypedefTarget::Compound(&r[2]));

        assert_eq!(got, want);
    }
}
