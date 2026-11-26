use std::collections::BTreeMap;

use rayon::prelude::*;
use serde_json::{Value, json};

use super::access::{self, HostCall};
use crate::wasm::{Contract, spec};

const SIDE: usize = 7;
const NAME: usize = 40;
const NUM: usize = 6;

/// The longest argument shown in the text, in characters. JSON has them in full.
const ARG: usize = 40;

/// One side's call: what it returned, and every host call it made, in order.
struct Run {
    result: String,
    calls: Vec<HostCall>,
}

type Side = Result<Run, String>;

/// How one step of the two traces lines up.
#[derive(Clone, Copy, PartialEq)]
enum Step {
    /// The same host function with the same arguments on both sides.
    Same(usize, usize),
    /// The same host function with different arguments.
    Args(usize, usize),
    SdkOnly(usize),
    SolangOnly(usize),
}

enum Item {
    Unsupported(String),
    Traced {
        name: String,
        sig: String,
        sdk: Side,
        solang: Side,
        /// The two traces lined up, when both calls ran.
        steps: Vec<Step>,
    },
}

pub struct Data {
    no_spec: bool,
    items: Vec<Item>,
}

/// Invoke every function once on both builds, as the network does, and record every host
/// call each makes, in order, with its arguments. The run is the one `access` makes.
///
/// The two traces are lined up by host function name with a longest common subsequence,
/// so a call one side makes and the other does not shows where it happens.
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
                access::trace(wasm, iface, &f.name, &f.inputs)
                    .map(|a| Run {
                        result: a.result,
                        calls: a.trace,
                    })
                    .map_err(|e| super::one_line(&format!("{e:?}")))
            };
            let (s, l) = (side(sw), side(lw));
            let steps = match (&s, &l) {
                (Ok(a), Ok(b)) => align(&a.calls, &b.calls),
                _ => Vec::new(),
            };
            Item::Traced {
                name: f.name.clone(),
                sig: spec::sig(f),
                sdk: s,
                solang: l,
                steps,
            }
        })
        .collect();

    Data {
        no_spec: iface.funcs.is_empty(),
        items,
    }
}

/// Line up two traces by host function name, keeping the longest run of names both share
/// in order.
fn align(a: &[HostCall], b: &[HostCall]) -> Vec<Step> {
    let (n, m) = (a.len(), b.len());
    // `len[i][j]` is the longest common subsequence of `a[i..]` and `b[j..]`.
    let mut len = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            len[i][j] = if a[i].name == b[j].name {
                len[i + 1][j + 1] + 1
            } else {
                len[i + 1][j].max(len[i][j + 1])
            };
        }
    }

    let mut steps = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && a[i].name == b[j].name {
            steps.push(if a[i].args == b[j].args {
                Step::Same(i, j)
            } else {
                Step::Args(i, j)
            });
            i += 1;
            j += 1;
        } else if j == m || (i < n && len[i + 1][j] >= len[i][j + 1]) {
            steps.push(Step::SdkOnly(i));
            i += 1;
        } else {
            steps.push(Step::SolangOnly(j));
            j += 1;
        }
    }
    steps
}

/// How many steps of each kind: the same, other arguments, sdk only, solang only.
fn tally(steps: &[Step]) -> (usize, usize, usize, usize) {
    let mut t = (0, 0, 0, 0);
    for s in steps {
        match s {
            Step::Same(..) => t.0 += 1,
            Step::Args(..) => t.1 += 1,
            Step::SdkOnly(_) => t.2 += 1,
            Step::SolangOnly(_) => t.3 += 1,
        }
    }
    t
}

/// How many times each host function was called.
fn counts(calls: &[HostCall]) -> BTreeMap<&'static str, usize> {
    let mut c = BTreeMap::new();
    for call in calls {
        *c.entry(call.name).or_insert(0) += 1;
    }
    c
}

/// A host call on one line, each argument cut to `ARG` characters.
fn render(c: &HostCall) -> String {
    let args: Vec<String> = c
        .args
        .iter()
        .map(|a| {
            if a.chars().count() > ARG {
                format!("{}…", a.chars().take(ARG).collect::<String>())
            } else {
                a.clone()
            }
        })
        .collect();
    format!("{}({})", c.name, args.join(", "))
}

