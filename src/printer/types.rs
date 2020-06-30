use super::SafeName;
use crate::ast::{BasicType, Node};
use crate::indexes::GenericIndex;
use crate::{Result, DERIVE, TRAIT_BOUNDS};

pub fn print_types<W: std::fmt::Write>(
    w: &mut W,
    item: &Node,
    generic_index: &GenericIndex,
) -> Result<()> {
    match item {
        Node::EOF => {}
        Node::Root(v) => {
            for field in v.iter() {
                print_types(w, field, generic_index)?;
            }
        }
        Node::Struct(v) => {
            writeln!(w, "{}", DERIVE)?;
            write!(w, "pub struct {}", v.name)?;
            if generic_index.contains(v.name.as_str()) {
                write!(w, "{}", TRAIT_BOUNDS)?;
            }

            writeln!(w, " {{")?;
            for f in v.fields.iter() {
                write!(w, "{}: ", SafeName(&f.field_name))?;

                // Optional fields require boxing to allow a self-referential
                // type chain
                if f.is_optional {
                    write!(w, "Option<Box<")?;
                }

                // For each field, replace any "opaque" types with T, which will
                // be generic for AsRef<[u8]>.
                //
                // For each ident, check if it is in the generic index, and if
                // so, append <T> for the AsRef.
                match f.field_value.unwrap_array() {
                    BasicType::Opaque => write!(w, "T")?,
                    BasicType::Ident(i) if generic_index.contains(i.as_ref()) => {
                        write!(w, "{}<T>", i.as_ref())?
                    }
                    _ => write!(w, "{}", f.field_value)?,
                }

                if f.is_optional {
                    write!(w, ">>")?;
                }

                writeln!(w, ",")?;
            }
            writeln!(w, "}}")?;
        }
        Node::Union(v) => {
            writeln!(w, "{}", DERIVE)?;
            write!(w, "pub enum {}", v.name())?;
            if generic_index.contains(v.name()) {
                write!(w, "{}", TRAIT_BOUNDS)?;
            }

            writeln!(w, " {{")?;
            for case in v.cases.iter().chain(v.default.iter()) {
                write!(w, "{}(", case.field_name)?;

                match case.field_value.unwrap_array() {
                    BasicType::Opaque => write!(w, "T")?,
                    BasicType::Ident(i) if generic_index.contains(i.as_ref()) => {
                        write!(w, "{}<T>", i.as_ref())?
                    }
                    _ => write!(w, "{}", case.field_value)?,
                }

                writeln!(w, "),")?;
            }

            // It may also have one or more cases leading to a "void".
            if v.void_cases.len() > 0 {
                writeln!(w, "Void,")?;
            }

            writeln!(w, "}}")?;
        }
        Node::Enum(v) => {
            writeln!(w, "{}", DERIVE)?;
            writeln!(w, "pub enum {} {{", v.name)?;
            for var in v.variants.iter() {
                writeln!(w, "{} = {},", var.name, var.value)?;
            }
            writeln!(w, "}}")?;
        }
        Node::Constant(v) => {
            writeln!(w, "const {}: u32 = {};", v[0].ident_str(), v[1].ident_str())?;
        }
        Node::Typedef(v) => {
            // No typedefs to self - this occurs because the ident/type values
            // convert common types directly.
            if v[0].ident_str() == v[1].ident_str() {
                return Ok(());
            }

            write!(w, "type {}", v[1].ident_str())?;
            if generic_index.contains(v[0].ident_str()) || v[0].ident_str() == "T" {
                write!(w, "<T>")?;
            }
            if generic_index.contains(v[0].ident_str()) {
                writeln!(w, " = {}<T>;", v[0].ident_str())?;
            } else {
                writeln!(w, " = {};", v[0].ident_str())?;
            }
        }
        Node::Array(_)
        | Node::ArrayVariable(_)
        | Node::ArrayFixed(_)
        | Node::StructDataField(_)
        | Node::UnionDataField(_)
        | Node::UnionDefault(_)
        | Node::UnionVoid
        | Node::EnumVariant(_)
        | Node::Ident(_)
        | Node::Type(_)
        | Node::Option(_)
        | Node::UnionCase(_) => unreachable!(),
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexes::*;
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;

    macro_rules! test_convert {
        ($name: ident, $input: expr, $want: expr) => {
            #[test]
            fn $name() {
                let mut ast = XDRParser::parse(Rule::item, $input).unwrap();
                let ast = walk(ast.next().unwrap()).unwrap();
                let generic_index = build_generic_index(&ast);

                let mut got = String::new();
                print_types(&mut got, &ast, &generic_index).unwrap();

                assert_eq!(got, $want);
            }
        };
    }

    test_convert!(
        test_union,
        r#"
			union locker4 switch (bool new_lock_owner) {
			case TRUE:
					open_to_lock_owner4     open_owner;
			case FALSE:
					exist_lock_owner4       lock_owner;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub enum locker4 {
open_owner(open_to_lock_owner4),
lock_owner(exist_lock_owner4),
}
"#
    );

    test_convert!(
        test_union_with_default,
        r#"
			union LOCKT4res switch (nfsstat4 status) {
				case NFS4ERR_DENIED:
						LOCK4denied    denied;
				case NFS4_OK:
						void;
				default:
						void;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub enum LOCKT4res {
denied(LOCK4denied),
Void,
}
"#
    );

    test_convert!(
        test_union_fallthrough_with_void,
        r#"
			union createtype4 switch (nfs_ftype4 type) {
				case NF4LNK:
						linktext4 linkdata;
				case NF4BLK:
				case NF4CHR:
						specdata4 devdata;
				case NF4SOCK:
				case NF4FIFO:
				case NF4DIR:
						void;
				default:
						void;  /* server should return NFS4ERR_BADTYPE */
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub enum createtype4 {
linkdata(linktext4),
devdata(specdata4),
Void,
}
"#
    );

    test_convert!(
        test_struct,
        r#"
			/*
			* LOCK/LOCKT/LOCKU: Record lock management
			*/
			struct LOCK4args {
					/* CURRENT_FH: file */
					nfs_lock_type4  locktype;
					bool            reclaim;
					offset4         offset;
					length4         length;
					locker4         locker;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct LOCK4args {
locktype: nfs_lock_type4,
reclaim: bool,
offset: offset4,
length: length4,
locker: locker4,
}
"#
    );

    test_convert!(
        test_struct_fixed_array,
        r#"
			struct stateid4 {
					uint32_t        seqid;
					opaque          other[NFS4_OTHER_SIZE];
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct stateid4<T> where T: AsRef<[u8]> + Debug {
seqid: u32,
other: T,
}
"#
    );

    test_convert!(
        test_struct_variable_array_with_max,
        r#"
			struct nfs_client_id4 {
					verifier4       verifier;
					opaque          id<NFS4_OPAQUE_LIMIT>;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct nfs_client_id4<T> where T: AsRef<[u8]> + Debug {
verifier: verifier4,
id: T,
}
"#
    );

    test_convert!(
        test_struct_variable_array_without_max,
        r#"
			struct READ4resok {
					bool            eof;
					opaque          data<>;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct READ4resok<T> where T: AsRef<[u8]> + Debug {
eof: bool,
data: T,
}
"#
    );

    test_convert!(
        test_struct_string,
        r#"
			struct clientaddr4 {
					/* see struct rpcb in RFC 1833 */
					string r_netid<>;       /* network id */
					string r_addr<>;        /* universal address */
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct clientaddr4 {
r_netid: Vec<String>,
r_addr: Vec<String>,
}
"#
    );

    test_convert!(
        test_enum,
        r#"
			enum opentype4 {
					OPEN4_NOCREATE  = 0,
					OPEN4_CREATE    = 1
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub enum opentype4 {
OPEN4_NOCREATE = 0,
OPEN4_CREATE = 1,
}
"#
    );

    test_convert!(
        test_const,
        r#"
			const ACL4_SUPPORT_ALLOW_ACL    = 0x00000001;
		"#,
        r#"const ACL4_SUPPORT_ALLOW_ACL: u32 = 0x00000001;
"#
    );

    test_convert!(
        test_typedef,
        r#"
			typedef uint32_t        acetype4;
			typedef opaque          utf8string<>;
			typedef opaque          sec_oid4;
			typedef utf8string      utf8str_cis;
		"#,
        r#"type acetype4 = u32;
type utf8string<T> = T;
type sec_oid4<T> = T;
type utf8str_cis<T> = utf8string<T>;
"#
    );

    test_convert!(
        test_convert_unsigned_int,
        r#"
			struct cb_client4 {
					unsigned int    cb_program;
					clientaddr4     cb_location;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct cb_client4 {
cb_program: u32,
cb_location: clientaddr4,
}
"#
    );

    test_convert!(
        test_generic_pushup_struct,
        r#"
			struct stateid4 {
				uint32_t        seqid;
				opaque          other[NFS4_OTHER_SIZE];
			};

			struct generic_field {
				stateid4        inner;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct stateid4<T> where T: AsRef<[u8]> + Debug {
seqid: u32,
other: T,
}
#[derive(Debug, PartialEq)]
pub struct generic_field<T> where T: AsRef<[u8]> + Debug {
inner: stateid4<T>,
}
"#
    );

    test_convert!(
        test_generic_pushup_union,
        r#"
			struct stateid4 {
				opaque          other[NFS4_OTHER_SIZE];
			};

			union nfs_argop4 switch (nfs_opnum4 argop) {
				case OP_GETATTR:       stateid4 field_name;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct stateid4<T> where T: AsRef<[u8]> + Debug {
other: T,
}
#[derive(Debug, PartialEq)]
pub enum nfs_argop4<T> where T: AsRef<[u8]> + Debug {
field_name(stateid4<T>),
}
"#
    );

    test_convert!(
        test_reserved_keyword_struct_field_name,
        r#"
			struct nfsace4 {
					acetype4                type;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct nfsace4 {
type_v: acetype4,
}
"#
    );

    test_convert!(
        test_reserved_keyword_union_field_name_ignored,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				CB_GETATTR4resok       resok4;
			case 2:
				SomeType       type;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub enum CB_GETATTR4res {
resok4(CB_GETATTR4resok),
type(SomeType),
}
"#
    );

    test_convert!(
        test_multiple_void_union,
        r#"
			union nfs_argop4 switch (nfs_opnum4 argop) {
				case OP_GETATTR:       GETATTR4args opgetattr;
				case OP_GETFH:         void;
				case OP_LINK:          LINK4args oplink;
				case OP_LOOKUPP:       void;
				case OP_NVERIFY:       NVERIFY4args opnverify;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub enum nfs_argop4 {
opgetattr(GETATTR4args),
oplink(LINK4args),
opnverify(NVERIFY4args),
Void,
}
"#
    );

    test_convert!(
        test_linked_list,
        r#"
			struct entry4 {
					entry4          *nextentry;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct entry4 {
nextentry: Option<Box<entry4>>,
}
"#
    );
}
