use soroban_env_host::{
    Env, EnvBase, Host, HostError, I64Val, U64Val, Val,
    xdr::{Limited, Limits, ReadXdr, ScSpecEntry, ScSpecTypeDef},
};
use std::io::Cursor;

pub struct FnSpec {
    pub name: String,
    pub inputs: Vec<ScSpecTypeDef>,
    /// At most one — the XDR bounds `outputs` to length 1.
    pub output: Option<ScSpecTypeDef>,
}

/// A `#[contracttype]` struct: its name and its fields in declaration order.
pub struct StructSpec {
    pub name: String,
    pub fields: Vec<(String, ScSpecTypeDef)>,
}

pub struct Interface {
    pub funcs: Vec<FnSpec>,
    pub structs: Vec<StructSpec>,
}

/// Read `contractspecv0` and XDR-decode its function and struct entries.
pub fn parse(wasm: &[u8], sections: &[super::Section]) -> Interface {
    let Some(bytes) = super::custom_section(wasm, sections, "contractspecv0") else {
        return Interface {
            funcs: Vec::new(),
            structs: Vec::new(),
        };
    };

    let mut reader = Limited::new(Cursor::new(bytes), Limits::none());
    let mut funcs = Vec::new();
    let mut structs = Vec::new();
    for entry in ScSpecEntry::read_xdr_iter(&mut reader) {
        match entry {
            Ok(ScSpecEntry::FunctionV0(f)) => funcs.push(FnSpec {
                name: f.name.0.to_utf8_string_lossy(),
                inputs: f.inputs.iter().map(|i| i.type_.clone()).collect(),
                output: f.outputs.first().cloned(),
            }),
            Ok(ScSpecEntry::UdtStructV0(u)) => structs.push(StructSpec {
                name: u.name.to_utf8_string_lossy(),
                fields: u
                    .fields
                    .iter()
                    .map(|f| (f.name.to_utf8_string_lossy(), f.type_.clone()))
                    .collect(),
            }),
            _ => {}
        }
    }

    Interface { funcs, structs }
}

/// Human-readable signature, e.g. `add(u32, u32) -> u32`.
pub fn sig(f: &FnSpec) -> String {
    let ins = f
        .inputs
        .iter()
        .map(type_name)
        .collect::<Vec<_>>()
        .join(", ");

    match &f.output {
        Some(o) => format!("{}({ins}) -> {}", f.name, type_name(o)),
        None => format!("{}({ins})", f.name),
    }
}

/// Whether every input type can be synthesized.
pub fn args_supported(iface: &Interface, inputs: &[ScSpecTypeDef]) -> bool {
    inputs.iter().all(|t| supported(iface, t))
}

fn supported(iface: &Interface, t: &ScSpecTypeDef) -> bool {
    use ScSpecTypeDef as St;
    if let St::Udt(u) = t {
        return struct_of(iface, &u.name.to_utf8_string_lossy())
            .is_some_and(|s| s.fields.iter().all(|(_, f)| supported(iface, f)));
    }
    if let St::Vec(v) = t {
        return supported(iface, &v.element_type);
    }
    matches!(
        t,
        St::Bool
            | St::Void
            | St::U32
            | St::I32
            | St::U64
            | St::I64
            | St::Timepoint
            | St::Duration
            | St::U128
            | St::I128
            | St::Bytes
            | St::String
            | St::Symbol
            | St::Address
            | St::BytesN(_)
    )
}

/// Which set of argument values to build.
///
/// A `Val` carries its payload in 56 bits. Anything larger cannot ride inline, so the
/// host passes an object handle instead and the contract has to unwrap it. Several of
/// these sets exist to cross that line deliberately.
#[derive(Clone, Copy, PartialEq)]
pub enum Inputs {
    /// Small positive values that ride inline.
    Small,
    /// Too large for the payload, so they arrive as object handles.
    Large,
    /// The largest value that still fits inline, and type extremes.
    Bound,
    /// Negatives, zeros and empties.
    Neg,
}

pub const ALL_INPUTS: [Inputs; 4] = [Inputs::Small, Inputs::Large, Inputs::Bound, Inputs::Neg];

/// Ways to get a struct argument wrong. Every other argument stays at `small`, so any
/// difference comes from the struct alone.
#[derive(Clone, Copy)]
pub enum Malformed {
    /// A field the struct does not declare.
    Extra,
    /// The last declared field left out.
    Missing,
    /// The first declared field given a value of another type.
    WrongType,
    /// The right values in declaration order, as a vec rather than a map.
    AsVec,
}

