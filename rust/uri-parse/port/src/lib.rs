//! C ABI for `uri_split_wrap.c`: drop-in replacements with the original names and signatures, generated
//! by rust-anything from `rust-anything.json`. All of the crate's `unsafe` code lives in this file.

pub mod port;
/// Safe entry point for Rust callers: `-1` when the original `uri_split()`
/// rejects the bytes, otherwise the packed field set / port / flags.
pub fn summary(bytes: &[u8]) -> i64 {
    port::uri_split_summary(Some(bytes))
}

use std::ffi::c_char;

mod __rust_anything_c_abi {
    use std::ffi::{CStr, c_char};

    /// A `const char *` argument as bytes; `None` for `NULL`.
    pub(super) fn c_str<'a>(ptr: *const c_char) -> Option<&'a [u8]> {
        if ptr.is_null() {
            return None;
        }

        // SAFETY: the C API takes NUL-terminated strings that stay valid for the call;
        // NULL is handled above.
        Some(unsafe { CStr::from_ptr(ptr) }.to_bytes())
    }
}

/// `int64_t uri_split_summary(const char* uri)` (`uri_split_wrap.c:6`).
#[cfg(not(feature = "shadow"))]
// SAFETY: exported under the C name it replaces: link it instead of the original object.
#[unsafe(no_mangle)]
pub extern "C" fn uri_split_summary(uri: *const c_char) -> i64 {
    port::uri_split_summary(crate::__rust_anything_c_abi::c_str(uri))
}

/// `uri_split_summary` as `rust_anything_uri_split_summary`, for shadow builds beside the original.
// SAFETY: a name no C code defines.
#[unsafe(no_mangle)]
pub extern "C" fn rust_anything_uri_split_summary(uri: *const c_char) -> i64 {
    port::uri_split_summary(crate::__rust_anything_c_abi::c_str(uri))
}
