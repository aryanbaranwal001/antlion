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

/// One function body. `index` is its position plus the number of imported functions.
pub fn function(
    body: &wasmparser::FunctionBody,
    index: u32,
    name: Option<String>,
    host: &[String],
) -> Result<Function, String> {
    let (locals, local_count) = locals_line(body)?;
    Ok(Function {
        index,
        name,
        bytes: body.range().len(),
        local_count,
        locals,
        instrs: read_body(body, host)?,
    })
}

/// The locals as one line, e.g. `locals: 2× i32, 1× i64`, plus the total count.
/// They're run-length encoded, so the total sums the counts, not the declarations.
fn locals_line(body: &wasmparser::FunctionBody) -> Result<(String, u32), String> {
    let bad = |e: wasmparser::BinaryReaderError| format!("malformed function body: {e}");
    let decls = body
        .get_locals_reader()
        .map_err(bad)?
        .into_iter()
        .collect::<Result<Vec<(u32, wasmparser::ValType)>, _>>()
        .map_err(bad)?;
    let total = decls.iter().map(|(n, _)| n).sum();

    if decls.is_empty() {
        return Ok(("locals: none".to_string(), 0));
    }

    let text: Vec<String> = decls
        .iter()
        .map(|(n, t)| format!("{n}× {}", valtype(*t)))
        .collect();
    Ok((format!("locals: {}", text.join(", ")), total))
}
/// Read the instruction stream, tagging each with its nesting depth.
///
/// A call is written `Call N`, and a call to an imported function also names the host
/// function, e.g. `Call 3 ; put_contract_data`.
fn read_body(body: &wasmparser::FunctionBody, host: &[String]) -> Result<Vec<Instr>, String> {
    let bad = |e: wasmparser::BinaryReaderError| format!("malformed function body: {e}");
    let mut reader = body.get_operators_reader().map_err(bad)?;

    let mut out = Vec::new();
    let mut depth = 0usize;

    while !reader.eof() {
        let op = reader.read().map_err(bad)?;

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
    Ok(out)
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
