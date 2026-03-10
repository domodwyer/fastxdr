use crate::ast::{indexes::*, Ast};
use crate::impls::SafeName;
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
                        writeln!(w, "dst.put_slice(self.0.as_ref());")?;
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
                            writeln!(
                                w,
                                "Self::{}(var) => {{ dst.put_u32(Self::{} as u32); var.serialize(dst); }},",
                                c_value, c_value,
                            )?;
                        }
                    }
                    writeln!(w, "_ => dst.put_i32(0),")?;
                    writeln!(w, "}}")?;
                    writeln!(w, "for _ in 0..pad_length(dst.len() - n_start) {{ dst.put_u8(0) }}")?;
                    Ok(())
                })?;
            }
            AstType::Struct(v) => {
                print_impl(w, &v.name, ast, |w| {
                    writeln!(w, "let n_start = dst.len();")?;
                    for field in v.fields.iter() {
                        writeln!(w, "self.{}.serialize(dst);", SafeName(field.field_name.as_str()))?;
                    }
                    writeln!(w, "for _ in 0..pad_length(dst.len() - n_start) {{ dst.put_u8(0) }}")?;
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

