use rayon::prelude::*;
use serde_json::{Value, json};
use soroban_env_host::{
    DiagnosticLevel, Host, HostError, TryFromVal,
    xdr::{ContractEventBody, ContractEventType, ScSpecTypeDef, ScVal},
};

use super::call::{Setup, call, deploy};
use super::ledger::show;
use crate::wasm::{Contract, spec};

const SIDE: usize = 7;
const KIND: usize = 9;
const TOPICS: usize = 30;

/// One event, rendered short.
#[derive(PartialEq)]
struct Event {
    /// `contract` for an event the contract emitted, or the first topic of a diagnostic
    /// one: `log` or `error`.
    kind: String,
    topics: Vec<String>,
    data: String,
    /// Emitted inside a call that failed. The network keeps no contract event from such a
    /// call, only the diagnostic ones.
    failed: bool,
}

/// What one side's call answered, and every event it emitted.
struct Run {
    result: String,
    events: Vec<Event>,
}

/// One side, or why it could not be deployed.
type Side = Result<Run, String>;

enum Item {
    Unsupported(String),
    Call {
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

/// Invoke every function once on both builds and record the events each one emits:
/// those the contract publishes, compared, and the `log` and `error` diagnostics the host
/// records, shown only. A diagnostic is where a failure says why it failed.
///
/// Each function runs on a fresh deploy with `small` inputs and every authorization granted,
/// as in `ledger`. Diagnostics are switched on after the deploy, so only the call's own
/// events are shown.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let (sw, lw, iface) = (sdk.wasm, solang.wasm, &sdk.interface);
    let items = iface
        .funcs
        .par_iter()
        .map(|f| {
            if !spec::args_supported(iface, &f.inputs) {
                return Item::Unsupported(spec::sig(f));
            }

            let side = |wasm| {
                run(wasm, iface, &f.name, &f.inputs).map_err(|e| super::one_line(&format!("{e:?}")))
            };
            let (s, l) = (side(sw), side(lw));

            let verdict = match (&s, &l) {
                (Ok(a), Ok(b)) => {
                    let (a, b) = (contract_events(a), contract_events(b));
                    if a.is_empty() && b.is_empty() {
                        "none emitted"
                    } else if a == b {
                        "same"
                    } else {
                        "differ"
                    }
                }
                _ => "not compared, a deploy failed",
            };
            Item::Call {
                name: f.name.clone(),
                sig: spec::sig(f),
                sdk: s,
                solang: l,
                verdict,
            }
        })
        .collect();

    Data {
        no_spec: iface.funcs.is_empty(),
        items,
    }
}

fn contract_events(r: &Run) -> Vec<&Event> {
    r.events.iter().filter(|e| e.kind == "contract").collect()
}

/// Deploy `wasm`, call `func` once with diagnostics on, and collect its events.
fn run(
    wasm: &[u8],
    iface: &spec::Interface,
    func: &str,
    inputs: &[ScSpecTypeDef],
) -> Result<Run, HostError> {
    let setup = Setup {
        cached: false,
        authed: true,
    };
    let (host, contract) = deploy(wasm, setup)?;
    let before = host.get_events()?.0.len();
    host.set_diagnostic_level(DiagnosticLevel::Debug)?;

    let args = |h: &Host, addr| spec::build_args(h, addr, iface, inputs, spec::Inputs::Small);
    let result = match call(&host, contract, func, args) {
        Ok((v, _)) => show(&ScVal::try_from_val(&host, &v)?),
        Err(e) => format!("[err] {:?}", e.error),
    };

    let mut events = Vec::new();
    for e in host.get_events()?.0.into_iter().skip(before) {
        let ContractEventBody::V0(body) = &e.event.body;
        let kind = match e.event.type_ {
            ContractEventType::Contract => "contract".to_string(),
            ContractEventType::System => "system".to_string(),
            // The host's own `fn_call` and `fn_return` trace every call and are the same on
            // both sides, so only `log` and `error` are kept.
            ContractEventType::Diagnostic => match body.topics.first() {
                Some(ScVal::Symbol(s))
                    if s.0.as_slice() == b"log" || s.0.as_slice() == b"error" =>
                {
                    s.0.to_utf8_string_lossy()
                }
                _ => continue,
            },
        };
        events.push(Event {
            kind,
            topics: body.topics.iter().map(show).collect(),
            data: show(&body.data),
            failed: e.failed_call,
        });
    }
    Ok(Run { result, events })
}

pub fn text(name: &str, d: &Data) {
    super::banner("events");

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
            Item::Call {
                sig,
                sdk,
                solang,
                verdict,
                ..
            } => (sig, sdk, solang, verdict),
        };

        println!("{sig}\n");
        println!(
            "{:<SIDE$} {:<KIND$} {:<TOPICS$} data",
            "side", "kind", "topics"
        );
        print_side("sdk", s);
        print_side("solang", l);
        println!("\nevents: {verdict}\n");
    }

    if super::notes() {
        println!(
            "note: only `contract` events are compared; `log` and `error` are the host's diagnostics"
        );
        println!(
            "note: * marks an event from a call that failed; the network keeps only its diagnostics"
        );
        println!();
    }
}

fn print_side(side: &str, s: &Side) {
    let run = match s {
        Err(e) => {
            println!("{side:<SIDE$} [err] {e}");
            return;
        }
        Ok(r) => r,
    };
    println!("{side:<SIDE$} {:<KIND$} {}", "result", run.result);
    for e in &run.events {
        let mark = if e.failed { " *" } else { "" };
        println!(
            "{side:<SIDE$} {:<KIND$} {:<TOPICS$} {}{mark}",
            e.kind,
            format!("[{}]", e.topics.join(", ")),
            e.data
        );
    }
}

/// A deploy that failed is `{"err": ...}` in place of `result` and `events`.
pub fn json(d: &Data) -> Value {
    let side = |s: &Side| match s {
        Ok(r) => {
            let events: Vec<Value> = r
                .events
                .iter()
                .map(|e| {
                    json!({
                        "kind": e.kind,
                        "topics": e.topics,
                        "data": e.data,
                        "failed_call": e.failed,
                    })
                })
                .collect();
            json!({ "result": r.result, "events": events })
        }
        Err(e) => json!({ "err": e }),
    };

    let mut functions = Vec::new();
    let mut skipped = Vec::new();
    for item in &d.items {
        match item {
            Item::Unsupported(sig) => skipped.push(json!(sig)),
            Item::Call {
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
                "events": verdict,
            })),
        }
    }

    json!({ "no_spec": d.no_spec, "functions": functions, "skipped": skipped })
}
