use soroban_env_host::{
    Env, EnvBase, Host, HostError, Symbol,
    testutils::{generate_account_id, generate_bytes_array},
    xdr::{ContractDataDurability, LedgerEntryData, ScSpecTypeDef, ScVal},
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

/// Invoke every function once on both builds and print what each one left in storage.
/// `returns` says whether the two agree on the answer; this says whether they agree on
/// what was written, under which key, in which storage class.
///
/// Each function runs on a fresh deploy with `small` inputs, so the entries shown are what
/// that one call wrote, plus anything the deploy itself wrote.
pub fn report(name: &str, sdk: &Contract, solang: &Contract) {
    super::banner("ledger after one call");

    if sdk.interface.funcs.is_empty() {
        println!("[skip] no contract spec found for `{name}`, nothing to invoke");
        println!();
        return;
    }

    for f in &sdk.interface.funcs {
        if !spec::args_supported(&sdk.interface, &f.inputs) {
            println!("[skip] {}: unsupported arg types", spec::sig(f));
            continue;
        }

        println!("{}\n", spec::sig(f));

        let s = snapshot(sdk.wasm, &sdk.interface, &f.name, &f.inputs);
        let l = snapshot(solang.wasm, &sdk.interface, &f.name, &f.inputs);

        println!(
            "{:<SIDE$} {:<CLASS$} {:<KEY$} value",
            "side", "class", "key"
        );
        print_side("sdk", &s);
        print_side("solang", &l);

        let verdict = match (&s, &l) {
            (Ok(a), Ok(b)) if a == b => "same",
            (Ok(_), Ok(_)) => "differs",
            _ => "not compared, a call failed",
        };
        println!("\nlayout: {verdict}\n");
    }
}

fn print_side(side: &str, entries: &Result<Vec<Entry>, HostError>) {
    match entries {
        Err(e) => {
            let flat = format!("{e:?}")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            println!("{side:<SIDE$} [err] {flat}");
        }
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

/// An `ScVal` on one line: numbers bare, symbols bare, strings quoted, vecs as `[..]` and
/// maps as `{k: v}`. Anything rarer falls back to its debug form.
fn show(v: &ScVal) -> String {
    let list = |items: Vec<String>| items.join(", ");

    match v {
        ScVal::Bool(b) => b.to_string(),
        ScVal::Void => "void".to_string(),
        ScVal::U32(n) => n.to_string(),
        ScVal::I32(n) => n.to_string(),
        ScVal::U64(n) => n.to_string(),
        ScVal::I64(n) => n.to_string(),
        ScVal::U128(p) => (((p.hi as u128) << 64) | p.lo as u128).to_string(),
        ScVal::I128(p) => (((p.hi as i128) << 64) | p.lo as i128).to_string(),
        ScVal::Symbol(s) if s.0.is_empty() => "<empty symbol>".to_string(),
        ScVal::Symbol(s) => s.0.to_utf8_string_lossy(),
        ScVal::String(s) => format!("{:?}", s.0.to_utf8_string_lossy()),
        ScVal::Error(e) => format!("Error({e:?})"),
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
