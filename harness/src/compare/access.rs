use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use rayon::prelude::*;
use serde_json::{Value, json};
use soroban_env_host::{
    HostError, TraceEvent, TraceHook, TryFromVal, Val,
    budget::Budget,
    e2e_invoke::{
        self, RecordingInvocationAuthMode, RecordingInvocationAuthParams, TtlLedgerEntryMeta,
    },
    storage::{EntryWithLiveUntil, SnapshotSource},
    testutils::generate_account_id,
    xdr::{
        ContractDataDurability, HostFunction, InvokeContractArgs, LedgerKey, Limits, ReadXdr,
        ScAddress, ScSpecTypeDef, ScSymbol, ScVal, WriteXdr,
    },
};

use super::call::{Setup, deploy};
use super::ledger::show;
use crate::wasm::{Contract, spec};

const SIDE: usize = 7;
const NUM: usize = 6;

/// The host functions that read or write contract storage, as the trace names them.
const STORAGE_CALLS: [&str; 9] = [
    "get_contract_data",
    "has_contract_data",
    "put_contract_data",
    "del_contract_data",
    "extend_contract_data_ttl",
    "extend_current_contract_instance_and_code_ttl",
    "extend_contract_instance_ttl",
    "extend_contract_instance_and_code_ttl",
    "extend_contract_code_ttl",
];

/// One ledger entry the call wrote, and its size before and after.
struct Write {
    entry: String,
    /// The entry's size for rent, in bytes; 0 when it did not exist.
    before: u32,
    /// `None` when the call deleted it.
    after: Option<u32>,
}

/// One host call the contract made: the host function's name and each argument, as the
/// trace renders it. An object argument is a handle, such as `Vec(obj#3)`.
#[derive(Clone, PartialEq)]
pub(super) struct HostCall {
    pub(super) name: &'static str,
    pub(super) args: Vec<String>,
}

/// What one call touched: the entries in its footprint, the bytes it wrote, and every
/// storage host call it made, by function and storage class.
pub(super) struct Access {
    pub(super) result: String,
    read_only: usize,
    read_write: usize,
    write_bytes: u32,
    writes: Vec<Write>,
    /// `get_contract_data instance` and so on, with how many times each was called.
    calls: BTreeMap<String, usize>,
    /// Every host call the contract made, in order.
    pub(super) trace: Vec<HostCall>,
}

type Side = Result<Box<Access>, String>;

enum Item {
    Unsupported(String),
    Traced {
        name: String,
        sig: String,
        sdk: Side,
        solang: Side,
    },
}

pub struct Data {
    no_spec: bool,
    items: Vec<Item>,
}

/// Invoke every function once on both builds, as the network does, and record which
/// ledger entries the call touched and every storage host call it made.
///
/// Each function runs on a fresh deploy with `small` arguments. The call is first run in
/// recording mode, which finds its footprint and its authorizations, then again in
/// enforcing mode with that footprint and a trace of every host call. An address argument
/// is the transaction's source account, so its authorization needs no signature.
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
                trace(wasm, iface, &f.name, &f.inputs)
                    .map(Box::new)
                    .map_err(|e| super::one_line(&format!("{e:?}")))
            };
            Item::Traced {
                name: f.name.clone(),
                sig: spec::sig(f),
                sdk: side(sw),
                solang: side(lw),
            }
        })
        .collect();

    Data {
        no_spec: iface.funcs.is_empty(),
        items,
    }
}

/// The ledger as it is after the deploy.
struct Snapshot(BTreeMap<LedgerKey, EntryWithLiveUntil>);

impl SnapshotSource for Snapshot {
    fn get(&self, key: &Rc<LedgerKey>) -> Result<Option<EntryWithLiveUntil>, HostError> {
        Ok(self.0.get(key.as_ref()).cloned())
    }
}

