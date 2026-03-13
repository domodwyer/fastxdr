use crate::ast::{indexes::*, ArrayType, Ast};
use crate::impls::{NonDigitName, SafeName};
use crate::Result;

pub fn print_serializers<W: std::fmt::Write>(w: &mut W, ast: &Ast) -> Result<()> {
    // Basic types
    for p in ["u32", "i32", "f32", "u64", "i64", "f64"] {
        writeln!(w, "impl Serialisable for {p} {{ fn serialize(&self, dst: &mut BytesMut) {{ dst.put_{p}(*self) }} }}")?;
    }
    // bool
    writeln!(w, "impl Serialisable for bool {{ fn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(*self as u32) }} }}")?;
    // String
    writeln!(w, "impl Serialisable for String {{ fn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(self.len() as u32); dst.put_slice(self.as_bytes()); for _ in 0..pad_length(self.len()) {{ dst.put_u8(0) }} }} }}")?;
    // serialize variable-length vectors
    writeln!(w, "impl<T> Serialisable for Vec<T> where T: Serialisable {{ fn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(self.len() as u32); let n_start = dst.len(); self.iter().for_each(|i| i.serialize(dst)); for _ in 0..pad_length(dst.len() - n_start) {{ dst.put_u8(0) }} }} }}")?;
    // serialize fixed-length vectors
    writeln!(w, "impl<T> Serialisable for [T] where T: Serialisable {{ fn serialize(&self, dst: &mut BytesMut) {{ let n_start = dst.len(); self.iter().for_each(|i| i.serialize(dst)); for _ in 0..pad_length(dst.len() - n_start) {{ dst.put_u8(0) }} }} }}")?;
    // serialize Option<T>
    writeln!(w, "impl<T> Serialisable for Option<Box<T>> where T: Serialisable {{ fn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(self.is_some() as u32); if let Some(v) = self {{ v.serialize(dst) }} }} }}")?;
    writeln!(w, "impl Serialisable for Bytes {{ fn serialize(&self, dst: &mut BytesMut) {{ dst.put_slice(self.as_ref()) }} }}")?;

    for item in ast.types().iter() {
        match item {
            AstType::Typedef(v) => {
                let alias = v.alias.unwrap_array().as_str();
                print_impl(w, alias, ast, |w| {
                    if v.target.is_opaque() {
                        if !matches!(v.alias, ArrayType::FixedSize(_, _)) {
                            writeln!(w, "dst.put_u32(self.0.as_ref().len() as u32);")?;
                        }
                        writeln!(w, "dst.put_slice(self.0.as_ref());")?;
                        writeln!(
                            w,
                            "for _ in 0..pad_length(self.0.as_ref().len()) {{ dst.put_u8(0) }}"
                        )?;
                    } else {
                        writeln!(w, "self.0.serialize(dst);")?;
                    }
                    Ok(())
                })?;
            }
            AstType::Enum(v) => {
                print_impl(w, &v.name, ast, |w| {
                    writeln!(w, "dst.put_u32(self.clone() as u32);")?;
                    Ok(())
                })?;
            }
            AstType::Union(v) => {
                print_impl(w, v.name(), ast, |w| {
                    writeln!(w, "let n_start = dst.len();")?;
                    writeln!(w, "match self {{")?;
                    for c in v.cases.iter() {
                        for c_value in c.case_values.iter() {
                            let matcher = get_matcher(ast, c_value);
                            writeln!(
                                w,
                                "Self::{}(var) => {{ dst.put_u32({} as u32); var.serialize(dst); }},",
                                NonDigitName(SafeName(c_value)),
                                matcher,
                            )?;
                        }
                    }

                    for c_value in v.void_cases.iter() {
                        let matcher = get_matcher(ast, c_value);

                        if c_value == "default" {
                            writeln!(w, "Self::default => {{ dst.put_u32(0); }},")?;
                        } else {
                            writeln!(
                                w,
                                "Self::{} => {{ dst.put_u32({} as u32); }},",
                                NonDigitName(SafeName(c_value)),
                                matcher,
                            )?;
                        }
                    }

                    if v.default.is_some() && !v.void_cases.contains(&"default".to_string()) {
                        writeln!(w, "Self::default => {{ dst.put_u32(0); }},")?;
                    }

                    writeln!(w, "_ => dst.put_i32(0),")?;
                    writeln!(w, "}}")?;
                    writeln!(
                        w,
                        "for _ in 0..pad_length(dst.len() - n_start) {{ dst.put_u8(0) }}"
                    )?;
                    Ok(())
                })?;
            }
            AstType::Struct(v) => {
                print_impl(w, &v.name, ast, |w| {
                    writeln!(w, "let n_start = dst.len();")?;
                    for field in v.fields.iter() {
                        writeln!(
                            w,
                            "self.{}.serialize(dst);",
                            SafeName(field.field_name.as_str())
                        )?;
                    }
                    writeln!(
                        w,
                        "for _ in 0..pad_length(dst.len() - n_start) {{ dst.put_u8(0) }}"
                    )?;
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}

fn print_impl<W: std::fmt::Write, F: FnOnce(&mut W) -> Result<()>>(
    w: &mut W,
    name: &str,
    ast: &Ast,
    func: F,
) -> Result<()> {
    if ast.generics().contains(name) {
        writeln!(
            w,
            "impl<T> Serialisable for {}<T> where T: Serialisable + AsRef<[u8]> + Debug {{",
            name
        )?;
    } else {
        writeln!(w, "impl Serialisable for {} {{", name)?;
    }

    writeln!(w, "fn serialize(&self, dst: &mut BytesMut) {{")?;
    func(w)?;
    writeln!(w, "}}\n}}")?;
    Ok(())
}

fn get_matcher(ast: &Ast, name: &str) -> String {
    ast.constants()
        .get(name)
        .map(|c| match *c {
            ConstantType::ConstValue(ref v) => SafeName(v).to_string(),
            ConstantType::EnumValue {
                ref enum_name,
                ref variant,
            } => format!("{}::{}", enum_name, variant,),
        })
        .unwrap_or_else(|| SafeName(name).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Ast;

    fn generate_serializers(xdr: &str) -> String {
        let ast = Ast::new(xdr).unwrap();
        let mut out = String::new();
        print_serializers(&mut out, &ast).unwrap();
        out
    }

    #[test]
    fn test_basic_types() {
        let out = generate_serializers("");
        assert!(out.contains("impl Serialisable for u32 { fn serialize(&self, dst: &mut BytesMut) { dst.put_u32(*self) } }"));
        assert!(out.contains("impl Serialisable for i32 { fn serialize(&self, dst: &mut BytesMut) { dst.put_i32(*self) } }"));
        assert!(out.contains("impl Serialisable for f32 { fn serialize(&self, dst: &mut BytesMut) { dst.put_f32(*self) } }"));
        assert!(out.contains("impl Serialisable for u64 { fn serialize(&self, dst: &mut BytesMut) { dst.put_u64(*self) } }"));
        assert!(out.contains("impl Serialisable for i64 { fn serialize(&self, dst: &mut BytesMut) { dst.put_i64(*self) } }"));
        assert!(out.contains("impl Serialisable for f64 { fn serialize(&self, dst: &mut BytesMut) { dst.put_f64(*self) } }"));
        assert!(out.contains("impl Serialisable for bool { fn serialize(&self, dst: &mut BytesMut) { dst.put_u32(*self as u32) } }"));
    }

    #[test]
    fn test_string_and_vec() {
        let out = generate_serializers("");
        assert!(out.contains("impl Serialisable for String { fn serialize(&self, dst: &mut BytesMut) { dst.put_u32(self.len() as u32); dst.put_slice(self.as_bytes()); for _ in 0..pad_length(self.len()) { dst.put_u8(0) } } }"));
        assert!(out.contains("impl<T> Serialisable for Vec<T> where T: Serialisable { fn serialize(&self, dst: &mut BytesMut) { dst.put_u32(self.len() as u32); let n_start = dst.len(); self.iter().for_each(|i| i.serialize(dst)); for _ in 0..pad_length(dst.len() - n_start) { dst.put_u8(0) } } }"));
    }

    #[test]
    fn test_struct() {
        let xdr = r#"
            struct my_struct {
                u32 a;
                string b<>;
            };
        "#;
        let out = generate_serializers(xdr);
        assert!(out.contains("impl Serialisable for my_struct {"));
        assert!(out.contains("self.a.serialize(dst);"));
        assert!(out.contains("self.b.serialize(dst);"));
        assert!(out.contains("for _ in 0..pad_length(dst.len() - n_start) { dst.put_u8(0) }"));
    }

    #[test]
    fn test_enum() {
        let xdr = r#"
            enum my_enum {
                A = 1,
                B = 2
            };
        "#;
        let out = generate_serializers(xdr);
        assert!(out.contains("impl Serialisable for my_enum {"));
        assert!(out.contains("dst.put_u32(self.clone() as u32);"));
    }

    #[test]
    fn test_union() {
        let xdr = r#"
            enum my_enum {
                A = 1,
                B = 2
            };
            union my_union switch (my_enum type) {
                case A:
                    u32 val_a;
                case B:
                    u64 val_b;
                default:
                    void;
            };
        "#;
        let out = generate_serializers(xdr);
        assert!(out.contains("impl Serialisable for my_union {"));
        assert!(out.contains("Self::A(var) => { dst.put_u32(my_enum::A as u32); var.serialize(dst); },"));
        assert!(out.contains("Self::B(var) => { dst.put_u32(my_enum::B as u32); var.serialize(dst); },"));
        assert!(out.contains("_ => dst.put_i32(0),"));
    }

    #[test]
    fn test_typedef_opaque() {
        let xdr = r#"
            typedef opaque my_opaque<10>;
        "#;
        let out = generate_serializers(xdr);
        // Opaque types are generic in this generator
        assert!(out.contains("impl<T> Serialisable for my_opaque<T> where T: Serialisable + AsRef<[u8]> + Debug {"));
        assert!(out.contains("dst.put_u32(self.0.as_ref().len() as u32);"));
        assert!(out.contains("dst.put_slice(self.0.as_ref());"));
        assert!(out.contains("for _ in 0..pad_length(self.0.as_ref().len()) { dst.put_u8(0) }"));
    }

    #[test]
    fn test_union_with_digit_and_void() {
        let xdr = r#"
            union my_union switch (u32 type) {
                case 1:
                    u32 val_a;
                case 2:
                    void;
            };
        "#;
        let out = generate_serializers(xdr);
        assert!(out.contains("Self::v_1(var) => { dst.put_u32(1 as u32); var.serialize(dst); },"));
        assert!(out.contains("Self::v_2 => { dst.put_u32(2 as u32); },"));
    }

    #[test]
    fn test_typedef_fixed_opaque() {
        let xdr = r#"
            typedef opaque my_fixed[8];
        "#;
        let out: String = generate_serializers(xdr);
        // Opaque types are generic
        assert!(out.contains("impl<T> Serialisable for my_fixed<T> where T: Serialisable + AsRef<[u8]> + Debug {"));
        // Fixed-length opaque does not have a length prefix
        assert!(!out.contains("dst.put_u32(self.0.as_ref().len() as u32);"));
        assert!(out.contains("dst.put_slice(self.0.as_ref());"));
        assert!(out.contains("for _ in 0..pad_length(self.0.as_ref().len()) { dst.put_u8(0) }"));
    }

    #[test]
    fn test_struct_padding() {
        let xdr = r#"
            struct my_struct {
                opaque a[3];
                u32 b;
            };
        "#;
        let out = generate_serializers(xdr);
        assert!(out.contains("impl<T> Serialisable for my_struct<T>"));
        assert!(out.contains("self.a.serialize(dst);"));
        assert!(out.contains("self.b.serialize(dst);"));
        // Struct padding at the end
        assert!(out.contains("for _ in 0..pad_length(dst.len() - n_start) { dst.put_u8(0) }"));
    }

    #[test]
    fn test_union_fallthrough_and_constants() {
        let xdr = r#"
            const VAL = 42;
            union my_union switch (u32 type) {
                case 1:
                case VAL:
                    u32 val_a;
            };
        "#;
        let out = generate_serializers(xdr);
        assert!(out.contains("Self::v_1(var) => { dst.put_u32(1 as u32); var.serialize(dst); },"));
        assert!(out.contains("Self::VAL(var) => { dst.put_u32(42 as u32); var.serialize(dst); },"));
    }
}
