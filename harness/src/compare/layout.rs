use serde_json::{Value, json};

use crate::wasm::{Contract, Layout};

/// A wasm page is 64 KiB.
const PAGE: u64 = 64 * 1024;

/// One table line: its JSON key, its text label, and a cell per side.
struct Row {
    key: &'static str,
    label: &'static str,
    sdk: Cell,
    solang: Cell,
}

pub struct Data {
    rows: Vec<Row>,
    notes: Vec<String>,
}

/// What each build declares for the VM to allocate at instantiation.
pub fn collect(sdk: &Contract, solang: &Contract) -> Data {
    let (s, l) = (&sdk.layout, &solang.layout);
    let row = |key, label, sdk, solang| Row {
        key,
        label,
        sdk,
        solang,
    };

    let rows = vec![
        row(
            "memory_pages_min",
            "memory pages (min)",
            pages(s, |m| Some(m.min)),
            pages(l, |m| Some(m.min)),
        ),
        row(
            "memory_pages_max",
            "memory pages (max)",
            pages(s, |m| m.max),
            pages(l, |m| m.max),
        ),
        row("table", "table", table(s), table(l)),
        row(
            "table_min",
            "table min",
            table_limit(s, |t| Some(t.min)),
            table_limit(l, |t| Some(t.min)),
        ),
        row(
            "table_max",
            "table max",
            table_limit(s, |t| t.max),
            table_limit(l, |t| t.max),
        ),
        row(
            "globals",
            "globals",
            num(s.globals as u64),
            num(l.globals as u64),
        ),
        row(
            "data_segments",
            "data segments",
            num(s.data_segments as u64),
            num(l.data_segments as u64),
        ),
        row(
            "data_bytes",
            "data bytes",
            num(s.data_bytes as u64),
            num(l.data_bytes as u64),
        ),
    ];

    Data {
        rows,
        notes: notes(s, l),
    }
}

pub fn text(d: &Data) {
    super::banner("declared layout");
    println!(
        "{:<22} {:>14} {:>14} {:>9}",
        "field", "sdk", "solang", "diff"
    );

    for r in &d.rows {
        row(r.label, &r.sdk, &r.solang);
    }

    println!();
    for n in &d.notes {
        println!("note: {n}");
    }
    println!();
}

/// Numbers are raw, pages included. A cell the text shows as `—` is `null`. `table` holds
/// the element type, or `null` when there is no table, and has no `diff`.
pub fn json(d: &Data) -> Value {
    let mut out = serde_json::Map::new();
    for r in &d.rows {
        let v = if r.key == "table" {
            let elem = |c: &Cell| match c.0.as_str() {
                "absent" => Value::Null,
                t => json!(t),
            };
            json!({ "sdk": elem(&r.sdk), "solang": elem(&r.solang) })
        } else {
            let diff = match (r.sdk.1, r.solang.1) {
                (Some(a), Some(b)) => json!(a as i64 - b as i64),
                _ => Value::Null,
            };
            json!({ "sdk": r.sdk.1, "solang": r.solang.1, "diff": diff })
        };
        out.insert(r.key.to_string(), v);
    }
    out.insert("notes".to_string(), json!(d.notes));
    Value::Object(out)
}

/// A cell: the rendered text, plus the number behind it when there is one.
type Cell = (String, Option<u64>);

fn num(v: u64) -> Cell {
    (v.to_string(), Some(v))
}

fn none() -> Cell {
    ("—".to_string(), None)
}

/// A page count with its byte equivalent, e.g. `16 (1 MiB)`.
fn pages(l: &Layout, pick: impl Fn(&crate::wasm::Limits) -> Option<u32>) -> Cell {
    match l.memory.as_ref().and_then(|m| pick(m)) {
        Some(p) => (format!("{p} ({})", bytes(p as u64 * PAGE)), Some(p as u64)),
        None => none(),
    }
}

fn table(l: &Layout) -> Cell {
    match &l.table {
        Some((elem, _)) => ((*elem).to_string(), None),
        None => ("absent".to_string(), None),
    }
}

fn table_limit(l: &Layout, pick: impl Fn(&crate::wasm::Limits) -> Option<u32>) -> Cell {
    match l.table.as_ref().and_then(|(_, lim)| pick(lim)) {
        Some(v) => num(v as u64),
        None => none(),
    }
}

/// Bytes as KiB/MiB.
fn bytes(n: u64) -> String {
    match n {
        n if n >= 1 << 20 => format!("{} MiB", n / (1 << 20)),
        n if n >= 1 << 10 => format!("{} KiB", n / (1 << 10)),
        n => format!("{n} B"),
    }
}

/// One line; the diff is `—` unless both sides gave a number.
fn row(label: &str, sdk: &Cell, solang: &Cell) {
    let diff = match (sdk.1, solang.1) {
        (Some(a), Some(b)) => super::signed(a as i64 - b as i64),
        _ => "—".to_string(),
    };
    println!("{label:<22} {:>14} {:>14} {diff:>9}", sdk.0, solang.0);
}

/// Divergences that don't show up as a number in the table.
fn notes(sdk: &Layout, solang: &Layout) -> Vec<String> {
    let mut out = Vec::new();

    let growable = |l: &Layout| matches!(&l.memory, Some(m) if m.max.is_none());
    if growable(sdk) != growable(solang) {
        let (yes, no) = if growable(sdk) {
            ("sdk", "solang")
        } else {
            ("solang", "sdk")
        };
        out.push(format!("{yes} memory can grow past its minimum, {no} memory cannot"));
    }

    if sdk.table.is_some() != solang.table.is_some() {
        let side = if sdk.table.is_some() { "solang" } else { "sdk" };
        out.push(format!("{side} declares no table, so it makes no indirect calls"));
    }

    for (name, l) in [("sdk", sdk), ("solang", solang)] {
        if l.memory_imported {
            out.push(format!("{name} imports its memory rather than defining one"));
        }
        if l.table_imported {
            out.push(format!("{name} imports its table rather than defining one"));
        }
    }
    out
}
