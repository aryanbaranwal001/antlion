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
pub fn build_args(
    host: &Host,
    self_addr: Val,
    inputs: &[ScSpecTypeDef],
    kind: Inputs,
) -> Result<Vec<Val>, HostError> {
    inputs
        .iter()
        .map(|t| arg_val(host, self_addr, t, kind))
        .collect()
}

fn arg_val(
    host: &Host,
    self_addr: Val,
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
