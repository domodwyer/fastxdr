use crate::ast::{BasicType, CompoundType, Node};
use std::collections::HashSet;

#[derive(Debug)]
pub struct GenericIndex<'a>(HashSet<&'a str>);

impl<'a> GenericIndex<'a> {
    pub fn contains(&self, ident: &str) -> bool {
        self.0.contains(ident)
    }
}

pub fn build_generic_index<'a>(ast: &'a Node) -> GenericIndex<'a> {
    // Define a recursive ast walker that visits all values in the tree, looking
    // for "opaque" types.
    //
    // Types containing opaque types, and types containing those types (and so
    // on) are added to index to build a full set of type names that require
    // generic bounds.
    fn recurse<'a>(v: &'a Node, index: &mut HashSet<&'a str>) -> bool {
        // Get the type name to see if it is already marked.
        //
        // Only structs, unions and typedefs can contain sub-types that may be
        // generic.
        let name = match v {
            Node::Struct(v) => Some(v.name()),
            Node::Union(v) => Some(v.name()),
            Node::Typedef(v) => Some(v[1].ident_str()),
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
                BasicType::Ident(i) => index.contains(i.as_ref()),
                _ => false,
            },

            // These Nodes can contain inner opaque types, or contain compound
            // types that themselves contain opaques.
            // TODO: common trait for inner_types()
            Node::Struct(v) => v.inner_types().iter().any(|t| match t.unwrap_array() {
                BasicType::Opaque => true,
                BasicType::Ident(i) => index.contains(i.as_ref()),
                _ => false,
            }),
            Node::Union(v) => v.inner_types().iter().any(|t| match t.unwrap_array() {
                BasicType::Opaque => true,
                BasicType::Ident(i) => index.contains(i.as_ref()),
                _ => false,
            }),

            Node::Root(v) | Node::Typedef(v) => v.iter().fold(false, |mut acc, v| {
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
            // TODO: don't have this variants
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
                index.insert(name);
                return true;
            }
        }

        contains_opaque
    };

    let mut index: HashSet<&'a str> = HashSet::new();
    let mut last_size: isize = -1;
    while last_size != index.len() as isize {
        last_size = index.len() as isize;
        recurse(ast, &mut index);
    }

    GenericIndex(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;

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

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();
        let got = build_generic_index(&ast);

        let mut want = HashSet::new();
        want.insert("stateid4");
        want.insert("generic_field");
        want.insert("another");

        let mut got: Vec<&str> = got.0.iter().cloned().collect();
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

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();
        let got = build_generic_index(&ast);

        let mut want = HashSet::new();
        want.insert("stateid4");
        want.insert("generic_field");
        want.insert("another");

        let mut got: Vec<&str> = got.0.iter().cloned().collect();
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

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();
        let got = build_generic_index(&ast);

        let mut want = HashSet::new();
        want.insert("attrlist4");
        want.insert("generic_field");

        want.insert("nfs_fh4");
        want.insert("another");

        let mut got: Vec<&str> = got.0.iter().cloned().collect();
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

        let mut ast = XDRParser::parse(Rule::item, input).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();
        let got = build_generic_index(&ast);

        let mut want = HashSet::new();

        want.insert("READ4resok");
        want.insert("READ4res");

        let mut got: Vec<&str> = got.0.iter().cloned().collect();
        let mut want: Vec<&str> = want.iter().cloned().collect();

        let got = got.as_mut_slice();
        let want = want.as_mut_slice();

        got.sort();
        want.sort();

        assert_eq!(got, want);
    }

    #[test]
    fn test_generic_pushup_nfs() {
        let mut ast = XDRParser::parse(Rule::item, include_str!("../nfs.x")).unwrap();
        let ast = walk(ast.next().unwrap()).unwrap();
        let got = build_generic_index(&ast);

        let mut want = HashSet::new();

        want.insert("READLINK4resok");
        want.insert("SECINFO4resok");
        want.insert("nfs_cb_argop4");
        want.insert("CB_COMPOUND4res");
        want.insert("component4");
        want.insert("createhow4");
        want.insert("OPEN4resok");
        want.insert("stateid4");
        want.insert("secinfo4");
        want.insert("NVERIFY4args");
        want.insert("COMPOUND4args");
        want.insert("COMPOUND4res");
        want.insert("nfs_client_id4");
        want.insert("rpcsec_gss_info");
        want.insert("CB_COMPOUND4args");
        want.insert("RENAME4args");
        want.insert("RELEASE_LOCKOWNER4args");
        want.insert("OPEN_CONFIRM4args");
        want.insert("fattr4_owner_group");
        want.insert("READDIR4res");
        want.insert("VERIFY4args");
        want.insert("fs_location4");
        want.insert("linktext4");
        want.insert("openflag4");
        want.insert("GETFH4res");
        want.insert("READDIR4args");
        want.insert("SETATTR4args");
        want.insert("SETCLIENTID4args");
        want.insert("SETCLIENTID4res");
        want.insert("ascii_REQUIRED4");
        want.insert("nfs_fh4");
        want.insert("SETCLIENTID4resok");
        want.insert("OPEN_CONFIRM4resok");
        want.insert("utf8string");
        want.insert("fattr4_owner");
        want.insert("LOCK4args");
        want.insert("GETFH4resok");
        want.insert("open_to_lock_owner4");
        want.insert("LOCK4resok");
        want.insert("OPEN_DOWNGRADE4res");
        want.insert("OPEN_CONFIRM4res");
        want.insert("locker4");
        want.insert("pathname4");
        want.insert("CLOSE4args");
        want.insert("WRITE4resok");
        want.insert("utf8str_cs");
        want.insert("LOCKT4res");
        want.insert("fattr4_acl");
        want.insert("entry4");
        want.insert("utf8str_mixed");
        want.insert("SETCLIENTID_CONFIRM4args");
        want.insert("nfs_argop4");
        want.insert("nfs_resop4");
        want.insert("CB_GETATTR4res");
        want.insert("nfs_cb_resop4");
        want.insert("READ4res");
        want.insert("open_owner4");
        want.insert("LOCK4res");
        want.insert("OPEN4res");
        want.insert("SECINFO4res");
        want.insert("LOCK4denied");
        want.insert("open_write_delegation4");
        want.insert("verifier4");
        want.insert("COMMIT4res");
        want.insert("READ4resok");
        want.insert("OPEN4args");
        want.insert("GETATTR4res");
        want.insert("OPEN_DOWNGRADE4args");
        want.insert("exist_lock_owner4");
        want.insert("READ4args");
        want.insert("SECINFO4args");
        want.insert("WRITE4res");
        want.insert("WRITE4args");
        want.insert("REMOVE4args");
        want.insert("utf8str_cis");
        want.insert("open_delegation4");
        want.insert("CREATE4args");
        want.insert("LINK4args");
        want.insert("fattr4_mimetype");
        want.insert("CLOSE4res");
        want.insert("DELEGRETURN4args");
        want.insert("CB_GETATTR4resok");
        want.insert("fattr4");
        want.insert("OPEN_DOWNGRADE4resok");
        want.insert("attrlist4");
        want.insert("fs_locations4");
        want.insert("nfsace4");
        want.insert("fattr4_filehandle");
        want.insert("lock_owner4");
        want.insert("PUTFH4args");
        want.insert("GETATTR4resok");
        want.insert("createtype4");
        want.insert("COMMIT4resok");
        want.insert("open_claim4");
        want.insert("dirlist4");
        want.insert("READLINK4res");
        want.insert("LOCKT4args");
        want.insert("open_claim_delegate_cur4");
        want.insert("fattr4_fs_locations");
        want.insert("CB_GETATTR4args");
        want.insert("LOOKUP4args");
        want.insert("LOCKU4args");
        want.insert("CB_RECALL4args");
        want.insert("open_read_delegation4");
        want.insert("READDIR4resok");
        want.insert("LOCKU4res");
        want.insert("sec_oid4");

        let mut got: Vec<&str> = got.0.iter().cloned().collect();
        let mut want: Vec<&str> = want.iter().cloned().collect();

        let got = got.as_mut_slice();
        let want = want.as_mut_slice();

        got.sort();
        want.sort();

        assert_eq!(got, want);
    }
}
