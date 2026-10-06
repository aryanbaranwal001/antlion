use serde_json::{Value, json};

use crate::wasm::{Contract, code};

/// The five numbers, for one side.
#[derive(Default)]
struct Metrics {
    bytes: usize,
    instrs: usize,
    locals: u32,
    depth: usize,
    calls: usize,
}

impl Metrics {
    fn of(f: &code::Function) -> Self {
        Metrics {
            bytes: f.bytes,
            instrs: f.instrs.len(),
            locals: f.local_count,
            depth: f.max_depth(),
            calls: f.calls(),
        }
    }

    /// Fold another function in. Depth takes the max — summing unrelated nesting
    /// would mean nothing.
    fn merge(&mut self, o: &Metrics) {
        self.bytes += o.bytes;
        self.instrs += o.instrs;
        self.locals += o.locals;
        self.depth = self.depth.max(o.depth);
        self.calls += o.calls;
    }

    fn json(&self) -> Value {
        json!({
            "bytes": self.bytes,
            "instrs": self.instrs,
            "locals": self.locals,
            "depth": self.depth,
            "calls": self.calls,
        })
    }

    /// `self` minus `o`, metric by metric.
    fn diff_json(&self, o: &Metrics) -> Value {
        let d = |a: usize, b: usize| a as i64 - b as i64;
        json!({
            "bytes": d(self.bytes, o.bytes),
            "instrs": d(self.instrs, o.instrs),
            "locals": d(self.locals as usize, o.locals as usize),
            "depth": d(self.depth, o.depth),
            "calls": d(self.calls, o.calls),
        })
    }
}

/// One exported function, `None` on a side that lacks it.
struct Row {
    name: String,
    sdk: Option<Metrics>,
    solang: Option<Metrics>,
}

pub struct Data {
    exported: Vec<Row>,
    /// Every internal function folded into one `Metrics`, with the count, per side.
    internal: [(Metrics, usize); 2],
}

/// Per-function size and complexity. Exports pair by name; internals are anonymous
/// and unpairable across builds, so they collapse into one summed row.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let (sdk, solang) = (sdk.functions(), solang.functions());

    let exported = exported_names(sdk, solang)
        .into_iter()
        .map(|name| Row {
            sdk: find(sdk, &name).map(Metrics::of),
            solang: find(solang, &name).map(Metrics::of),
            name,
        })
        .collect();

    Data {
        exported,
        internal: [internals(sdk), internals(solang)],
    }
}

pub fn text(d: &Data) {
    super::banner("function-level metrics");

    header();

    for r in &d.exported {
        row(&r.name, r.sdk.as_ref(), r.solang.as_ref());
    }

    let [(s, s_n), (l, l_n)] = &d.internal;
    if s_n + l_n > 0 {
        println!();
        row(&format!("{s_n}│{l_n} internal"), Some(s), Some(l));
    }

    println!();
    println!("note: each function counts only its own code, not the functions it calls");
    println!();
}

/// A side without the function is `null`, and so is the `diff`. `internal` is `null` when
/// neither side has internal functions, as the text then shows no row.
pub fn json(d: &Data) -> Value {
    let exported: Vec<Value> = d
        .exported
        .iter()
        .map(|r| {
            let diff = match (&r.sdk, &r.solang) {
                (Some(s), Some(l)) => s.diff_json(l),
                _ => Value::Null,
            };
            json!({
                "function": r.name,
                "sdk": r.sdk.as_ref().map(Metrics::json),
                "solang": r.solang.as_ref().map(Metrics::json),
                "diff": diff,
            })
        })
        .collect();

    let [(s, s_n), (l, l_n)] = &d.internal;
    let internal = if s_n + l_n > 0 {
        json!({
            "count": { "sdk": s_n, "solang": l_n },
            "sdk": s.json(),
            "solang": l.json(),
            "diff": s.diff_json(l),
        })
    } else {
        Value::Null
    };

    json!({ "exported": exported, "internal": internal })
}

/// Exported names from both builds, sdk order first.
fn exported_names(sdk: &[code::Function], solang: &[code::Function]) -> Vec<String> {
    let mut out: Vec<String> = sdk.iter().filter_map(|f| f.name.clone()).collect();
    for f in solang {
        let Some(n) = &f.name else { continue };
        if !out.contains(n) {
            out.push(n.clone());
        }
    }
    out
}

fn find<'a>(fns: &'a [code::Function], name: &str) -> Option<&'a code::Function> {
    fns.iter().find(|f| f.name.as_deref() == Some(name))
}

/// Every internal function folded into one `Metrics`, plus the count.
fn internals(fns: &[code::Function]) -> (Metrics, usize) {
    let mut m = Metrics::default();
    let mut n = 0;

    for f in fns.iter().filter(|f| f.name.is_none()) {
        m.merge(&Metrics::of(f));
        n += 1;
    }
    (m, n)
}

/// Column widths, each span including the spaces between its cells.
const NAME: usize = 13;
const WIDE: usize = 17;
const NARROW: usize = 14;

/// Two header rows: the metric name over its group, then the per-side labels.
fn header() {
    println!(
        "{:<NAME$} {:^WIDE$} │ {:^WIDE$} │ {:^NARROW$} │ {:^NARROW$} │ {:^NARROW$}",
        "", "bytes", "instrs", "locals", "depth", "calls"
    );
    println!(
        "{:<NAME$} {:>5} {:>5} {:>5} │ {:>5} {:>5} {:>5} │ {:>4} {:>4} {:>4} │ {:>4} {:>4} {:>4} │ {:>4} {:>4} {:>4}",
        "function",
        "sdk",
        "sol",
        "diff",
        "sdk",
        "sol",
        "diff",
        "sdk",
        "sol",
        "diff",
        "sdk",
        "sol",
        "diff",
        "sdk",
        "sol",
        "diff",
    );
    println!(
        "{}",
        "─".repeat(NAME + 1 + 2 * (WIDE + 3) + 3 * (NARROW + 3) - 3)
    );
}

/// One line; a side missing the function prints `—`.
fn row(name: &str, sdk: Option<&Metrics>, solang: Option<&Metrics>) {
    let pair = |s: Option<usize>, l: Option<usize>| match (s, l) {
        (Some(s), Some(l)) => (
            s.to_string(),
            l.to_string(),
            super::signed(s as i64 - l as i64),
        ),
        (Some(s), None) => (s.to_string(), "—".to_string(), "—".to_string()),
        (None, Some(l)) => ("—".to_string(), l.to_string(), "—".to_string()),
        (None, None) => ("—".to_string(), "—".to_string(), "—".to_string()),
    };

    let (sb, lb, db) = pair(sdk.map(|m| m.bytes), solang.map(|m| m.bytes));
    let (si, li, di) = pair(sdk.map(|m| m.instrs), solang.map(|m| m.instrs));
    let (sl, ll, dl) = pair(
        sdk.map(|m| m.locals as usize),
        solang.map(|m| m.locals as usize),
    );
    let (sd, ld, dd) = pair(sdk.map(|m| m.depth), solang.map(|m| m.depth));
    let (sc, lc, dc) = pair(sdk.map(|m| m.calls), solang.map(|m| m.calls));

    println!(
        "{name:<NAME$} {sb:>5} {lb:>5} {db:>5} │ {si:>5} {li:>5} {di:>5} │ {sl:>4} {ll:>4} {dl:>4} │ {sd:>4} {ld:>4} {dd:>4} │ {sc:>4} {lc:>4} {dc:>4}"
    );
}
