pub mod code;
pub mod names;
pub mod spec;

use std::collections::HashMap;
use wasmparser::{Parser, Payload, RefType, TypeRef};

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
    /// Locally defined functions, in code-section order.
    functions: Vec<code::Function>,
}

impl<'a> Contract<'a> {
    /// Read the module in one pass. The error says what is malformed.
    pub fn load(wasm: &'a [u8]) -> Result<Self, String> {
        let mut sections = Vec::new();
        let mut imports = Vec::new();
        let mut layout = Layout::default();
        let mut spec_bytes: Option<&[u8]> = None;
        let mut functions = Vec::new();

        // Imported function names in index order, so a call to import N can name
        // its target.
        let mut host: Vec<String> = Vec::new();
        let mut exported: HashMap<u32, String> = HashMap::new();

        let bad = |e: wasmparser::BinaryReaderError| format!("malformed wasm: {e}");
        let mut section = |name: &str, size: usize| {
            sections.push(Section {
                name: name.to_string(),
                size,
            })
        };

        for payload in Parser::new(0).parse_all(wasm) {
            match payload.map_err(bad)? {
                Payload::TypeSection(r) => section("type", r.range().len()),
                Payload::ImportSection(r) => {
                    section("import", r.range().len());
                    for im in r {
                        let im = im.map_err(bad)?;
                        imports.push((im.module.to_string(), im.name.to_string()));
                        match im.ty {
                            TypeRef::Func(_) => host.push(
                                names::host_fn_name(im.module, im.name)
                                    .map(str::to_string)
                                    .unwrap_or_else(|| {
                                        format!("{}.{}", im.module, im.name)
                                    }),
                            ),
                            TypeRef::Table(t) => {
                                layout.table = Some((
                                    reftype(t.element_type),
                                    limits(t.initial, t.maximum),
                                ));
                                layout.table_imported = true;
                            }
                            TypeRef::Memory(m) => {
                                layout.memory = Some(limits(
                                    m.initial as u32,
                                    m.maximum.map(|v| v as u32),
                                ));
                                layout.memory_imported = true;
                            }
                            TypeRef::Global(_) | TypeRef::Tag(_) => layout.globals += 1,
                        }
                    }
                }
                Payload::FunctionSection(r) => section("function", r.range().len()),
                Payload::TableSection(r) => {
                    section("table", r.range().len());
                    if let Some(t) = r.into_iter().next() {
                        let t = t.map_err(bad)?.ty;
                        if !layout.table_imported {
                            layout.table = Some((
                                reftype(t.element_type),
                                limits(t.initial, t.maximum),
                            ));
                        }
                    }
                }
                Payload::MemorySection(r) => {
                    section("memory", r.range().len());
                    if let Some(m) = r.into_iter().next() {
                        let m = m.map_err(bad)?;
                        if !layout.memory_imported {
                            layout.memory = Some(limits(
                                m.initial as u32,
                                m.maximum.map(|v| v as u32),
                            ));
                        }
                    }
                }
                Payload::TagSection(r) => section("unknown", r.range().len()),
                Payload::GlobalSection(r) => {
                    section("global", r.range().len());
                    layout.globals += r.count();
                }
                Payload::ExportSection(r) => {
                    section("export", r.range().len());
                    for e in r {
                        let e = e.map_err(bad)?;
                        if matches!(e.kind, wasmparser::ExternalKind::Func) {
                            exported.insert(e.index, e.name.to_string());
                        }
                    }
                }
                Payload::StartSection { range, .. } => section("start", range.len()),
                Payload::ElementSection(r) => section("element", r.range().len()),
                Payload::DataCountSection { range, .. } => {
                    section("data_count", range.len())
                }
                Payload::DataSection(r) => {
                    section("data", r.range().len());
                    layout.data_segments = r.count();
                    layout.data_bytes = r.range().len();
                }
                Payload::CodeSectionStart { range, .. } => section("code", range.len()),
                Payload::CodeSectionEntry(body) => {
                    let index = (host.len() + functions.len()) as u32;
                    let name = exported.get(&index).cloned();
                    functions.push(code::function(&body, index, name, &host)?);
                }
                Payload::CustomSection(r) => {
                    section(&format!("custom:{}", r.name()), r.range().len());
                    if r.name() == "contractspecv0" && spec_bytes.is_none() {
                        spec_bytes = Some(r.data());
                    }
                }
                _ => {}
            }
        }

        Ok(Contract {
            wasm,
            sections,
            imports,
            interface: spec::parse(spec_bytes)?,
            layout,
            functions,
        })
    }

    /// Locally defined functions, in code-section order.
    pub fn functions(&self) -> &[code::Function] {
        &self.functions
    }
}

/// One section's name and payload size. `name` is `custom:<name>` for custom sections.
pub struct Section {
    pub name: String,
    pub size: usize,
}

/// A memory's or table's declared size. No `max` means it can grow unbounded.
#[derive(Clone, Copy)]
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}

fn limits(min: u32, max: Option<u32>) -> Limits {
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

/// A table's element type.
fn reftype(t: RefType) -> &'static str {
    if t == RefType::FUNCREF {
        "funcref"
    } else if t == RefType::EXTERNREF {
        "externref"
    } else {
        "?"
    }
}
