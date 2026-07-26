use crate::wasm::{Contract, Layout};

/// A wasm page is 64 KiB — a bare page count doesn't read as a size.
const PAGE: u64 = 64 * 1024;

/// What each build declares for the VM to allocate at instantiation: linear memory,
/// the function table, globals and data segments. `sections` already gives the byte
/// size of these sections; this gives their contents.
pub fn report(sdk: &Contract, solang: &Contract) {
    super::banner("declared layout");
    println!(
        "{:<22} {:>14} {:>14} {:>9}",
        "field", "sdk", "solang", "diff"
    );

    let (s, l) = (&sdk.layout, &solang.layout);

    row(
        "memory pages (min)",
        pages(s, |m| Some(m.min)),
        pages(l, |m| Some(m.min)),
    );
    row(
        "memory pages (max)",
        pages(s, |m| m.max),
        pages(l, |m| m.max),
    );
    row("table", table(s), table(l));
    row(
        "table min",
        table_limit(s, |t| Some(t.min)),
        table_limit(l, |t| Some(t.min)),
    );
    row(
        "table max",
        table_limit(s, |t| t.max),
        table_limit(l, |t| t.max),
    );
    row("globals", num(s.globals as u64), num(l.globals as u64));
    row(
        "data segments",
        num(s.data_segments as u64),
        num(l.data_segments as u64),
    );
    row(
        "data bytes",
        num(s.data_bytes as u64),
        num(l.data_bytes as u64),
    );

    println!();
    notes(s, l);
    println!();
}

/// A cell: the rendered text, plus the number behind it when there is one, so the
/// diff column can be computed without re-deriving it.
type Cell = (String, Option<u64>);

fn num(v: u64) -> Cell {
    (v.to_string(), Some(v))
}

fn none() -> Cell {
    ("—".to_string(), None)
}

/// A page count rendered with its byte equivalent, e.g. `16 (1 MiB)`.
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

/// Bytes as KiB/MiB — page counts are only meaningful once converted.
fn bytes(n: u64) -> String {
    match n {
        n if n >= 1 << 20 => format!("{} MiB", n / (1 << 20)),
        n if n >= 1 << 10 => format!("{} KiB", n / (1 << 10)),
        n => format!("{n} B"),
    }
}

/// One line; the diff is `—` unless both sides produced a number.
fn row(label: &str, sdk: Cell, solang: Cell) {
    let diff = match (sdk.1, solang.1) {
        (Some(a), Some(b)) => super::signed(a as i64 - b as i64),
        _ => "—".to_string(),
    };
    println!("{label:<22} {:>14} {:>14} {diff:>9}", sdk.0, solang.0);
}

/// Call out the divergences that matter but don't show up as a number in the table.
fn notes(sdk: &Layout, solang: &Layout) {
    let growable = |l: &Layout| matches!(&l.memory, Some(m) if m.max.is_none());
    if growable(sdk) != growable(solang) {
        let (yes, no) = if growable(sdk) {
            ("sdk", "solang")
        } else {
            ("solang", "sdk")
        };
        println!("note: {yes} memory can grow past its minimum, {no} memory cannot");
    }

    if sdk.table.is_some() != solang.table.is_some() {
        let side = if sdk.table.is_some() { "solang" } else { "sdk" };
        println!("note: {side} declares no table, so it makes no indirect calls");
    }

    for (name, l) in [("sdk", sdk), ("solang", solang)] {
        if l.memory_imported {
            println!("note: {name} imports its memory rather than defining one");
        }
        if l.table_imported {
            println!("note: {name} imports its table rather than defining one");
        }
    }
}
