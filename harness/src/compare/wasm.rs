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