/// Run `func` once on a fresh deploy of `wasm` as the network does, recorded first and then
/// enforced, and trace every host call of the enforced run.
pub(super) fn trace(
    wasm: &[u8],
    iface: &spec::Interface,
    func: &str,
    inputs: &[ScSpecTypeDef],
) -> Result<Access, HostError> {
    let setup = Setup {
        cached: false,
        authed: false,
    };
    let (host, contract) = deploy(wasm, setup)?;
    let ledger_info = host.with_ledger_info(|li| Ok(li.clone()))?;
    let source = generate_account_id(&host);

    let ScVal::Address(contract_address) = ScVal::try_from_val(&host, &contract.to_val())? else {
        unreachable!("[err] a contract address is an address");
    };
    let source_val = Val::try_from_val(&host, &ScVal::Address(ScAddress::Account(source.clone())))?;
    let args = spec::build_args(&host, source_val, iface, inputs, spec::Inputs::Small)?
        .iter()
        .map(|v| ScVal::try_from_val(&host, v))
        .collect::<Result<Vec<_>, _>>()?;

    let snapshot = Snapshot(
        host.get_stored_entries()?
            .into_iter()
            .filter_map(|(k, e)| e.map(|e| ((*k).clone(), e)))
            .collect(),
    );

    let host_fn = HostFunction::InvokeContract(InvokeContractArgs {
        contract_address,
        function_name: ScSymbol(func.try_into()?),
        args: args.try_into()?,
    });

    // Recording mode finds the footprint and the authorizations, as a simulation does.
    let snapshot = Rc::new(snapshot);
    let recorded = e2e_invoke::invoke_host_function_in_recording_mode(
        &Budget::default(),
        false,
        &host_fn,
        &source,
        RecordingInvocationAuthMode::Recording(RecordingInvocationAuthParams::new(true, false)),
        ledger_info.clone(),
        snapshot.clone(),
        [0; 32],
        &mut Vec::new(),
    )?;

    // Enforcing mode runs the call as the network would, with a trace of every host call.
    let budget = Budget::default();
    let footprint = &recorded.resources.footprint;
    let keys: Vec<&LedgerKey> = footprint
        .read_only
        .iter()
        .chain(footprint.read_write.iter())
        .collect();
    let mut entries = Vec::new();
    for k in &keys {
        entries.push(match snapshot.0.get(k) {
            Some((e, live_until)) => {
                let xdr = e.to_xdr(Limits::none())?;
                let size = e2e_invoke::entry_size_for_rent(&budget, e, xdr.len() as u32)?;
                let meta = live_until.map(|l| TtlLedgerEntryMeta {
                    live_until_ledger: l,
                    entry_size_for_rent: size,
                });
                (Some(xdr), meta)
            }
            None => (None, None),
        });
    }

    let trace = Rc::new(RefCell::new(Vec::new()));
    let seen = trace.clone();
    let hook: TraceHook = Rc::new(move |_, event| {
        if let TraceEvent::EnvCall(name, args) = event {
            seen.borrow_mut().push(HostCall {
                name,
                args: args.iter().map(|a| format!("{a:?}")).collect(),
            });
        }
        Ok(())
    });

    let auth = recorded
        .auth
        .iter()
        .map(|a| a.to_xdr(Limits::none()))
        .collect::<Result<Vec<_>, _>>()?;
    let out = e2e_invoke::invoke_host_function(
        &budget,
        false,
        host_fn.to_xdr(Limits::none())?,
        recorded.resources.to_xdr(Limits::none())?,
        &recorded.restored_rw_entry_indices,
        source.to_xdr(Limits::none())?,
        auth.into_iter(),
        ledger_info,
        entries.into_iter(),
        vec![0; 32],
        &mut Vec::new(),
        Some(hook),
        None,
    )?;

    let result = match &out.encoded_invoke_result {
        Ok(v) => show(&ScVal::from_xdr(v, Limits::none())?),
        Err(e) => format!("[err] {:?}", e.error),
    };
    let mut writes = Vec::new();
    for c in out.ledger_changes.iter().filter(|c| !c.read_only) {
        let key = LedgerKey::from_xdr(&c.encoded_key, Limits::none())?;
        writes.push(Write {
            entry: entry_name(&key),
            before: c.old_entry_size_bytes_for_rent,
            after: c
                .encoded_new_value
                .as_ref()
                .map(|_| c.new_entry_size_bytes_for_rent),
        });
    }

    let trace = trace.borrow().clone();
    let mut calls = BTreeMap::new();
    for c in trace.iter().filter(|c| STORAGE_CALLS.contains(&c.name)) {
        // The storage class is the call's last argument, where it has one.
        let class = c
            .args
            .last()
            .map(|a| a.to_lowercase())
            .filter(|a| matches!(a.as_str(), "instance" | "persistent" | "temporary"));
        let label = match class {
            Some(class) => format!("{} {class}", c.name),
            None => c.name.to_string(),
        };
        *calls.entry(label).or_insert(0) += 1;
    }
    Ok(Access {
        result,
        read_only: footprint.read_only.len(),
        read_write: footprint.read_write.len(),
        write_bytes: recorded.resources.write_bytes,
        writes,
        calls,
        trace,
    })
}

