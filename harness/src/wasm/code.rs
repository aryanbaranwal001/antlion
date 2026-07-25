// Read the instruction stream of every function body, via `wasmparser`.
use std::collections::HashMap;
use wasmparser::{Parser, Payload};

/// The variant name of every instruction across all function bodies (e.g. `I32Load`,
/// `Call`, `End`). Imported host functions have no body, so only the module's own code
/// is seen.
pub fn opcodes(wasm: &[u8]) -> Vec<String> {
    let mut out = Vec::new();

    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(Payload::CodeSectionEntry(body)) = payload else {
            continue;
        };
        let Ok(mut reader) = body.get_operators_reader() else {
            continue;
        };
        while !reader.eof() {
            match reader.read() {
                Ok(op) => out.push(name(&op)),
                Err(_) => break,
            }
        }
    }
    out
}

/// One locally-defined function: its module index, its export name (if exported), and
/// its disassembly lines (the `locals:` line followed by the instruction stream).
pub struct Function {
    pub index: u32,
    pub name: Option<String>,
    pub lines: Vec<String>,
}

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
                emit_body(&body, &mut lines);
                out.push(Function {
                    index,
                    name: names.get(&index).cloned(),
                    lines,
                });
            }
            _ => {}
        }
    }
    out
}

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
fn emit_body(body: &wasmparser::FunctionBody, out: &mut Vec<String>) {
    let mut reader = body
        .get_operators_reader()
        .expect("malformed function body: no operators reader");

    let mut depth = 0usize;

    while !reader.eof() {
        let Ok(op) = reader.read() else { break };

        let n = name(&op);
        let closes = n == "End" || n == "Else";
        let opens = matches!(n.as_str(), "Block" | "Loop" | "If");

        if closes {
            depth = depth.saturating_sub(1);
        }

        out.push(format!("{}{op:?}", "  ".repeat(depth)));

        if opens || n == "Else" {
            depth += 1;
        }
    }
}

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

fn name(op: &wasmparser::Operator) -> String {
    format!("{op:?}")
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
        .to_string()
}