pub const ALL_MALFORMED: [Malformed; 4] = [
    Malformed::Extra,
    Malformed::Missing,
    Malformed::WrongType,
    Malformed::AsVec,
];

impl Malformed {
    pub fn label(self) -> &'static str {
        match self {
            Malformed::Extra => "extra",
            Malformed::Missing => "short",
            Malformed::WrongType => "wrong",
            Malformed::AsVec => "asvec",
        }
    }
}

/// A value of the wrong type for `t`: `i32(-1)` where anything else is declared, and
/// `u32::MAX` where an `i32` is. All 32 bits set, so a side that reinterprets the word
/// rather than rejecting it answers with a visibly different number.
fn wrong_val(t: &ScSpecTypeDef) -> Val {
    match t {
        ScSpecTypeDef::I32 => Val::from_u32(u32::MAX).to_val(),
        _ => Val::from_i32(-1).to_val(),
    }
}

/// Whether any input is something other than a struct, so a wrongly typed argument can be
/// sent.
pub fn takes_plain(inputs: &[ScSpecTypeDef]) -> bool {
    inputs.iter().any(|t| !matches!(t, ScSpecTypeDef::Udt(_)))
}

/// Arguments at `small`, except the first one that is not a struct, which is sent as the
/// wrong type.
pub fn build_wrong_arg(
    host: &Host,
    self_addr: Val,
    iface: &Interface,
    inputs: &[ScSpecTypeDef],
) -> Result<Vec<Val>, HostError> {
    let mut done = false;
    inputs
        .iter()
        .map(|t| match t {
            ScSpecTypeDef::Udt(_) => arg_val(host, self_addr, iface, t, Inputs::Small),
            _ if !done => {
                done = true;
                Ok(wrong_val(t))
            }
            _ => arg_val(host, self_addr, iface, t, Inputs::Small),
        })
        .collect()
}

/// Whether any input is a struct, so the malformed shapes apply.
pub fn takes_struct(inputs: &[ScSpecTypeDef]) -> bool {
    inputs.iter().any(|t| matches!(t, ScSpecTypeDef::Udt(_)))
}

impl Inputs {
    pub fn label(self) -> &'static str {
        match self {
            Inputs::Small => "small",
            Inputs::Large => "large",
            Inputs::Bound => "bound",
            Inputs::Neg => "neg",
        }
    }
}

/// A concrete argument `Val` per input type.
///
/// Object types (`u128`, `bytes`, `string`, `address`, …) are handles into one host's
/// object table, so they have to be built against the host that will run the call.
/// `self_addr` is the contract's own address, used wherever an `address` is wanted.
/// Structs are looked up in `iface` and built from their fields at the same `kind`.
pub fn build_args(
    host: &Host,
    self_addr: Val,
    iface: &Interface,
    inputs: &[ScSpecTypeDef],
    kind: Inputs,
) -> Result<Vec<Val>, HostError> {
    inputs
        .iter()
        .map(|t| arg_val(host, self_addr, iface, t, kind))
        .collect()
}

/// Arguments at `small`, except the first struct argument, which is built wrong in the way
/// `m` says. Only its top level is wrong; a nested struct inside it stays well formed.
pub fn build_malformed(
    host: &Host,
    self_addr: Val,
    iface: &Interface,
    inputs: &[ScSpecTypeDef],
    m: Malformed,
) -> Result<Vec<Val>, HostError> {
    let mut done = false;
    inputs
        .iter()
        .map(|t| match t {
            ScSpecTypeDef::Udt(u) if !done => {
                done = true;
                let s = struct_of(iface, &u.name.to_utf8_string_lossy())
                    .expect("[err] build_malformed called on an unknown struct");
                malformed_val(host, self_addr, iface, s, m)
            }
            _ => arg_val(host, self_addr, iface, t, Inputs::Small),
        })
        .collect()
}

