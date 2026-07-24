use soroban_env_host::{
    I64Val, U64Val, Val,
    xdr::{Limited, Limits, ReadXdr, ScSpecEntry, ScSpecTypeDef},
};
use std::io::Cursor;

pub struct Contract<'a> {
    pub wasm: &'a [u8],
    pub interface: Interface,
}

pub struct FnSpec {
    pub name: String,
    pub inputs: Vec<ScSpecTypeDef>,
    pub output: Option<ScSpecTypeDef>,
}

pub struct Interface {
    pub funcs: Vec<FnSpec>,
}

/// Read `contractspecv0` and XDR-decode its function entries.
pub fn parse(wasm: &[u8]) -> Interface {
    let Some(bytes) = super::custom_section(wasm, "contractspecv0") else {
        return Interface { funcs: Vec::new() };
    };

    let mut reader = Limited::new(Cursor::new(bytes), Limits::none());
    let mut funcs = Vec::new();
    for entry in ScSpecEntry::read_xdr_iter(&mut reader) {
        if let Ok(ScSpecEntry::FunctionV0(f)) = entry {
            funcs.push(FnSpec {
                name: f.name.0.to_utf8_string_lossy(),
                inputs: f.inputs.iter().map(|i| i.type_.clone()).collect(),
                output: f.outputs.first().cloned(),
            });
        }
    }

    Interface { funcs }
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

/// Synthesize a concrete argument `Val` for each input type. `None` if any input is
/// a type we don't build yet, so the caller can skip that function cleanly.
pub fn synth_args(inputs: &[ScSpecTypeDef]) -> Option<Vec<Val>> {
    inputs.iter().map(arg_val).collect()
}

fn arg_val(t: &ScSpecTypeDef) -> Option<Val> {
    use ScSpecTypeDef as St;
    Some(match t {
        St::Bool => Val::from_bool(true).to_val(),
        St::U32 => Val::from_u32(1).to_val(),
        St::I32 => Val::from_i32(1).to_val(),
        St::U64 => U64Val::from_u32(1).to_val(),
        St::I64 => I64Val::from_i32(1).to_val(),
        _ => return None,
    })
}

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
