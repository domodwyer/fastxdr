use super::{NonDigitName, SafeName};
use crate::ast::{ArrayType, BasicType, Node};
use crate::indexes::*;
use crate::Result;

// TODO: set lints + docs

const TRAIT_BOUNDS: &str = "<T> where T: AsRef<[u8]> + Debug";

pub fn print_types<W: std::fmt::Write>(
    w: &mut W,
    item: &Node,
    generic_index: &GenericIndex,
    derive: &str,
) -> Result<()> {
    match item {
        Node::EOF => {}
        Node::Root(v) => {
            for field in v.iter() {
                print_types(w, field, generic_index, derive)?;
            }
        }
        Node::Struct(v) => {
            writeln!(w, "{}", derive)?;
            write!(w, "pub struct {}", v.name)?;
            if generic_index.contains(v.name.as_str()) {
                write!(w, "{}", TRAIT_BOUNDS)?;
            }

            writeln!(w, " {{")?;
            for f in v.fields.iter() {
                write!(w, "pub {}: ", SafeName(&f.field_name))?;

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
                    BasicType::String => write!(w, "String")?,
                    BasicType::Ident(i) if generic_index.contains(i.as_ref()) => {
                        f.field_value
                            .write_with_bounds(w, Some(vec!["T"].as_ref()))?;
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
            writeln!(w, "{}", derive)?;
            write!(w, "pub enum {}", v.name())?;
            if generic_index.contains(v.name()) {
                write!(w, "{}", TRAIT_BOUNDS)?;
            }

            writeln!(w, " {{")?;
            for case in v.cases.iter() {
                // A single case statement may have many case values tied to it
                // if fallthrough values are used:
                //
                // 	case 1:
                // 	case 2:
                // 		// statement
                //
                for c_value in case.case_values.iter() {
                    write!(w, "{}(", NonDigitName(SafeName(&c_value)))?;

                    match case.field_value.unwrap_array() {
                        BasicType::Opaque => write!(w, "T")?,
                        BasicType::String => write!(w, "String")?,
                        BasicType::Ident(i) if generic_index.contains(i.as_ref()) => {
                            write!(w, "{}<T>", i.as_ref())?
                        }
                        _ => write!(w, "{}", case.field_value)?,
                    }

                    writeln!(w, "),")?;
                }
            }

            // There may also be several "void" cases
            for c in v.void_cases.iter() {
                writeln!(w, "{},", NonDigitName(SafeName(c.as_str())))?;
            }

            if v.default.is_some() {
                writeln!(w, "default,")?;
            }

            writeln!(w, "}}")?;
        }
        Node::Enum(v) => {
            writeln!(w, "{}", derive)?;
            writeln!(w, "pub enum {} {{", v.name)?;
            for var in v.variants.iter() {
                writeln!(w, "{} = {},", var.name, var.value)?;
            }
            writeln!(w, "}}")?;
        }
        Node::Constant(v) => {
            writeln!(
                w,
                "pub const {}: u32 = {};",
                v[0].ident_str(),
                v[1].ident_str()
            )?;
        }
        Node::Typedef(v) => {
            // No typedefs to self - this occurs because the ident/type values
            // convert common types directly.
            if v.target == *v.alias.unwrap_array() {
                return Ok(());
            }

            // For typedefs, the array identifier is defined on the alias.
            //
            // Wrap the target in the same array as the alias to generate the
            // array container for the target.
            let target = match &v.alias {
                ArrayType::None(_) => ArrayType::None(&v.target),
                ArrayType::FixedSize(_, s) => ArrayType::FixedSize(&v.target, s.clone()),
                ArrayType::VariableSize(_, s) => ArrayType::VariableSize(&v.target, s.clone()),
            };

            writeln!(w, "{}", derive)?;
            write!(w, "pub struct {}", v.alias.unwrap_array().as_str())?;
            if generic_index.contains(v.target.as_str()) || v.target.is_opaque() {
                write!(
                    w,
                    "<{}>",
                    TRAIT_BOUNDS.split("where").nth(1).unwrap_or("").trim()
                )?;
            }

            // If the target is the opaque type, it should not have array
            // quantifiers - the opaque type has a variable length already.
            if v.target.is_opaque() {
                return Ok(writeln!(w, "(pub T);")?);
            }

            if generic_index.contains(v.target.as_str()) {
                write!(w, " (pub ")?;
                target.write_with_bounds(w, Some(&["T"]))?;
                writeln!(w, ");")?;
            } else {
                writeln!(w, "(pub {});", target)?;
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
    use crate::{walk, Rule, XDRParser};
    use pest::Parser;

    macro_rules! test_convert {
        ($name: ident, $input: expr, $want: expr) => {
            #[test]
            fn $name() {
                let mut ast = XDRParser::parse(Rule::item, $input).unwrap();
                let ast = walk(ast.next().unwrap()).unwrap();
                let generic_index = GenericIndex::new(&ast);

                let mut got = String::new();
                print_types(
                    &mut got,
                    &ast,
                    &generic_index,
                    "#[derive(Debug, PartialEq)]",
                )
                .unwrap();

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
TRUE(open_to_lock_owner4),
FALSE(exist_lock_owner4),
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
NFS4ERR_DENIED(LOCK4denied),
NFS4_OK,
default,
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
NF4LNK(linktext4),
NF4BLK(specdata4),
NF4CHR(specdata4),
NF4SOCK,
NF4FIFO,
NF4DIR,
default,
}
"#
    );

    test_convert!(
        test_struct_nested_to_array_variable_max_union_generic,
        r#"
            union u_type_name switch (unsigned int s) {
                case 1:    opaque some_var;
            };
            struct CB_COMPOUND4res {
                u_type_name   resarray<42>;
            };
        "#,
        r#"#[derive(Debug, PartialEq)]
pub enum u_type_name<T> where T: AsRef<[u8]> + Debug {
v_1(T),
}
#[derive(Debug, PartialEq)]
pub struct CB_COMPOUND4res<T> where T: AsRef<[u8]> + Debug {
pub resarray: Vec<u_type_name<T>>,
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
pub locktype: nfs_lock_type4,
pub reclaim: bool,
pub offset: offset4,
pub length: length4,
pub locker: locker4,
}
"#
    );

    test_convert!(
        test_struct_fixed_array,
        r#"
			struct stateid4 {
					uint32_t        seqid;
					opaque          other[3];
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct stateid4<T> where T: AsRef<[u8]> + Debug {
pub seqid: u32,
pub other: T,
}
"#
    );

    test_convert!(
        test_struct_fixed_array_const,
        r#"
            const SIZE = 3;
			struct stateid4 {
					uint32_t        seqid;
					opaque          other[SIZE];
			};
		"#,
        r#"pub const SIZE: u32 = 3;
#[derive(Debug, PartialEq)]
pub struct stateid4<T> where T: AsRef<[u8]> + Debug {
pub seqid: u32,
pub other: T,
}
"#
    );

    test_convert!(
        test_struct_variable_array_with_max,
        r#"
			struct nfs_client_id4 {
					verifier4       verifier;
					opaque          id<3>;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct nfs_client_id4<T> where T: AsRef<[u8]> + Debug {
pub verifier: verifier4,
pub id: T,
}
"#
    );

    test_convert!(
        test_struct_variable_array_with_max_const,
        r#"
            const SIZE = 3;
			struct nfs_client_id4 {
					verifier4       verifier;
					opaque          id<SIZE>;
			};
		"#,
        r#"pub const SIZE: u32 = 3;
#[derive(Debug, PartialEq)]
pub struct nfs_client_id4<T> where T: AsRef<[u8]> + Debug {
pub verifier: verifier4,
pub id: T,
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
pub eof: bool,
pub data: T,
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
pub r_netid: String,
pub r_addr: String,
}
"#
    );

    test_convert!(
        test_struct_string_max_len,
        r#"
			struct clientaddr4 {
					/* see struct rpcb in RFC 1833 */
					string r_netid<42>;       /* network id */
					string r_addr<24>;        /* universal address */
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct clientaddr4 {
pub r_netid: String,
pub r_addr: String,
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
        r#"pub const ACL4_SUPPORT_ALLOW_ACL: u32 = 0x00000001;
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
        r#"#[derive(Debug, PartialEq)]
pub struct acetype4(pub u32);
#[derive(Debug, PartialEq)]
pub struct utf8string<T: AsRef<[u8]> + Debug>(pub T);
#[derive(Debug, PartialEq)]
pub struct sec_oid4<T: AsRef<[u8]> + Debug>(pub T);
#[derive(Debug, PartialEq)]
pub struct utf8str_cis<T: AsRef<[u8]> + Debug> (pub utf8string<T>);
"#
    );

    test_convert!(
        test_struct_typedef_structs,
        r#"
            typedef uint32_t        acemask4;
            typedef utf8string      utf8str_mixed;
            typedef opaque  utf8string<>;
            struct nfsace4 {
                acemask4                access_mask;
                utf8str_mixed           who;
            };
        "#,
        r#"#[derive(Debug, PartialEq)]
pub struct acemask4(pub u32);
#[derive(Debug, PartialEq)]
pub struct utf8str_mixed<T: AsRef<[u8]> + Debug> (pub utf8string<T>);
#[derive(Debug, PartialEq)]
pub struct utf8string<T: AsRef<[u8]> + Debug>(pub T);
#[derive(Debug, PartialEq)]
pub struct nfsace4<T> where T: AsRef<[u8]> + Debug {
pub access_mask: acemask4,
pub who: utf8str_mixed<T>,
}
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
pub cb_program: u32,
pub cb_location: clientaddr4,
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
pub seqid: u32,
pub other: T,
}
#[derive(Debug, PartialEq)]
pub struct generic_field<T> where T: AsRef<[u8]> + Debug {
pub inner: stateid4<T>,
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
pub other: T,
}
#[derive(Debug, PartialEq)]
pub enum nfs_argop4<T> where T: AsRef<[u8]> + Debug {
OP_GETATTR(stateid4<T>),
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
pub type_v: acetype4,
}
"#
    );

    test_convert!(
        test_reserved_keyword_union_field_name_ignored,
        r#"
			union CB_GETATTR4res switch (unsigned int status) {
			case 1:
				CB_GETATTR4resok       resok4;
			case type:
				SomeType       async;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub enum CB_GETATTR4res {
v_1(CB_GETATTR4resok),
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
OP_GETATTR(GETATTR4args),
OP_LINK(LINK4args),
OP_NVERIFY(NVERIFY4args),
OP_GETFH,
OP_LOOKUPP,
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
pub nextentry: Option<Box<entry4>>,
}
"#
    );

    test_convert!(
        test_typedef_array,
        r#"
            typedef small alias<>;
			struct small {
				uint32_t        id;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct alias(pub Vec<small>);
#[derive(Debug, PartialEq)]
pub struct small {
pub id: u32,
}
"#
    );

    test_convert!(
        test_typedef_array_generic,
        r#"
            typedef small alias<>;
			struct small {
				opaque        id;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct alias<T: AsRef<[u8]> + Debug> (pub Vec<small<T>>);
#[derive(Debug, PartialEq)]
pub struct small<T> where T: AsRef<[u8]> + Debug {
pub id: T,
}
"#
    );

    test_convert!(
        test_typedef_fixed_array_known,
        r#"
            typedef small alias[8];
			struct small {
				uint32_t        id;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct alias(pub [small; 8]);
#[derive(Debug, PartialEq)]
pub struct small {
pub id: u32,
}
"#
    );

    test_convert!(
        test_typedef_fixed_array_known_generic,
        r#"
            typedef small alias[8];
			struct small {
				opaque        id;
			};
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct alias<T: AsRef<[u8]> + Debug> (pub [small<T>; 8]);
#[derive(Debug, PartialEq)]
pub struct small<T> where T: AsRef<[u8]> + Debug {
pub id: T,
}
"#
    );

    test_convert!(
        test_typedef_fixed_array_constant,
        r#"
            const SIZE        = 8;
            typedef small alias[SIZE];
			struct small {
				uint32_t        id;
			};
		"#,
        r#"pub const SIZE: u32 = 8;
#[derive(Debug, PartialEq)]
pub struct alias(pub [small; SIZE as usize]);
#[derive(Debug, PartialEq)]
pub struct small {
pub id: u32,
}
"#
    );

    test_convert!(
        test_typedef_fixed_array_constant_generic,
        r#"
            const SIZE        = 8;
            typedef small alias[SIZE];
			struct small {
				opaque        id;
			};
		"#,
        r#"pub const SIZE: u32 = 8;
#[derive(Debug, PartialEq)]
pub struct alias<T: AsRef<[u8]> + Debug> (pub [small<T>; SIZE as usize]);
#[derive(Debug, PartialEq)]
pub struct small<T> where T: AsRef<[u8]> + Debug {
pub id: T,
}
"#
    );

    test_convert!(
        test_typedef_generic_array_fixed_opaque,
        r#"
            typedef opaque  alias[42];
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct alias<T: AsRef<[u8]> + Debug>(pub T);
"#
    );

    test_convert!(
        test_typedef_generic_array_fixed_constant_opaque,
        r#"
            const SIZE = 42;
            typedef opaque  alias[SIZE];
		"#,
        r#"pub const SIZE: u32 = 42;
#[derive(Debug, PartialEq)]
pub struct alias<T: AsRef<[u8]> + Debug>(pub T);
"#
    );

    test_convert!(
        test_typedef_generic_array_variable_opaque,
        r#"
            typedef opaque  alias<42>;
		"#,
        r#"#[derive(Debug, PartialEq)]
pub struct alias<T: AsRef<[u8]> + Debug>(pub T);
"#
    );

    test_convert!(
        test_typedef_generic_array_variable_constant_opaque,
        r#"
            const SIZE = 42;
            typedef opaque  alias<SIZE>;
		"#,
        r#"pub const SIZE: u32 = 42;
#[derive(Debug, PartialEq)]
pub struct alias<T: AsRef<[u8]> + Debug>(pub T);
"#
    );
}
