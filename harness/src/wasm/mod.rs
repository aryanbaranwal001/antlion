pub mod code;
pub mod names;
pub mod spec;

use std::cell::OnceCell;

/// One built contract, parsed once.
pub struct Contract<'a> {
    /// The raw module.
    pub wasm: &'a [u8],
    /// Section headers, in file order.
    pub sections: Vec<Section>,
    /// Every import, of every kind.
    pub imports: Vec<(String, String)>,
    /// Function signatures from `contractspecv0`.
    pub interface: spec::Interface,
    /// What the VM allocates at instantiation.
    pub layout: Layout,
    /// Function bodies, lazily decoded on first use.
    functions: OnceCell<Vec<code::Function>>,
}

impl<'a> Contract<'a> {
    pub fn load(wasm: &'a [u8]) -> Self {
        let sections = sections(wasm);
        Contract {
            imports: imports(wasm, &sections),
            interface: spec::parse(wasm, &sections),
            layout: layout(wasm, &sections),
            functions: OnceCell::new(),
            sections,
            wasm,
        }
    }

    /// Locally defined functions, in code-section order.
    pub fn functions(&self) -> &[code::Function] {
        self.functions.get_or_init(|| code::functions(self.wasm))
    }
}

/// Where one section's payload lives. `name` is `custom:<name>` for custom sections.
pub struct Section {
    pub id: u8,
    pub name: String,
    pub start: usize,
    pub size: usize,
}

/// Read every section header, from byte 8 past the magic and version.
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

/// Every `(module, name)` the module imports.
pub fn imports(bytes: &[u8], sections: &[Section]) -> Vec<(String, String)> {
    sections
        .iter()
        .find(|s| s.id == 2)
        .map(|s| decode_imports(&bytes[s.start..s.start + s.size]))
        .unwrap_or_default()
}

/// Decode the import section body. Kind byte: `0` func, `1` table, `2` memory,
/// `3` global — each followed by a descriptor we skip.
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
            2 => {
                read_limits(body, &mut j);
            }
            _ => j += 2,
        }

        out.push((module, name));
    }
    out
}

/// Read a length-prefixed UTF-8 name, advancing `*j` past it.
fn read_name(body: &[u8], j: &mut usize) -> String {
    let (nlen, nb) = read_leb128(&body[*j..]);
    let start = *j + nb;
    let end = start + nlen as usize;
    *j = end;
    String::from_utf8_lossy(&body[start..end]).into_owned()
}

/// A memory's or table's declared size. No `max` means it can grow unbounded.
#[derive(Clone, Copy)]
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}

/// Read a limits, advancing `*j` past it.
fn read_limits(body: &[u8], j: &mut usize) -> Limits {
    let flag = body[*j];
    *j += 1;

    let (min, n) = read_leb128(&body[*j..]);
    *j += n;

    let mut max = None;
    if flag & 1 != 0 {
        let (m, n) = read_leb128(&body[*j..]);
        *j += n;
        max = Some(m);
    }
    Limits { min, max }
}

/// What the VM allocates at instantiation, before any code runs.
#[derive(Default)]
pub struct Layout {
    pub memory: Option<Limits>,
    pub memory_imported: bool,
    pub table: Option<(&'static str, Limits)>,
    pub table_imported: bool,
    pub globals: u32,
    pub data_segments: u32,
    /// Data section payload size, segment headers included.
    pub data_bytes: usize,
}

/// Decode the memory, table, global and data declarations, defined or imported.
pub fn layout(bytes: &[u8], sections: &[Section]) -> Layout {
    let mut out = Layout::default();
    let body = |id: u8| {
        sections
            .iter()
            .find(|s| s.id == id)
            .map(|s| (&bytes[s.start..s.start + s.size], s.size))
    };

    if let Some((b, _)) = body(5) {
        let (_, mut j) = read_leb128(b);
        out.memory = Some(read_limits(b, &mut j));
    }

    if let Some((b, _)) = body(4) {
        let (n, mut j) = read_leb128(b);
        if n > 0 {
            let elem = reftype(b[j]);
            j += 1;
            out.table = Some((elem, read_limits(b, &mut j)));
        }
    }

    if let Some((b, _)) = body(6) {
        out.globals = read_leb128(b).0;
    }

    if let Some((b, size)) = body(11) {
        out.data_segments = read_leb128(b).0;
        out.data_bytes = size;
    }

    if let Some((b, _)) = body(2) {
        imported_layout(b, &mut out);
    }
    out
}

/// Fold imported memories, tables and globals into `out`.
fn imported_layout(body: &[u8], out: &mut Layout) {
    let (count, mut j) = read_leb128(body);

    for _ in 0..count {
        read_name(body, &mut j);
        read_name(body, &mut j);

        let kind = body[j];
        j += 1;

        match kind {
            0 => j += read_leb128(&body[j..]).1,
            1 => {
                let elem = reftype(body[j]);
                j += 1;
                out.table = Some((elem, read_limits(body, &mut j)));
                out.table_imported = true;
            }
            2 => {
                out.memory = Some(read_limits(body, &mut j));
                out.memory_imported = true;
            }
            _ => {
                j += 2;
                out.globals += 1;
            }
        }
    }
}

/// A table's element type.
fn reftype(b: u8) -> &'static str {
    match b {
        0x70 => "funcref",
        0x6f => "externref",
        _ => "?",
    }
}

/// Payload of the custom section named `want`, past its name prefix.
pub fn custom_section<'a>(bytes: &'a [u8], sections: &[Section], want: &str) -> Option<&'a [u8]> {
    let s = sections
        .iter()
        .find(|s| s.name.strip_prefix("custom:") == Some(want))?;

    let body = &bytes[s.start..s.start + s.size];
    let (nlen, nb) = read_leb128(body);
    Some(&body[nb + nlen as usize..])
}
