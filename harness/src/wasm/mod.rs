pub mod code;
pub mod names;
pub mod spec;

use std::cell::OnceCell;

/// Every view of one built contract, parsed once up front. Reports read from this
/// instead of re-walking the bytes.
pub struct Contract<'a> {
    /// The raw module, borrowed. Every other field indexes into or derives from it.
    pub wasm: &'a [u8],
    /// Section headers in file order, with each payload's offset and length.
    pub sections: Vec<Section>,
    /// `(module, name)` per import, in import-section order. Every kind is listed —
    /// functions, tables, memories and globals — so this is not a function count.
    pub imports: Vec<(String, String)>,
    /// Function signatures decoded from the `contractspecv0` custom section. Empty
    /// if the module doesn't carry one.
    pub interface: spec::Interface,
    /// Decoded function bodies — see `functions()`. Only the `opcodes` report needs
    /// them, so they're decoded on first use rather than in `load`.
    functions: OnceCell<Vec<code::Function>>,
}

impl<'a> Contract<'a> {
    pub fn load(wasm: &'a [u8]) -> Self {
        let sections = sections(wasm);
        Contract {
            imports: imports(wasm, &sections),
            interface: spec::parse(wasm, &sections),
            functions: OnceCell::new(),
            sections,
            wasm,
        }
    }

    /// Locally defined functions in code-section order, decoded on first call and
    /// cached. Imports have no body, so these start after the *function* imports;
    /// each carries its own `index`, counted by `code::functions` from the import
    /// entries of kind func alone.
    pub fn functions(&self) -> &[code::Function] {
        self.functions.get_or_init(|| code::functions(self.wasm))
    }
}

/// One section as it appears in the module: its name, and where its payload lives.
/// `name` is the spec name, or `custom:<name>` for custom sections.
pub struct Section {
    pub id: u8,
    pub name: String,
    pub start: usize,
    pub size: usize,
}

/// Walk the section headers once, from byte 8 (past the magic and version).
pub fn sections(bytes: &[u8]) -> Vec<Section> {
    let mut out = Vec::new();
    let mut i = 8;

    while i < bytes.len() {
        let id = bytes[i];
        i += 1;

        let (size, len) = read_leb128(&bytes[i..]);
        i += len;
        let size = size as usize;

        let mut name = section_name(id).to_string();
        if id == 0 {
            let (nlen, nb) = read_leb128(&bytes[i..]);
            let start = i + nb;
            let sname = String::from_utf8_lossy(&bytes[start..start + nlen as usize]);
            name = format!("custom:{sname}");
        }

        out.push(Section {
            id,
            name,
            start: i,
            size,
        });
        i += size;
    }
    out
}

/// The spec's name for a section id.
fn section_name(id: u8) -> &'static str {
    match id {
        0 => "custom",
        1 => "type",
        2 => "import",
        3 => "function",
        4 => "table",
        5 => "memory",
        6 => "global",
        7 => "export",
        8 => "start",
        9 => "element",
        10 => "code",
        11 => "data",
        12 => "data_count",
        _ => "unknown",
    }
}

/// Read an unsigned LEB128 integer: (value, bytes_consumed).
pub fn read_leb128(bytes: &[u8]) -> (u32, usize) {
    let mut result = 0u32;
    let mut shift = 0;
    let mut i = 0;

    loop {
        let byte = bytes[i];
        result |= ((byte & 0x7f) as u32) << shift;
        i += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    (result, i)
}

/// Every `(module, name)` a module imports, decoded from the already-walked
/// import section (id 2).
pub fn imports(bytes: &[u8], sections: &[Section]) -> Vec<(String, String)> {
    sections
        .iter()
        .find(|s| s.id == 2)
        .map(|s| decode_imports(&bytes[s.start..s.start + s.size]))
        .unwrap_or_default()
}

/// Decode the body of an import section: a count, then that many entries of
/// `module`, `name`, kind byte, and a kind-specific descriptor we only skip over.
/// The kind byte is `0` func (a type index), `1` table (an elem type then limits),
/// `2` memory (limits), `3` global (a valtype then a mutability byte).
fn decode_imports(body: &[u8]) -> Vec<(String, String)> {
    let mut j = 0;
    let (count, len) = read_leb128(&body[j..]);
    j += len;

    let mut out = Vec::new();
    for _ in 0..count {
        let module = read_name(body, &mut j);
        let name = read_name(body, &mut j);

        let kind = body[j];
        j += 1;

        match kind {
            0 => j += read_leb128(&body[j..]).1,
            1 => {
                j += 1;
                read_limits(body, &mut j);
            }
            2 => read_limits(body, &mut j),
            _ => j += 2,
        }

        out.push((module, name));
    }
    out
}

/// Read a length-prefixed UTF-8 name at `*j`, advancing `*j` past it. `nlen` is the
/// name's length, `nb` the bytes that length field itself took.
fn read_name(body: &[u8], j: &mut usize) -> String {
    let (nlen, nb) = read_leb128(&body[*j..]);
    let start = *j + nb;
    let end = start + nlen as usize;
    *j = end;
    String::from_utf8_lossy(&body[start..end]).into_owned()
}

/// Advance `*j` past a limits: flag byte, min, and max if the flag's low bit is set.
fn read_limits(body: &[u8], j: &mut usize) {
    let flag = body[*j];
    *j += 1;
    *j += read_leb128(&body[*j..]).1;
    if flag & 1 != 0 {
        *j += read_leb128(&body[*j..]).1;
    }
}

/// Payload of the custom section named `want`, past its name prefix, if present.
pub fn custom_section<'a>(bytes: &'a [u8], sections: &[Section], want: &str) -> Option<&'a [u8]> {
    let s = sections
        .iter()
        .find(|s| s.name.strip_prefix("custom:") == Some(want))?;

    let body = &bytes[s.start..s.start + s.size];
    let (nlen, nb) = read_leb128(body);
    Some(&body[nb + nlen as usize..])
}
