use crate::ast::{indexes::*, Ast, BasicType};
use crate::impls::SafeName;
use crate::Result;

pub fn print_serializers<W: std::fmt::Write>(w: &mut W, ast: &Ast) -> Result<()> {
    for primitive in [
        "u8", "i8", "u16", "i16", "f32", "u32", "i32", "f64", "u64", "i64", "u128", "i128",
    ] {
        writeln!(w,"impl Serialisable for {p} {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_{p}(*self) }}\n}}", p=primitive)?;
    }
    // bool
    writeln!(w,"impl Serialisable for bool {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(*self as u32) }}\n}}")?;
    // serialize variable-length vectors
    writeln!(w,"impl<T> Serialisable for Vec<T> where T: Serialisable {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_u32(self.len() as u32); let n_start = dst.len(); self.iter().for_each(|i| i.serialize(dst)); for _ in 0..(4 - ((dst.len() - n_start) % 4)) {{ dst.put_u8(0) }} }}\n}}")?;
    // serialize fixed-length vectors
    writeln!(w,"impl<T> Serialisable for [T] where T: Serialisable {{\nfn serialize(&self, dst: &mut BytesMut) {{ let n_start = dst.len(); self.iter().for_each(|i| i.serialize(dst)); for _ in 0..(4 - ((dst.len() - n_start) % 4)) {{ dst.put_u8(0) }} }}\n}}")?;
    for item in ast.types().iter() {
        match item {
            AstType::Typedef(v) => {
                // map Typedefs to their target type
                writeln!(w,"impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{ dst.put_{type}(self.0) }}\n}}", v.alias.unwrap_array().as_str(), type=v.target.as_str())?;
            }
            AstType::Enum(v) => {
                writeln!(
                    w,
                    "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                    v.name
                )?;
                // enum serializer body
                writeln!(w, "dst.put_u32(self.clone() as u32);")?;
                writeln!(w, "}}\n}}")?;
            }
            AstType::Union(v) => {
                writeln!(
                    w,
                    "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                    v.name
                )?;
                // enum/union serializer body
                // https://datatracker.ietf.org/doc/html/rfc1014#section-3.14

                writeln!(w, "let n_start = dst.len();")?;
                //serialize discriminant (4 bytes)
                writeln!(w, "match self {{",)?;
                for c in v.cases.iter() {
                    for c_value in c.case_values.iter() {
                        let matcher = ast
                            .constants()
                            .get(c_value.as_str())
                            .map(|c| match *c {
                                ConstantType::ConstValue(ref v) => SafeName(v).to_string(),
                                ConstantType::EnumValue {
                                    ref enum_name,
                                    ref variant,
                                } => format!(
                                    "c if c == {}::{} as {}",
                                    enum_name, variant, v.switch.var_type,
                                ),
                            })
                            .unwrap_or_else(|| SafeName(c_value).to_string());

                        writeln!(
                            w,
                            "Self::{}(var) => {{dst.put_{}({} as {}); var.serialize(dst);}},",
                            SafeName(c_value),
                            SafeName(v.switch.var_type.as_str()),
                            matcher,
                            SafeName(v.switch.var_type.as_str())
                        )?;
                    }
                }
                writeln!(w, "_ => dst.put_i32(0)")?;
                writeln!(w, "}}")?;

                writeln!(
                    w,
                    "for _ in 0..(4 - ((dst.len() - n_start) % 4)) {{ dst.put_u8(0) }} \n}}"
                )?;

                writeln!(w, "}}")?;
            }
            AstType::Struct(v) => {
                writeln!(
                    w,
                    "impl Serialisable for {} {{\nfn serialize(&self, dst: &mut BytesMut) {{",
                    v.name
                )?;

                // struct serializer body
                writeln!(w, "let n_start = dst.len();")?;
                for field in v.fields.iter() {
                    match field.field_value.unwrap_array() {
                        BasicType::Ident(f) => {
                            writeln!(w, "self.{}.serialize(dst);", field.field_name)?;
                        }
                        _ => {
                            writeln!(w, "self.{}.serialize(dst);", field.field_name)?;
                        }
                    }
                }
                writeln!(
                    w,
                    "for _ in 0..(4 - ((dst.len() - n_start) % 4)) {{ dst.put_u8(0) }} "
                )?;
                writeln!(w, "}}\n}}")?;
            }
        }
    }
    Ok(())
}
