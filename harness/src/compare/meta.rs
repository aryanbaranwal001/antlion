use std::io::Cursor;

use serde_json::{Map, Value, json};
use soroban_env_host::xdr::{
    Limited, Limits, ReadXdr, ScEnvMetaEntry, ScMetaEntry, ScSpecEntry, ScSpecUdtUnionCaseV0,
};

use crate::wasm::Contract;

const KEY: usize = 14;

/// The kinds of `contractspecv0` entry, in the order shown.
const KINDS: [&str; 6] = ["function", "struct", "union", "enum", "error enum", "event"];

/// What one build declares about itself, decoded.
struct Side {
    /// From `contractenvmetav0`: the host interface the build was made for, as
    /// `(protocol, pre_release)`. More than one is listed if more than one is declared.
    env: Vec<(u32, u32)>,
    /// From `contractmetav0`, in file order.
    meta: Vec<(String, String)>,
    /// The names in `contractspecv0`, one list per entry kind, in `KINDS` order. A
    /// union, an enum and an error enum carry their cases.
    spec: [Vec<String>; 6],
}

pub struct Data {
    sdk: Result<Side, String>,
    solang: Result<Side, String>,
}

/// Decode the three contract custom sections of each build: the protocol it needs, its
/// key and value metadata, and every entry of its spec by kind. Each section may appear
/// more than once; its parts are read in order, as one stream.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    Data {
        sdk: side(sdk),
        solang: side(solang),
    }
}

fn side(c: &Contract) -> Result<Side, String> {
    let section = |name: &str| -> Vec<u8> {
        c.custom
            .iter()
            .filter(|(n, _)| n == name)
            .flat_map(|(_, d)| d.iter().copied())
            .collect()
    };
    let bytes = section("contractenvmetav0");
    let env = entries::<ScEnvMetaEntry>(&bytes, "contractenvmetav0")?
        .into_iter()
        .map(|ScEnvMetaEntry::ScEnvMetaKindInterfaceVersion(v)| (v.protocol, v.pre_release))
        .collect();

    let bytes = section("contractmetav0");
    let meta = entries::<ScMetaEntry>(&bytes, "contractmetav0")?
        .into_iter()
        .map(|ScMetaEntry::ScMetaV0(m)| {
            (m.key.to_utf8_string_lossy(), m.val.to_utf8_string_lossy())
        })
        .collect();

    let bytes = section("contractspecv0");
    let mut spec: [Vec<String>; 6] = Default::default();
    for e in entries::<ScSpecEntry>(&bytes, "contractspecv0")? {
        let (kind, name) = match e {
            ScSpecEntry::FunctionV0(f) => (0, f.name.0.to_utf8_string_lossy()),
            ScSpecEntry::UdtStructV0(s) => (1, s.name.to_utf8_string_lossy()),
            ScSpecEntry::UdtUnionV0(u) => {
                let cases: Vec<String> = u
                    .cases
                    .iter()
                    .map(|c| match c {
                        ScSpecUdtUnionCaseV0::VoidV0(v) => v.name.to_utf8_string_lossy(),
                        ScSpecUdtUnionCaseV0::TupleV0(t) => t.name.to_utf8_string_lossy(),
                    })
                    .collect();
                (
                    2,
                    format!("{}{{{}}}", u.name.to_utf8_string_lossy(), cases.join(", ")),
                )
            }
            ScSpecEntry::UdtEnumV0(u) => {
                let cases: Vec<String> = u
                    .cases
                    .iter()
                    .map(|c| format!("{} = {}", c.name.to_utf8_string_lossy(), c.value))
                    .collect();
                (
                    3,
                    format!("{}{{{}}}", u.name.to_utf8_string_lossy(), cases.join(", ")),
                )
            }
            ScSpecEntry::UdtErrorEnumV0(u) => {
                let cases: Vec<String> = u
                    .cases
                    .iter()
                    .map(|c| format!("{} = {}", c.name.to_utf8_string_lossy(), c.value))
                    .collect();
                (
                    4,
                    format!("{}{{{}}}", u.name.to_utf8_string_lossy(), cases.join(", ")),
                )
            }
            ScSpecEntry::EventV0(v) => (5, v.name.to_utf8_string_lossy()),
        };
        spec[kind].push(name);
    }

    Ok(Side { env, meta, spec })
}

