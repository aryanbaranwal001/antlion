use serde_json::{Value, json};
use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol,
    testutils::{generate_account_id, generate_bytes_array},
    xdr::{ContractDataDurability, LedgerEntryData, ScAddress, ScSpecTypeDef, ScVal},
};

use crate::wasm::{Contract, spec};

const SIDE: usize = 7;
const CLASS: usize = 11;
const KEY: usize = 22;

/// One stored value: which storage class holds it, its key and its value, rendered short.
#[derive(PartialEq)]
struct Entry {
    class: &'static str,
    key: String,
    value: String,
}

/// What one side stored after the call, or the call's error on one line.
type Side = Result<Vec<Entry>, String>;

/// One function's storage on both sides, or a function skipped for its argument types.
enum Item {
    Unsupported(String),
    Snapshot {
        name: String,
        sig: String,
        sdk: Side,
        solang: Side,
        verdict: &'static str,
    },
}

pub struct Data {
    no_spec: bool,
    items: Vec<Item>,
}

/// Invoke every function once on both builds and record what each one left in storage.
/// `returns` says whether the two agree on the answer; this says whether they agree on
/// what was written, under which key, in which storage class.
///
/// Each function runs on a fresh deploy with `small` inputs and every authorization granted,
/// so the entries shown are what that one call wrote, plus anything the deploy itself
/// wrote.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let items = sdk
        .interface
        .funcs
        .iter()
        .map(|f| {
            if !spec::args_supported(&sdk.interface, &f.inputs) {
                return Item::Unsupported(spec::sig(f));
            }

            let side = |wasm| {
                snapshot(wasm, &sdk.interface, &f.name, &f.inputs)
                    .map_err(|e| super::one_line(&format!("{e:?}")))
            };
            let (s, l) = (side(sdk.wasm), side(solang.wasm));

            let verdict = match (&s, &l) {
                (Ok(a), Ok(b)) if a == b => "same",
                (Ok(_), Ok(_)) => "differs",
                _ => "not compared, a call failed",
            };
            Item::Snapshot {
                name: f.name.clone(),
                sig: spec::sig(f),
                sdk: s,
                solang: l,
                verdict,
            }
        })
        .collect();

    Data {
        no_spec: sdk.interface.funcs.is_empty(),
        items,
    }
}

/// Print what each side left in storage after each function.
pub fn text(name: &str, d: &Data) {
    super::banner("ledger after one call");

    if d.no_spec {
        println!("[skip] no contract spec found for `{name}`, nothing to invoke");
        println!();
        return;
    }

    for item in &d.items {
        let (sig, s, l, verdict) = match item {
            Item::Unsupported(sig) => {
                println!("[skip] {sig}: unsupported arg types");
                continue;
            }
            Item::Snapshot {
                sig,
                sdk,
                solang,
                verdict,
                ..
            } => (sig, sdk, solang, verdict),
        };

        println!("{sig}\n");

        println!(
            "{:<SIDE$} {:<CLASS$} {:<KEY$} value",
            "side", "class", "key"
        );
        print_side("sdk", s);
        print_side("solang", l);

        println!("\nlayout: {verdict}\n");
    }
}

/// A failed call is `{"err": ...}` in place of `entries`.
pub fn json(d: &Data) -> Value {
    let side = |s: &Side| match s {
        Ok(v) => {
            let entries: Vec<Value> = v
                .iter()
                .map(|e| json!({ "class": e.class, "key": e.key, "value": e.value }))
                .collect();
            json!({ "entries": entries })
        }
        Err(e) => json!({ "err": e }),
    };

    let mut functions = Vec::new();
    let mut skipped = Vec::new();
    for item in &d.items {
        match item {
            Item::Unsupported(sig) => skipped.push(json!(sig)),
            Item::Snapshot {
                name,
                sig,
                sdk,
                solang,
                verdict,
            } => functions.push(json!({
                "function": name,
                "signature": sig,
                "sdk": side(sdk),
                "solang": side(solang),
                "layout": verdict,
            })),
        }
    }

    json!({ "no_spec": d.no_spec, "functions": functions, "skipped": skipped })
}