fn malformed_val(
    host: &Host,
    self_addr: Val,
    iface: &Interface,
    s: &StructSpec,
    m: Malformed,
) -> Result<Val, HostError> {
    let mut fields = s
        .fields
        .iter()
        .map(|(n, t)| {
            Ok((
                n.clone(),
                arg_val(host, self_addr, iface, t, Inputs::Small)?,
            ))
        })
        .collect::<Result<Vec<(String, Val)>, HostError>>()?;

    match m {
        Malformed::AsVec => {
            let vals: Vec<Val> = fields.iter().map(|(_, v)| *v).collect();
            return Ok(host.vec_new_from_slice(&vals)?.to_val());
        }
        Malformed::Extra => fields.push(("zzz".to_string(), Val::from_u32(1).to_val())),
        Malformed::Missing => {
            fields.pop();
        }
        Malformed::WrongType => fields[0].1 = wrong_val(&s.fields[0].1),
    }

    fields.sort_by(|a, b| a.0.cmp(&b.0));
    let keys: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
    let vals: Vec<Val> = fields.iter().map(|(_, v)| *v).collect();
    Ok(host.map_new_from_slices(&keys, &vals)?.to_val())
}

fn struct_of<'a>(iface: &'a Interface, name: &str) -> Option<&'a StructSpec> {
    iface.structs.iter().find(|s| s.name == name)
}

/// A struct in the encoding soroban-sdk uses: a tuple struct (fields named `0`, `1`, ...)
/// is a vec of its fields in order, any other struct is a map from field name symbol to
/// value, with keys sorted.
fn struct_val(
    host: &Host,
    self_addr: Val,
    iface: &Interface,
    s: &StructSpec,
    kind: Inputs,
) -> Result<Val, HostError> {
    let tuple = s
        .fields
        .iter()
        .enumerate()
        .all(|(i, (n, _))| *n == i.to_string());

    if tuple {
        let vals = s
            .fields
            .iter()
            .map(|(_, t)| arg_val(host, self_addr, iface, t, kind))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(host.vec_new_from_slice(&vals)?.to_val());
    }

    let mut fields: Vec<&(String, ScSpecTypeDef)> = s.fields.iter().collect();
    fields.sort_by(|a, b| a.0.cmp(&b.0));
    let keys: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
    let vals = fields
        .iter()
        .map(|(_, t)| arg_val(host, self_addr, iface, t, kind))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(host.map_new_from_slices(&keys, &vals)?.to_val())
}