/// Every XDR entry of type `T` in `bytes`, in order. A damaged entry is an error.
fn entries<T: ReadXdr>(bytes: &[u8], section: &str) -> Result<Vec<T>, String> {
    let mut reader = Limited::new(Cursor::new(bytes), Limits::none());
    T::read_xdr_iter(&mut reader)
        .map(|e| e.map_err(|e| format!("malformed {section}: {e}")))
        .collect()
}

pub fn text(d: &Data) {
    super::banner("contract metadata");

    let (s, l) = match (&d.sdk, &d.solang) {
        (Ok(s), Ok(l)) => (s, l),
        (s, l) => {
            for (side, x) in [("sdk", s), ("solang", l)] {
                if let Err(e) = x {
                    println!("[err] {side}: {e}");
                }
            }
            println!();
            return;
        }
    };
    let dash = || "—".to_string();
    // Versions and hashes run long, so the key and sdk columns fit what they hold.
    let key_w = s
        .meta
        .iter()
        .chain(&l.meta)
        .map(|(k, _)| k.chars().count())
        .fold(KEY, usize::max);
    let col_w = s
        .meta
        .iter()
        .map(|(_, v)| v.chars().count())
        .fold(6, usize::max);
    let row = |k: &str, a: String, b: String| println!("{k:<key_w$} {a:<col_w$} {b}");

    println!("contractenvmetav0: the host protocol each build was made for\n");
    row("item", "sdk".into(), "solang".into());
    let n = s.env.len().max(l.env.len());
    for i in 0..n {
        let get = |e: &[(u32, u32)], f: fn(&(u32, u32)) -> u32| {
            e.get(i).map_or_else(dash, |v| f(v).to_string())
        };
        row("protocol", get(&s.env, |v| v.0), get(&l.env, |v| v.0));
        row("pre_release", get(&s.env, |v| v.1), get(&l.env, |v| v.1));
    }
    if n == 0 {
        println!("[skip] neither build declares one");
    }
    println!();

    println!("contractmetav0: key and value text each build carries\n");
    row("key", "sdk".into(), "solang".into());
    let mut keys: Vec<&str> = s.meta.iter().map(|(k, _)| k.as_str()).collect();
    for (k, _) in &l.meta {
        if !keys.contains(&k.as_str()) {
            keys.push(k);
        }
    }
    for k in &keys {
        let get = |m: &[(String, String)]| {
            m.iter()
                .find(|(x, _)| x == k)
                .map_or_else(dash, |(_, v)| v.clone())
        };
        row(k, get(&s.meta), get(&l.meta));
    }
    if keys.is_empty() {
        println!("[skip] neither build carries any");
    }
    println!();

    println!("contractspecv0: what each build declares, by kind\n");
    println!("{:<KEY$} {:>6} {:>6}", "kind", "sdk", "solang");
    for (i, kind) in KINDS.iter().enumerate() {
        println!("{kind:<KEY$} {:>6} {:>6}", s.spec[i].len(), l.spec[i].len());
    }
    println!();
    // Functions are listed by `interface`; every other kind is named here.
    for (i, kind) in KINDS.iter().enumerate().skip(1) {
        for (side, x) in [("sdk", s), ("solang", l)] {
            for name in &x.spec[i] {
                println!("{side:<7} {kind:<11} {name}");
            }
        }
    }

    // The order of entries is the compiler's, not part of the contract.
    let sorted = |x: &Side| {
        x.spec[1..]
            .iter()
            .map(|v| {
                let mut v = v.clone();
                v.sort();
                v
            })
            .collect::<Vec<_>>()
    };
    let same = sorted(s) == sorted(l);
    if super::notes() {
        println!();
        if same {
            println!("note: both builds declare the same types, errors and events");
        } else {
            println!("note: the builds declare different types, errors or events");
        }
    }
    println!();
}

pub fn json(d: &Data) -> Value {
    let side = |x: &Result<Side, String>| match x {
        Ok(s) => {
            let spec: Map<String, Value> = KINDS
                .iter()
                .zip(&s.spec)
                .map(|(k, v)| (k.to_string(), json!(v)))
                .collect();
            let meta: Map<String, Value> =
                s.meta.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
            json!({
                "env": s.env.iter().map(|(p, r)| json!({ "protocol": p, "pre_release": r })).collect::<Vec<_>>(),
                "meta": meta,
                "spec": spec,
            })
        }
        Err(e) => json!({ "err": e }),
    };
    json!({ "sdk": side(&d.sdk), "solang": side(&d.solang) })
}
