pub const DOC_START: &[u8] = b"\xD3\xA8\xA8";
pub const DOC_END: &[u8] = b"\xD3\xA9\xA8";
pub const PG_START: &[u8] = b"\xD3\xA8\xAD";
pub const PG_END: &[u8] = b"\xD3\xA9\xAD";
pub const PAGE_START: &[u8] = b"\xD3\xA8\xAF";
pub const PAGE_END: &[u8] = b"\xD3\xA9\xAF";
pub const PF_START: &[u8] = b"\xD3\xA8\xA5";
pub const PF_END: &[u8] = b"\xD3\xA9\xA5";

// type code X'EE' is Data, so NOP sits next to IPD (D3EEFB), PTX (D3EE9B) and
// OCD (D3EE92) — match the full three bytes, never just the type
pub const NOP_SF: &[u8] = b"\xD3\xEE\xEE";
pub const TLE_SF: &[u8] = b"\xD3\xA0\x90";