fn arg_val(
    host: &Host,
    self_addr: Val,
    iface: &Interface,
    t: &ScSpecTypeDef,
    kind: Inputs,
) -> Result<Val, HostError> {
    use Inputs::*;
    use ScSpecTypeDef as St;

    // The inline payload is 56 bits: unsigned fills all of it, signed loses one to the
    // sign. One past each of those is the first value that must become an object.
    const U_FITS: u64 = (1 << 56) - 1;
    const I_FITS: i64 = (1 << 55) - 1;
    const BIG: u64 = 1 << 60;

    Ok(match (t, kind) {
        (St::Udt(u), _) => {
            let s = struct_of(iface, &u.name.to_utf8_string_lossy())
                .expect("[err] arg_val called on an unknown struct");
            struct_val(host, self_addr, iface, s, kind)?
        }

        // A vec is sized by the class too: three elements, twenty past the inline range,
        // one at the boundary, or none.
        (St::Vec(v), _) => {
            let (n, elem) = match kind {
                Small => (3, Small),
                Large => (20, Large),
                Bound => (1, Bound),
                Neg => (0, Small),
            };
            let vals = (0..n)
                .map(|_| arg_val(host, self_addr, iface, &v.element_type, elem))
                .collect::<Result<Vec<_>, _>>()?;
            host.vec_new_from_slice(&vals)?.to_val()
        }

        (St::Void, _) => Val::VOID.to_val(),
        (St::Address, _) => self_addr,

        (St::Bool, Small | Bound) => Val::from_bool(true).to_val(),
        (St::Bool, Large | Neg) => Val::from_bool(false).to_val(),

        (St::U32, Small) => Val::from_u32(1).to_val(),
        (St::U32, Large | Bound) => Val::from_u32(u32::MAX).to_val(),
        (St::U32, Neg) => Val::from_u32(0).to_val(),

        (St::I32, Small) => Val::from_i32(1).to_val(),
        (St::I32, Large) => Val::from_i32(i32::MAX).to_val(),
        (St::I32, Bound) => Val::from_i32(i32::MIN).to_val(),
        (St::I32, Neg) => Val::from_i32(-1).to_val(),

        (St::U64 | St::Timepoint | St::Duration, Small) => U64Val::from_u32(1).to_val(),
        (St::U64 | St::Timepoint | St::Duration, Large) => host.obj_from_u64(BIG)?.to_val(),
        (St::U64 | St::Timepoint | St::Duration, Bound) => host.obj_from_u64(U_FITS)?.to_val(),
        (St::U64 | St::Timepoint | St::Duration, Neg) => host.obj_from_u64(0)?.to_val(),

        (St::I64, Small) => I64Val::from_i32(1).to_val(),
        (St::I64, Large) => host.obj_from_i64(BIG as i64)?.to_val(),
        (St::I64, Bound) => host.obj_from_i64(I_FITS)?.to_val(),
        (St::I64, Neg) => host.obj_from_i64(-(BIG as i64))?.to_val(),

        (St::U128, Small) => host.obj_from_u128_pieces(0, 1)?.to_val(),
        (St::U128, Large) => host.obj_from_u128_pieces(7, 5)?.to_val(),
        (St::U128, Bound) => host.obj_from_u128_pieces(0, u64::MAX)?.to_val(),
        (St::U128, Neg) => host.obj_from_u128_pieces(0, 0)?.to_val(),

        (St::I128, Small) => host.obj_from_i128_pieces(0, 1)?.to_val(),
        (St::I128, Large) => host.obj_from_i128_pieces(7, 5)?.to_val(),
        (St::I128, Bound) => host.obj_from_i128_pieces(0, u64::MAX)?.to_val(),
        (St::I128, Neg) => host.obj_from_i128_pieces(-1, 5)?.to_val(),

        (St::Bytes, Small) => host.bytes_new_from_slice(&[1, 2, 3, 4])?.to_val(),
        (St::Bytes, Large) => host.bytes_new_from_slice(&[0xff; 64])?.to_val(),
        (St::Bytes, Bound) => host.bytes_new_from_slice(&[0xff])?.to_val(),
        (St::Bytes, Neg) => host.bytes_new_from_slice(&[])?.to_val(),

        (St::BytesN(n), Neg) => host.bytes_new_from_slice(&vec![0u8; n.n as usize])?.to_val(),
        (St::BytesN(n), Small) => host.bytes_new_from_slice(&vec![1u8; n.n as usize])?.to_val(),
        (St::BytesN(n), _) => host.bytes_new_from_slice(&vec![0xff; n.n as usize])?.to_val(),

        (St::String, Small) => host.string_new_from_slice(b"antlion")?.to_val(),
        (St::String, Large) => host.string_new_from_slice(&[0x61; 40])?.to_val(),
        (St::String, Bound) => host.string_new_from_slice(b"a")?.to_val(),
        (St::String, Neg) => host.string_new_from_slice(b"")?.to_val(),

        (St::Symbol, Small) => host.symbol_new_from_slice(b"sym")?.to_val(),
        (St::Symbol, Large) => host.symbol_new_from_slice(b"abcdefghijklmnopqrstuvwxyz012345")?.to_val(),
        (St::Symbol, Bound) => host.symbol_new_from_slice(b"a")?.to_val(),
        (St::Symbol, Neg) => host.symbol_new_from_slice(b"")?.to_val(),

        (other, _) => unreachable!("[err] arg_val called on unsupported type {other:?}"),
    })
}

/// A spec type rendered for display.
fn type_name(t: &ScSpecTypeDef) -> String {
    use ScSpecTypeDef::*;

    match t {
        Val => "val",
        Bool => "bool",
        Void => "void",
        Error => "error",
        U32 => "u32",
        I32 => "i32",
        U64 => "u64",
        I64 => "i64",
        Timepoint => "timepoint",
        Duration => "duration",
        U128 => "u128",
        I128 => "i128",
        U256 => "u256",
        I256 => "i256",
        Bytes => "bytes",
        String => "string",
        Symbol => "symbol",
        Address => "address",
        MuxedAddress => "muxed_address",
        Result(r) => {
            return format!(
                "result<{}, {}>",
                type_name(&r.ok_type),
                type_name(&r.error_type)
            );
        }
        Tuple(t) => {
            let inner = t
                .value_types
                .iter()
                .map(type_name)
                .collect::<std::vec::Vec<_>>()
                .join(", ");
            return format!("tuple<{inner}>");
        }
        Option(o) => return format!("option<{}>", type_name(&o.value_type)),
        Vec(v) => return format!("vec<{}>", type_name(&v.element_type)),
        Map(m) => {
            return format!(
                "map<{}, {}>",
                type_name(&m.key_type),
                type_name(&m.value_type)
            );
        }
        BytesN(b) => return format!("bytes{}", b.n),
        Udt(u) => return u.name.to_utf8_string_lossy(),
    }
    .to_string()
}
