pub mod code;
pub mod names;
pub mod spec;

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

pub fn imports(bytes: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut i = 8;

    while i < bytes.len() {
        let id = bytes[i];
        i += 1;

        let (size, len) = read_leb128(&bytes[i..]);
        i += len;
        let size = size as usize;
        let body = &bytes[i..i + size];
        i += size;

        if id == 2 {
            out = decode_imports(body);
        }
    }
    out
}

/// Decode the body of an import section
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
            0 => j += read_leb128(&body[j..]).1, // func
            1 => {
                j += 1; // table
                read_limits(body, &mut j);
            }
            2 => read_limits(body, &mut j), // mem
            _ => j += 2,                    // global
        }

        out.push((module, name));
    }
    out
}

// nlen: name length
// nb: bytes the length field took
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

/// Return the payload bytes of the custom section in wasm named `want`, if present.
pub fn custom_section<'a>(bytes: &'a [u8], want: &str) -> Option<&'a [u8]> {
    let mut i = 8;
    while i < bytes.len() {
        let id = bytes[i];
        i += 1;

        let (size, len) = read_leb128(&bytes[i..]);
        i += len;
        let size = size as usize;
        let body = &bytes[i..i + size];
        i += size;

        if id == 0 {
            let (nlen, nb) = read_leb128(body);
            let name = &body[nb..nb + nlen as usize];
            if name == want.as_bytes() {
                return Some(&body[nb + nlen as usize..]);
            }
        }
    }
    None
}
