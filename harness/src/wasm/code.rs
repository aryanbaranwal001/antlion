// Read the instruction stream of every function body, via `wasmparser`.
use std::collections::HashMap;
use wasmparser::{Parser, Payload};

/// One locally-defined function: its module index, its export name (if exported), its
/// disassembly lines (the `locals:` line followed by the instruction stream), and the
/// variant name of each instruction in order (e.g. `I32Load`, `Call`, `End`).
pub struct Function {
    pub index: u32,
    pub name: Option<String>,
    pub lines: Vec<String>,
    pub ops: Vec<String>,
}

/// Every locally-defined function, in code-section order. The code section stores no
/// indices — imports occupy `0..k`, so a body's index is `k` plus its position — and
/// export names are looked up against that index.
pub fn functions(wasm: &[u8]) -> Vec<Function> {
    let mut imported_funcs = 0u32;
    let mut names: HashMap<u32, String> = HashMap::new();
    let mut local_i = 0u32;
    let mut out = Vec::new();

    for payload in Parser::new(0).parse_all(wasm) {
        match payload {
            Ok(Payload::ImportSection(reader)) => {
                imported_funcs += reader
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|im| matches!(im.ty, wasmparser::TypeRef::Func(_)))
                    .count() as u32;
            }

            Ok(Payload::ExportSection(reader)) => {
                for e in reader.into_iter().filter_map(Result::ok) {
                    if matches!(e.kind, wasmparser::ExternalKind::Func) {
                        names.insert(e.index, e.name.to_string());
                    }
                }
            }
            Ok(Payload::CodeSectionEntry(body)) => {
                let index = imported_funcs + local_i;
                local_i += 1;
                let mut lines = vec![locals_line(&body)];
                let ops = emit_body(&body, &mut lines);
                out.push(Function {
                    index,
                    name: names.get(&index).cloned(),
                    lines,
                    ops,
                });
            }
            _ => {}
        }
    }
    out
}

/// The function's local declarations as one line, e.g. `locals: 2× i32, 1× i64`.
/// Locals are stored run-length encoded, hence the `count× type` pairs.
fn locals_line(body: &wasmparser::FunctionBody) -> String {
    let reader = body
        .get_locals_reader()
        .expect("malformed function body: no locals reader");

    let decls: Vec<String> = reader
        .into_iter()
        .filter_map(Result::ok)
        .map(|(n, t)| format!("{n}× {}", valtype(t)))
        .collect();

    if decls.is_empty() {
        "locals: none".to_string()
    } else {
        format!("locals: {}", decls.join(", "))
    }
}
/// Push one line per instruction, indenting by control-flow nesting depth.
///
/// Of the arms below, only `Block`, `Loop` and `End` occur in our builds — LLVM lowers
/// branches to `block` + `br_if`, so `If`/`Else` never appear. They're kept for
/// hand-written wat and non-LLVM toolchains, but are untested here.
// TODO: remove redundant If / Else
// TODO: check if all the operators have been taken into account
fn emit_body(body: &wasmparser::FunctionBody, out: &mut Vec<String>) -> Vec<String> {
    let mut reader = body
        .get_operators_reader()
        .expect("malformed function body: no operators reader");

    let mut ops = Vec::new();
    let mut depth = 0usize;

    while !reader.eof() {
        let Ok(op) = reader.read() else { break };

        let text = format!("{op:?}");
        let n = variant(&text);
        let closes = n == "End" || n == "Else";
        let opens = matches!(n, "Block" | "Loop" | "If");

        if closes {
            depth = depth.saturating_sub(1);
        }

        out.push(format!("{}{text}", "  ".repeat(depth)));
        ops.push(n.to_string());

        if opens || n == "Else" {
            depth += 1;
        }
    }
    ops
}

/// A value type's spec name.
fn valtype(t: wasmparser::ValType) -> &'static str {
    use wasmparser::ValType::*;

    match t {
        I32 => "i32",
        I64 => "i64",
        F32 => "f32",
        F64 => "f64",
        V128 => "v128",
        Ref(_) => "ref",
    }
}

/// An operator's variant name, taken as the leading alphanumeric run of its `Debug`
/// output — `I32Const { value: 4 }` becomes `I32Const`. Avoids matching ~200 variants
/// by hand, at the cost of depending on `Debug` formatting.
fn variant(text: &str) -> &str {
    text.split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
}
