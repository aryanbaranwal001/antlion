use std::collections::HashMap;
use wasmparser::{Parser, Payload};

/// One instruction: its `Debug` text with immediates, and its nesting depth.
pub struct Instr {
    pub text: String,
    pub depth: usize,
}

/// One locally defined function.
pub struct Function {
    pub index: u32,
    pub name: Option<String>,
    pub bytes: usize,
    pub local_count: u32,
    pub locals: String,
    pub instrs: Vec<Instr>,
}

impl Function {
    /// Each instruction's opcode name, e.g. `I32Load`, `Call`, `End`.
    pub fn ops(&self) -> impl Iterator<Item = &str> {
        self.instrs.iter().map(|i| variant(&i.text))
    }

    /// Deepest nesting reached in the body.
    pub fn max_depth(&self) -> usize {
        self.instrs.iter().map(|i| i.depth).max().unwrap_or(0)
    }

    /// How many calls the body issues.
    pub fn calls(&self) -> usize {
        self.ops()
            .filter(|n| matches!(*n, "Call" | "CallIndirect"))
            .count()
    }

    /// The disassembly, built on demand — only `--dump` needs it.
    pub fn lines(&self) -> Vec<String> {
        let mut out = vec![self.locals.clone()];
        out.extend(
            self.instrs
                .iter()
                .map(|i| format!("{}{}", "  ".repeat(i.depth), i.text)),
        );
        out
    }
}

/// Every locally defined function, in code-section order. A body's index is its
/// position plus the number of imported functions.
pub fn functions(wasm: &[u8]) -> Vec<Function> {
    let mut imported_funcs = 0u32;
    // Imported function names in index order, so a call to import N can name its target.
    let mut host: Vec<String> = Vec::new();
    let mut names: HashMap<u32, String> = HashMap::new();
    let mut local_i = 0u32;
    let mut out = Vec::new();

    for payload in Parser::new(0).parse_all(wasm) {
        match payload {
            Ok(Payload::ImportSection(reader)) => {
                for im in reader.into_iter().filter_map(Result::ok) {
                    if matches!(im.ty, wasmparser::TypeRef::Func(_)) {
                        imported_funcs += 1;
                        host.push(
                            super::names::host_fn_name(im.module, im.name)
                                .map(str::to_string)
                                .unwrap_or_else(|| format!("{}.{}", im.module, im.name)),
                        );
                    }
                }
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
                let (locals, local_count) = locals_line(&body);
                out.push(Function {
                    index,
                    name: names.get(&index).cloned(),
                    bytes: body.range().len(),
                    local_count,
                    locals,
                    instrs: read_body(&body, &host),
                });
            }
            _ => {}
        }
    }
    out
}

/// The locals as one line, e.g. `locals: 2× i32, 1× i64`, plus the total count.
/// They're run-length encoded, so the total sums the counts, not the declarations.
fn locals_line(body: &wasmparser::FunctionBody) -> (String, u32) {
    let reader = body
        .get_locals_reader()
        .expect("[err] malformed function body: no locals reader");

    let decls: Vec<(u32, wasmparser::ValType)> =
        reader.into_iter().filter_map(Result::ok).collect();
    let total = decls.iter().map(|(n, _)| n).sum();

    if decls.is_empty() {
        return ("locals: none".to_string(), 0);
    }

    let text: Vec<String> = decls
        .iter()
        .map(|(n, t)| format!("{n}× {}", valtype(*t)))
        .collect();
    (format!("locals: {}", text.join(", ")), total)
}
/// Read the instruction stream, tagging each with its nesting depth.
///
/// Only `Block`, `Loop` and `End` occur in our builds — LLVM lowers branches to
/// `block` + `br_if`, so `If`/`Else` never appear.
// TODO: remove redundant If / Else
// TODO: check if all the operators have been taken into account
/// A call is written `Call N`, and a call to an imported function also names the host
/// function, e.g. `Call 3 ; put_contract_data`.
fn read_body(body: &wasmparser::FunctionBody, host: &[String]) -> Vec<Instr> {
    let mut reader = body
        .get_operators_reader()
        .expect("[err] malformed function body: no operators reader");

    let mut out = Vec::new();
    let mut depth = 0usize;

    while !reader.eof() {
        let Ok(op) = reader.read() else { break };

        let text = match op {
            wasmparser::Operator::Call { function_index } => {
                match host.get(function_index as usize) {
                    Some(name) => format!("Call {function_index} ; {name}"),
                    None => format!("Call {function_index}"),
                }
            }
            _ => format!("{op:?}"),
        };
        let n = variant(&text);
        let is_else = n == "Else";
        let closes = n == "End" || is_else;
        let opens = matches!(n, "Block" | "Loop" | "If");

        if closes {
            depth = depth.saturating_sub(1);
        }

        out.push(Instr { text, depth });

        if opens || is_else {
            depth += 1;
        }
    }
    out
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

/// An operator's variant name: the leading alphanumeric run of its `Debug` output,
/// so `I32Const { value: 4 }` becomes `I32Const`.
fn variant(text: &str) -> &str {
    text.split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or("")
}
