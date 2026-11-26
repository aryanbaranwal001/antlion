use rayon::prelude::*;
use serde_json::{Value, json};
use soroban_env_host::{
    AddressObject, Env, EnvBase, Host, HostError, Symbol, TryFromVal,
    testutils::{generate_account_id, generate_bytes_array},
    xdr::{ScSpecTypeDef, ScVal},
};

use super::access::entry_name;
use super::call::{Setup, deploy};
use super::ledger::show;
use crate::wasm::{Contract, spec};

const SIDE: usize = 7;
const ENTRY: usize = 30;
const NUM: usize = 10;
const FUNC: usize = 12;
const VALUE: usize = 24;

/// How many times the lifetime run moves the ledger past the next expiry.
const JUMPS: usize = 3;

/// One ledger entry and the last ledger it is live in.
#[derive(Clone, PartialEq)]
struct Live {
    entry: String,
    live_until: u32,
}

/// The rent a call pays for, as the host meters it: the rent is in proportion to these
/// ledger bytes, one byte kept alive for one ledger.
#[derive(Default)]
struct Rent {
    persistent: i64,
    persistent_entries: u32,
    temporary: i64,
    temporary_entries: u32,
}

/// One call on a fresh deploy: its answer, every entry's lifetime before and after, and the
/// rent it pays.
struct Call {
    result: String,
    before: Vec<Live>,
    after: Vec<Live>,
    rent: Rent,
}

type Side = Result<Box<Call>, String>;

enum Item {
    Unsupported(String),
    Called {
        name: String,
        sig: String,
        sdk: Side,
        solang: Side,
    },
}

/// One round of the lifetime run: the ledger it ran in, the entries that had expired by
/// then on each side, and each function's answer on each side.
struct Round {
    ledger: u32,
    expired: (Vec<String>, Vec<String>),
    steps: Vec<(String, String, String)>,
}

/// The rent of a deploy: the persistent ledger bytes of uploading the code and creating the
/// instance, and the part of that the constructor's own writes add.
struct Deploy {
    total: i64,
    constructor: i64,
}

pub struct Data {
    no_spec: bool,
    deploy: (Result<Deploy, String>, Result<Deploy, String>),
    items: Vec<Item>,
    lifetime: Result<Vec<Round>, String>,
}

/// How long each entry lives, how each call changes that, the rent each call pays, and what
/// each side does once its entries expire.
///
/// Each function first runs once on a fresh deploy with `small` arguments and every
/// authorization granted, at ledger 0. The lifetime run then deploys each side once and calls
/// every function in spec order. It moves both ledgers to one past the first entry on either
/// side to expire, and calls every function again, `JUMPS` times. An entry that has expired
/// once is not waited for again, even if a call writes it again.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let (sw, lw, iface) = (sdk.wasm, solang.wasm, &sdk.interface);
    let text = |e: HostError| super::one_line(&format!("{e:?}"));
    let funcs: Vec<&spec::FnSpec> = iface
        .funcs
        .iter()
        .filter(|f| spec::args_supported(iface, &f.inputs))
        .collect();

    let deploy = rayon::join(
        || deploy_rent(sw).map_err(text),
        || deploy_rent(lw).map_err(text),
    );
    let (items, lifetime) = rayon::join(
        || {
            iface
                .funcs
                .par_iter()
                .map(|f| {
                    if !spec::args_supported(iface, &f.inputs) {
                        return Item::Unsupported(spec::sig(f));
                    }
                    let side = |wasm| {
                        once(wasm, iface, &f.name, &f.inputs)
                            .map(Box::new)
                            .map_err(text)
                    };
                    Item::Called {
                        name: f.name.clone(),
                        sig: spec::sig(f),
                        sdk: side(sw),
                        solang: side(lw),
                    }
                })
                .collect()
        },
        || lifetime(sw, lw, iface, &funcs).map_err(text),
    );

    Data {
        no_spec: iface.funcs.is_empty(),
        deploy,
        items,
        lifetime,
    }
}

fn setup() -> Setup {
    Setup {
        cached: false,
        authed: true,
    }
}

/// Every ledger entry the host holds that has a lifetime, with the last ledger it is live in.
fn lives(host: &Host) -> Result<Vec<Live>, HostError> {
    Ok(host
        .get_stored_entries()?
        .into_iter()
        .filter_map(|(k, e)| {
            let (_, live_until) = e?;
            Some(Live {
                entry: entry_name(&k),
                live_until: live_until?,
            })
        })
        .collect())
}

/// Call `func` with `small` arguments, and give its answer or its error code.
fn answer(
    host: &Host,
    contract: AddressObject,
    iface: &spec::Interface,
    func: &str,
    inputs: &[ScSpecTypeDef],
) -> Result<String, HostError> {
    let args = spec::build_args(host, contract.to_val(), iface, inputs, spec::Inputs::Small)?;
    let sym = Symbol::from(host.symbol_new_from_slice(func.as_bytes())?);
    let argv = host.vec_new_from_slice(&args)?;
    Ok(match host.call(contract, sym, argv) {
        Ok(v) => show(&ScVal::try_from_val(host, &v)?),
        Err(e) => format!("[err] {:?}", e.error),
    })
}

