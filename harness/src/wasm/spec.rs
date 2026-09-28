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

pub struct Interface {
    pub funcs: Vec<FnSpec>,
}

/// Read `contractspecv0` and XDR-decode its function entries.
pub fn parse(wasm: &[u8], sections: &[super::Section]) -> Interface {
    let Some(bytes) = super::custom_section(wasm, sections, "contractspecv0") else {
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

/// Whether every input type can be synthesized.
pub fn args_supported(inputs: &[ScSpecTypeDef]) -> bool {
    inputs.iter().all(supported)
}

fn supported(t: &ScSpecTypeDef) -> bool {
    use ScSpecTypeDef as St;
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

/// A concrete argument `Val` per input type.
///
/// Object types (`u128`, `bytes`, `string`, `address`, …) are handles into one host's
/// object table, so they have to be built against the host that will run the call.
/// `self_addr` is the contract's own address, used wherever an `address` is wanted.
pub fn build_args(
    host: &Host,
    self_addr: Val,
    inputs: &[ScSpecTypeDef],
) -> Result<Vec<Val>, HostError> {
    inputs.iter().map(|t| arg_val(host, self_addr, t)).collect()
}

fn arg_val(host: &Host, self_addr: Val, t: &ScSpecTypeDef) -> Result<Val, HostError> {
    use ScSpecTypeDef as St;
    Ok(match t {
        St::Bool => Val::from_bool(true).to_val(),
        St::Void => Val::VOID.to_val(),
        St::U32 => Val::from_u32(1).to_val(),
        St::I32 => Val::from_i32(1).to_val(),
        St::U64 | St::Timepoint | St::Duration => U64Val::from_u32(1).to_val(),
        St::I64 => I64Val::from_i32(1).to_val(),
        St::U128 => host.obj_from_u128_pieces(0, 1)?.to_val(),
        St::I128 => host.obj_from_i128_pieces(0, 1)?.to_val(),
        St::Bytes => host.bytes_new_from_slice(&[1, 2, 3, 4])?.to_val(),
        St::BytesN(n) => host.bytes_new_from_slice(&vec![1u8; n.n as usize])?.to_val(),
        St::String => host.string_new_from_slice(b"antlion")?.to_val(),
        St::Symbol => host.symbol_new_from_slice(b"sym")?.to_val(),
        St::Address => self_addr,
        other => unreachable!("[err] arg_val called on unsupported type {other:?}"),
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
