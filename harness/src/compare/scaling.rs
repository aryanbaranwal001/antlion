use rayon::prelude::*;
use serde_json::{Value, json};
use soroban_env_host::{Host, xdr::ScSpecTypeDef};

use super::call::{Cost, Setup, call, deploy};
use crate::wasm::{Contract, spec};

/// The largest size. Every size from 0 to this is called, in order, on one deploy.
const MAX: u32 = 64;

/// The sizes shown. The growth is read from 4, 16 and 64.
const SIZES: [u32; 7] = [1, 2, 4, 8, 16, 32, 64];

const N: usize = 5;
const NUM: usize = 12;

/// One side's cost at each size in `SIZES`, or why the deploy failed.
type Side = Result<Vec<Result<Cost, String>>, String>;

enum Item {
    /// A function with no argument that can carry a size.
    Unsized(String),
    Unsupported(String),
    Swept {
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

/// Call every function at growing sizes on both builds and measure how its cost grows.
/// Size `n` means every integer argument is `n` and every list has `n` elements.
///
/// Each function gets one deploy per side, with every authorization granted, and is called
/// at every size from 0 to `MAX` in order. So a call also sees what the calls before it
/// wrote: at size `n`, a function that writes one key per call finds `n` keys already
/// there. The cost of a size is its call alone, with a full budget.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let (sw, lw, iface) = (sdk.wasm, solang.wasm, &sdk.interface);
    let items = iface
        .funcs
        .par_iter()
        .map(|f| {
            if !spec::args_supported(iface, &f.inputs) {
                return Item::Unsupported(spec::sig(f));
            }
            if !spec::sizable(&f.inputs) {
                return Item::Unsized(spec::sig(f));
            }
            let (s, l) = rayon::join(
                || sweep(sw, iface, &f.name, &f.inputs),
                || sweep(lw, iface, &f.name, &f.inputs),
            );
            Item::Swept {
                name: f.name.clone(),
                sig: spec::sig(f),
                sdk: s,
                solang: l,
            }
        })
        .collect();

    Data {
        no_spec: iface.funcs.is_empty(),
        items,
    }
}

/// Deploy `wasm` once, call `func` at every size from 0 to `MAX`, and keep the sizes in
/// `SIZES`.
fn sweep(wasm: &[u8], iface: &spec::Interface, func: &str, inputs: &[ScSpecTypeDef]) -> Side {
    let text = |e| super::one_line(&format!("{e:?}"));
    let setup = Setup {
        cached: true,
        authed: true,
    };
    let (host, contract) = deploy(wasm, setup).map_err(text)?;

    let mut out = Vec::new();
    for n in 0..=MAX {
        host.budget_cloned().reset().map_err(text)?;
        let args = |h: &Host, addr| spec::build_sized(h, addr, iface, inputs, n);
        let cost = call(&host, contract, func, args).map(|(_, c)| c);
        if SIZES.contains(&n) {
            out.push(cost.map_err(|e| format!("{:?}", e.error)));
        }
    }
    Ok(out)
}

/// How the CPU cost grows with the size, read from sizes 4, 16 and 64: `flat` when it
/// grows by less than 1%, else `n^k`, the power of `n` that fits the two steps.
fn growth(s: &Side) -> String {
    let Ok(costs) = s else {
        return "n/a".to_string();
    };
    let at = |n: u32| {
        let i = SIZES.iter().position(|&x| x == n)?;
        costs[i].as_ref().ok().map(|c| c.cpu as f64)
    };
    let (Some(a), Some(b), Some(c)) = (at(4), at(16), at(64)) else {
        return "n/a, a call failed".to_string();
    };
    if (c - a).abs() < a * 0.01 {
        return "flat".to_string();
    }
    let (d1, d2) = (b - a, c - b);
    if d1 <= 0.0 || d2 <= 0.0 {
        return "irregular".to_string();
    }
    format!("n^{:.1}", (d2 / d1).log(4.0))
}

pub fn text(name: &str, d: &Data) {
    super::banner("cost scaling");

    if d.no_spec {
        println!("[skip] no contract spec found for `{name}`, nothing to invoke");
        println!();
        return;
    }

    println!(
        "one deploy per side, called at every size from 0 to {MAX} in order; size n is every integer argument n and every list n long\n"
    );

    for item in &d.items {
        let (sig, s, l) = match item {
            Item::Unsupported(sig) => {
                println!("[skip] {sig}: unsupported arg types");
                continue;
            }
            Item::Unsized(sig) => {
                println!("[skip] {sig}: no integer or list argument to size");
                continue;
            }
            Item::Swept {
                sig, sdk, solang, ..
            } => (sig, sdk, solang),
        };

        println!("{sig}\n");
        for (side, x) in [("sdk", s), ("solang", l)] {
            if let Err(e) = x {
                println!("[err] {side} deploy failed: {e}");
            }
        }
        if let (Ok(s), Ok(l)) = (s, l) {
            println!(
                "{:<N$} {:>NUM$} {:>NUM$} {:>NUM$} {:>NUM$} {:>NUM$}",
                "n", "sdk cpu", "solang cpu", "diff", "sdk mem", "solang mem"
            );
            for ((n, a), b) in SIZES.iter().zip(s).zip(l) {
                let cell = |c: &Result<Cost, String>, f: fn(&Cost) -> u64| match c {
                    Ok(c) => f(c).to_string(),
                    Err(_) => "err".to_string(),
                };
                let diff = match (a, b) {
                    (Ok(a), Ok(b)) => super::signed(a.cpu as i64 - b.cpu as i64),
                    _ => "—".to_string(),
                };
                println!(
                    "{:<N$} {:>NUM$} {:>NUM$} {:>NUM$} {:>NUM$} {:>NUM$}",
                    n,
                    cell(a, |c| c.cpu),
                    cell(b, |c| c.cpu),
                    diff,
                    cell(a, |c| c.mem),
                    cell(b, |c| c.mem)
                );
            }
            for (side, x) in [("sdk", s), ("solang", l)] {
                if let Some((n, Err(e))) = SIZES.iter().zip(x).find(|(_, c)| c.is_err()) {
                    println!("[err] {side} first failed at n = {n}: {e}");
                }
            }
        }
        println!("\ngrowth: sdk {}, solang {}\n", growth(s), growth(l));
    }

    println!(
        "note: growth is the power of n that fits the cpu from n = 4 to 16 to 64; flat is under 1%"
    );
    println!("note: cpu is in metered instructions and mem in bytes, for one call");
    println!();
}

/// A failed deploy is `{"err": ...}` in place of the sizes; a failed call is `{"err": ...}`
/// in place of its cost.
pub fn json(d: &Data) -> Value {
    let side = |x: &Side| match x {
        Ok(costs) => json!(
            SIZES
                .iter()
                .zip(costs)
                .map(|(n, c)| match c {
                    Ok(c) => json!({ "n": n, "cpu": c.cpu, "mem": c.mem }),
                    Err(e) => json!({ "n": n, "err": e }),
                })
                .collect::<Vec<_>>()
        ),
        Err(e) => json!({ "err": e }),
    };

    let mut functions = Vec::new();
    let (mut skipped, mut no_size) = (Vec::new(), Vec::new());
    for item in &d.items {
        match item {
            Item::Unsupported(sig) => skipped.push(json!(sig)),
            Item::Unsized(sig) => no_size.push(json!(sig)),
            Item::Swept {
                name,
                sig,
                sdk,
                solang,
            } => functions.push(json!({
                "function": name,
                "signature": sig,
                "sdk": side(sdk),
                "solang": side(solang),
                "growth": { "sdk": growth(sdk), "solang": growth(solang) },
            })),
        }
    }

    json!({
        "no_spec": d.no_spec,
        "max": MAX,
        "functions": functions,
        "unsized": no_size,
        "skipped": skipped,
    })
}