pub fn text(name: &str, d: &Data) {
    super::banner("host call trace");

    if d.no_spec {
        println!("[skip] no contract spec found for `{name}`, nothing to invoke");
        println!();
        return;
    }

    for item in &d.items {
        let (sig, s, l, steps) = match item {
            Item::Unsupported(sig) => {
                println!("[skip] {sig}: unsupported arg types");
                continue;
            }
            Item::Traced {
                sig,
                sdk,
                solang,
                steps,
                ..
            } => (sig, sdk, solang, steps),
        };

        println!("{sig}\n");
        let (a, b) = match (s, l) {
            (Ok(a), Ok(b)) => (a, b),
            _ => {
                for (side, x) in [("sdk", s), ("solang", l)] {
                    if let Err(e) = x {
                        println!("{side:<SIDE$} [err] {e}");
                    }
                }
                println!();
                continue;
            }
        };

        // D.1: which host functions, and how many times.
        println!(
            "{:<NAME$} {:>NUM$} {:>NUM$}",
            "host function", "sdk", "solang"
        );
        let (ca, cb) = (counts(&a.calls), counts(&b.calls));
        let mut names: Vec<&str> = ca.keys().chain(cb.keys()).copied().collect();
        names.sort();
        names.dedup();
        for n in &names {
            let get = |c: &BTreeMap<&str, usize>| c.get(n).copied().unwrap_or(0);
            println!("{n:<NAME$} {:>NUM$} {:>NUM$}", get(&ca), get(&cb));
        }
        println!(
            "{:<NAME$} {:>NUM$} {:>NUM$}",
            "total",
            a.calls.len(),
            b.calls.len()
        );
        println!("{:<NAME$} {:<NUM$} {}", "result", a.result, b.result);
        println!();

        // D.2 and D.3: the two traces in order, lined up, with their arguments.
        if !steps.is_empty() {
            for step in steps {
                match *step {
                    Step::Same(i, _) => println!("  =  {:<SIDE$} {}", "both", render(&a.calls[i])),
                    Step::Args(i, j) => {
                        println!("  ~  {:<SIDE$} {}", "sdk", render(&a.calls[i]));
                        println!("     {:<SIDE$} {}", "solang", render(&b.calls[j]));
                    }
                    Step::SdkOnly(i) => println!("  -  {:<SIDE$} {}", "sdk", render(&a.calls[i])),
                    Step::SolangOnly(j) => {
                        println!("  +  {:<SIDE$} {}", "solang", render(&b.calls[j]))
                    }
                }
            }
            println!();
        }
        let (same, args, so, lo) = tally(steps);
        let verdict = if so == 0 && lo == 0 && args == 0 {
            "same".to_string()
        } else {
            format!(
                "differs: {same} same, {args} with other arguments, {so} sdk only, {lo} solang only"
            )
        };
        println!("trace: {verdict}\n");
    }

    if super::notes() {
        println!(
            "note: = the same call on both sides, ~ the same host function with other arguments, - sdk only, + solang only"
        );
        println!(
            "note: an object argument is a handle, such as Vec(obj#3), so two equal objects can show different numbers"
        );
        println!();
    }
}

pub fn json(d: &Data) -> Value {
    let calls = |c: &[HostCall]| -> Vec<Value> {
        c.iter()
            .map(|c| json!({ "name": c.name, "args": c.args }))
            .collect()
    };
    let side = |x: &Side| match x {
        Ok(r) => json!({ "result": r.result, "calls": calls(&r.calls) }),
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
                steps,
            } => {
                let mut v = json!({
                    "function": name,
                    "signature": sig,
                    "sdk": side(sdk),
                    "solang": side(solang),
                });
                if let (Ok(a), Ok(b)) = (sdk, solang) {
                    let (ca, cb) = (counts(&a.calls), counts(&b.calls));
                    let mut names: Vec<&str> = ca.keys().chain(cb.keys()).copied().collect();
                    names.sort();
                    names.dedup();
                    let counts: serde_json::Map<String, Value> = names
                        .iter()
                        .map(|n| {
                            let get = |c: &BTreeMap<&str, usize>| c.get(n).copied().unwrap_or(0);
                            (
                                n.to_string(),
                                json!({ "sdk": get(&ca), "solang": get(&cb) }),
                            )
                        })
                        .collect();
                    let (same, args, so, lo) = tally(steps);
                    v["counts"] = json!(counts);
                    v["order"] = json!({
                        "same": same,
                        "other_args": args,
                        "sdk_only": so,
                        "solang_only": lo,
                    });
                }
                functions.push(v);
            }
        }
    }

    json!({ "no_spec": d.no_spec, "functions": functions, "skipped": skipped })
}