/// The rent of deploying `wasm`, as `deploy` does, with the host metering it.
fn deploy_rent(wasm: &[u8]) -> Result<Deploy, HostError> {
    let host = Host::test_host_with_recording_footprint();
    host.enable_invocation_metering();
    let account = generate_account_id(&host);
    let salt = generate_bytes_array(&host);
    host.register_test_contract_wasm_from_source_account(wasm, account, salt)?;
    let r = host.get_detailed_last_invocation_resources();
    Ok(Deploy {
        total: r
            .as_ref()
            .map_or(0, |r| r.resources.persistent_rent_ledger_bytes),
        constructor: r.as_ref().map_or(0, |r| {
            r.sub_call_resources
                .iter()
                .map(|c| c.resources.persistent_rent_ledger_bytes)
                .sum()
        }),
    })
}

/// One call on a fresh deploy at ledger 0, with the host metering what it uses.
fn once(
    wasm: &[u8],
    iface: &spec::Interface,
    func: &str,
    inputs: &[ScSpecTypeDef],
) -> Result<Call, HostError> {
    let (host, contract) = deploy(wasm, setup())?;
    host.enable_invocation_metering();
    let before = lives(&host)?;
    let result = answer(&host, contract, iface, func, inputs)?;
    let after = lives(&host)?;
    let rent = host
        .get_last_invocation_resources()
        .map(|r| Rent {
            persistent: r.persistent_rent_ledger_bytes,
            persistent_entries: r.persistent_entry_rent_bumps,
            temporary: r.temporary_rent_ledger_bytes,
            temporary_entries: r.temporary_entry_rent_bumps,
        })
        .unwrap_or_default();
    Ok(Call {
        result,
        before,
        after,
        rent,
    })
}