fn print_side(side: &str, entries: &Side) {
    match entries {
        Err(e) => println!("{side:<SIDE$} [err] {e}"),
        Ok(v) if v.is_empty() => println!("{side:<SIDE$} {:<CLASS$} (nothing stored)", "—"),
        Ok(v) => {
            for e in v {
                println!(
                    "{side:<SIDE$} {:<CLASS$} {:<KEY$} {}",
                    e.class, e.key, e.value
                );
            }
        }
    }
}

/// Deploy `wasm`, call `func` once, and return every contract data entry the host holds.
/// Instance storage lives inside the contract instance entry, so its map is unpacked into
/// one entry per key.
fn snapshot(
    wasm: &[u8],
    iface: &spec::Interface,
    func: &str,
    inputs: &[ScSpecTypeDef],
) -> Result<Vec<Entry>, HostError> {
    let host = Host::test_host_with_recording_footprint();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    let contract = host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;
    // Grant any authorization the call asks for, so a function guarded by `require_auth`
    // still reaches its storage writes. Calls that ask for none are unaffected.
    host.switch_to_recording_auth(false)?;

    let args = spec::build_args(&host, contract.to_val(), iface, inputs, spec::Inputs::Small)?;
    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(&args)?;
    host.call(contract, sym, argv)?;

    let mut out = Vec::new();
    for (_, entry) in host.get_stored_entries()? {
        let Some((e, _)) = entry else { continue };
        let LedgerEntryData::ContractData(d) = &e.data else {
            continue;
        };

        match &d.val {
            ScVal::ContractInstance(i) => {
                for m in i.storage.iter().flat_map(|m| m.iter()) {
                    out.push(Entry {
                        class: "instance",
                        key: show(&m.key),
                        value: show(&m.val),
                    });
                }
            }
            v => out.push(Entry {
                class: match d.durability {
                    ContractDataDurability::Persistent => "persistent",
                    ContractDataDurability::Temporary => "temporary",
                },
                key: show(&d.key),
                value: show(v),
            }),
        }
    }
    Ok(out)
}

/// An `ScVal` on one line: numbers with their type as a suffix (`1u32`), symbols bare,
/// strings quoted, vecs as `[..]` and maps as `{k: v}`. Anything rarer falls back to its
/// debug form. The suffix matters: `U32(0)` and `U64(0)` are different keys.
fn show(v: &ScVal) -> String {
    let list = |items: Vec<String>| items.join(", ");

    match v {
        ScVal::Bool(b) => b.to_string(),
        ScVal::Void => "void".to_string(),
        ScVal::U32(n) => format!("{n}u32"),
        ScVal::I32(n) => format!("{n}i32"),
        ScVal::U64(n) => format!("{n}u64"),
        ScVal::I64(n) => format!("{n}i64"),
        ScVal::U128(p) => format!("{}u128", ((p.hi as u128) << 64) | p.lo as u128),
        ScVal::I128(p) => format!("{}i128", ((p.hi as i128) << 64) | p.lo as i128),
        ScVal::U256(p) if p.hi_hi == 0 && p.hi_lo == 0 => {
            format!("{}u256", ((p.lo_hi as u128) << 64) | p.lo_lo as u128)
        }
        ScVal::I256(p) if p.hi_hi == 0 && p.hi_lo == 0 => {
            format!("{}i256", ((p.lo_hi as u128) << 64) | p.lo_lo as u128)
        }
        ScVal::Timepoint(t) => format!("timepoint({})", t.0),
        ScVal::Duration(d) => format!("duration({})", d.0),
        ScVal::Symbol(s) if s.0.is_empty() => "<empty symbol>".to_string(),
        ScVal::Symbol(s) => s.0.to_utf8_string_lossy(),
        ScVal::String(s) => format!("{:?}", s.0.to_utf8_string_lossy()),
        ScVal::Error(e) => format!("Error({e:?})"),
        ScVal::Address(ScAddress::Contract(c)) => {
            let hex: String = c.0.0[..4].iter().map(|b| format!("{b:02x}")).collect();
            format!("contract({hex}..)")
        }
        ScVal::Vec(Some(items)) => format!("[{}]", list(items.iter().map(show).collect())),
        ScVal::Vec(None) => "[]".to_string(),
        ScVal::Map(Some(m)) => format!(
            "{{{}}}",
            list(
                m.iter()
                    .map(|e| format!("{}: {}", show(&e.key), show(&e.val)))
                    .collect()
            )
        ),
        ScVal::Map(None) => "{}".to_string(),
        other => format!("{other:?}"),
    }
}