/// A ledger key, short: the storage class and the key for contract data.
fn entry_name(k: &LedgerKey) -> String {
    match k {
        LedgerKey::ContractData(d) => match &d.key {
            ScVal::LedgerKeyContractInstance => "instance entry".to_string(),
            key => {
                let class = match d.durability {
                    ContractDataDurability::Persistent => "persistent",
                    ContractDataDurability::Temporary => "temporary",
                };
                format!("{class} {}", show(key))
            }
        },
        LedgerKey::ContractCode(_) => "contract code".to_string(),
        LedgerKey::Account(_) => "account".to_string(),
        other => format!("{other:?}"),
    }
}

pub fn text(name: &str, d: &Data) {
    super::banner("storage access");

    if d.no_spec {
        println!("[skip] no contract spec found for `{name}`, nothing to invoke");
        println!();
        return;
    }

    for item in &d.items {
        let (sig, s, l) = match item {
            Item::Unsupported(sig) => {
                println!("[skip] {sig}: unsupported arg types");
                continue;
            }
            Item::Traced {
                sig, sdk, solang, ..
            } => (sig, sdk, solang),
        };

        println!("{sig}\n");
        println!(
            "{:<SIDE$} {:>NUM$} {:>NUM$} {:>NUM$}  result",
            "side", "read", "write", "bytes"
        );
        for (side, x) in [("sdk", s), ("solang", l)] {
            match x {
                Ok(a) => println!(
                    "{side:<SIDE$} {:>NUM$} {:>NUM$} {:>NUM$}  {}",
                    a.read_only, a.read_write, a.write_bytes, a.result
                ),
                Err(e) => println!("{side:<SIDE$} [err] {e}"),
            }
        }
        println!();
        let mut lines = Vec::new();
        for (side, x) in [("sdk", s), ("solang", l)] {
            let Ok(a) = x else { continue };
            for w in &a.writes {
                let after = match w.after {
                    Some(n) => format!("{n} bytes"),
                    None => "deleted".to_string(),
                };
                lines.push(format!(
                    "{side:<SIDE$} wrote  {}: {} -> {after}",
                    w.entry, w.before
                ));
            }
            for (call, n) in &a.calls {
                lines.push(format!("{side:<SIDE$} called {call} x{n}"));
            }
        }
        if !lines.is_empty() {
            println!("{}\n", lines.join("\n"));
        }
    }

    println!(
        "note: read and write count the ledger entries in the call's footprint; bytes is what it writes"
    );
    println!(
        "note: each call ran as the network runs it: recorded first, then enforced with that footprint"
    );
    println!();
}

pub fn json(d: &Data) -> Value {
    let side = |x: &Side| match x {
        Ok(a) => json!({
            "result": a.result,
            "read_only": a.read_only,
            "read_write": a.read_write,
            "write_bytes": a.write_bytes,
            "writes": a.writes.iter().map(|w| json!({
                "entry": w.entry,
                "before": w.before,
                "after": w.after,
            })).collect::<Vec<_>>(),
            "calls": a.calls,
        }),
        Err(e) => json!({ "err": e }),
    };

    let mut functions = Vec::new();
    let mut skipped = Vec::new();
    for item in &d.items {
        match item {
            Item::Unsupported(sig) => skipped.push(json!(sig)),
            Item::Traced {
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

    json!({ "no_spec": d.no_spec, "functions": functions, "skipped": skipped })
}