/// The lifetime run: every function in order, then the ledger moved past the next expiry,
/// and again.
fn lifetime(
    sw: &[u8],
    lw: &[u8],
    iface: &spec::Interface,
    funcs: &[&spec::FnSpec],
) -> Result<Vec<Round>, HostError> {
    let (s, l) = (deploy(sw, setup())?, deploy(lw, setup())?);
    let mut rounds = Vec::new();
    let mut ledger = 0;
    let mut expired: (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    let mut seen: Vec<String> = Vec::new();

    for jump in 0..=JUMPS {
        let mut steps = Vec::new();
        for f in funcs {
            let run = |(host, contract): &(Host, AddressObject)| {
                // Each call gets a full budget, as each would be its own transaction.
                host.budget_cloned().reset()?;
                answer(host, *contract, iface, &f.name, &f.inputs)
            };
            steps.push((f.name.clone(), run(&s)?, run(&l)?));
        }
        rounds.push(Round {
            ledger,
            expired: expired.clone(),
            steps,
        });
        if jump == JUMPS {
            break;
        }

        // The first entry still live on either side, that has not expired before, is the
        // next to expire.
        let (ls, ll) = (lives(&s.0)?, lives(&l.0)?);
        let next = ls
            .iter()
            .chain(&ll)
            .filter(|e| e.live_until >= ledger && !seen.contains(&e.entry))
            .map(|e| e.live_until)
            .min();
        let Some(until) = next else { break };
        ledger = until + 1;
        let gone = |v: &[Live]| -> Vec<String> {
            v.iter()
                .filter(|e| e.live_until < ledger)
                .map(|e| e.entry.clone())
                .collect()
        };
        expired = (gone(&ls), gone(&ll));
        seen.extend(expired.0.iter().chain(&expired.1).cloned());
        for host in [&s.0, &l.0] {
            host.with_mut_ledger_info(|li| li.sequence_number = ledger)?;
        }
    }
    Ok(rounds)
}

/// The entries a call created or whose lifetime it changed.
fn changed(c: &Call) -> Vec<(String, Option<u32>, u32)> {
    c.after
        .iter()
        .filter_map(|a| {
            let b = c.before.iter().find(|b| b.entry == a.entry);
            match b {
                Some(b) if b.live_until == a.live_until => None,
                _ => Some((a.entry.clone(), b.map(|b| b.live_until), a.live_until)),
            }
        })
        .collect()
}

pub fn text(name: &str, d: &Data) {
    super::banner("entry lifetime and rent");

    if d.no_spec {
        println!("[skip] no contract spec found for `{name}`, nothing to invoke");
        println!();
        return;
    }

    println!("deploy: uploading the code, creating the instance and running the constructor\n");
    for (side, x) in [("sdk", &d.deploy.0), ("solang", &d.deploy.1)] {
        match x {
            Ok(r) => println!(
                "{side:<SIDE$} rent: persistent {} ledger bytes, of which the constructor {}",
                r.total, r.constructor
            ),
            Err(e) => println!("{side:<SIDE$} [err] {e}"),
        }
    }
    println!();

    if super::notes() {
        println!(
            "each call on a fresh deploy at ledger 0, so an entry's last live ledger is also its ttl\n"
        );
    }
    for item in &d.items {
        let (sig, s, l) = match item {
            Item::Unsupported(sig) => {
                println!("[skip] {sig}: unsupported arg types");
                continue;
            }
            Item::Called {
                sig, sdk, solang, ..
            } => (sig, sdk, solang),
        };

        println!("{sig}\n");
        let mut rows = Vec::new();
        for (side, x) in [("sdk", s), ("solang", l)] {
            match x {
                Ok(c) => {
                    for (entry, before, after) in changed(c) {
                        let before = before.map_or_else(|| "new".to_string(), |b| b.to_string());
                        rows.push(format!(
                            "{side:<SIDE$} {entry:<ENTRY$} {before:>NUM$} {after:>NUM$}"
                        ));
                    }
                }
                Err(e) => rows.push(format!("{side:<SIDE$} [err] {e}")),
            }
        }
        // Only the entries the call created or whose lifetime it changed.
        if !rows.is_empty() {
            println!(
                "{:<SIDE$} {:<ENTRY$} {:>NUM$} {:>NUM$}",
                "side", "entry", "live until", "now"
            );
            println!("{}\n", rows.join("\n"));
        }
        for (side, x) in [("sdk", s), ("solang", l)] {
            if let Ok(c) = x {
                println!(
                    "{side:<SIDE$} rent: persistent {} ledger bytes over {} entries, temporary {} over {}; answer {}",
                    c.rent.persistent,
                    c.rent.persistent_entries,
                    c.rent.temporary,
                    c.rent.temporary_entries,
                    c.result
                );
            }
        }
        println!();
    }

    println!(
        "lifetime: one deploy per side, every function in order, then the ledger moved past the next expiry\n"
    );
    match &d.lifetime {
        Err(e) => println!("[err] {e}"),
        Ok(rounds) => {
            for r in rounds {
                println!("ledger {}", r.ledger);
                for (side, gone) in [("sdk", &r.expired.0), ("solang", &r.expired.1)] {
                    if !gone.is_empty() {
                        println!("  expired on {side}: {}", gone.join(", "));
                    }
                }
                for (f, a, b) in &r.steps {
                    let verdict = if a == b {
                        "same"
                    } else if a.starts_with("[err]") && b.starts_with("[err]") {
                        "errors differ"
                    } else {
                        "differ"
                    };
                    println!(
                        "  {f:<FUNC$} {:<VALUE$} {:<VALUE$} {verdict}",
                        cut(a),
                        cut(b)
                    );
                }
                println!();
            }
        }
    }

    if super::notes() {
        println!(
            "note: rent is charged in proportion to its ledger bytes: one byte kept live for one ledger"
        );
        println!();
    }
}

/// `s` cut to fit a column.
fn cut(s: &str) -> String {
    if s.chars().count() > VALUE {
        format!("{}…", s.chars().take(VALUE - 1).collect::<String>())
    } else {
        s.to_string()
    }
}

pub fn json(d: &Data) -> Value {
    let side = |x: &Side| match x {
        Ok(c) => json!({
            "result": c.result,
            "changed": changed(c).into_iter().map(|(entry, before, after)| json!({
                "entry": entry,
                "live_until_before": before,
                "live_until_after": after,
            })).collect::<Vec<_>>(),
            "rent": {
                "persistent_ledger_bytes": c.rent.persistent,
                "persistent_entries": c.rent.persistent_entries,
                "temporary_ledger_bytes": c.rent.temporary,
                "temporary_entries": c.rent.temporary_entries,
            },
        }),
        Err(e) => json!({ "err": e }),
    };

    let mut functions = Vec::new();
    let mut skipped = Vec::new();
    for item in &d.items {
        match item {
            Item::Unsupported(sig) => skipped.push(json!(sig)),
            Item::Called {
                name,
                sig,
                sdk,
                solang,
            } => functions.push(json!({
                "function": name,
                "signature": sig,
                "sdk": side(sdk),
                "solang": side(solang),
            })),
        }
    }

    let lifetime = match &d.lifetime {
        Ok(rounds) => json!(
            rounds
                .iter()
                .map(|r| json!({
                    "ledger": r.ledger,
                    "expired": { "sdk": r.expired.0, "solang": r.expired.1 },
                    "steps": r.steps.iter().map(|(f, a, b)| json!({
                        "function": f, "sdk": a, "solang": b,
                    })).collect::<Vec<_>>(),
                }))
                .collect::<Vec<_>>()
        ),
        Err(e) => json!({ "err": e }),
    };

    let deploy = |x: &Result<Deploy, String>| match x {
        Ok(r) => json!({ "persistent_ledger_bytes": r.total, "constructor": r.constructor }),
        Err(e) => json!({ "err": e }),
    };

    json!({
        "no_spec": d.no_spec,
        "deploy": { "sdk": deploy(&d.deploy.0), "solang": deploy(&d.deploy.1) },
        "functions": functions,
        "skipped": skipped,
        "lifetime": lifetime,
    })
}
