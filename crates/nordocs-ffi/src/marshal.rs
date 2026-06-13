//! Byte-buffer and UTF-8 string marshalling across the FFI boundary.
//!
//! Both [`ByteBuffer`] and [`FfiString`] are flat, `#[repr(C)]` `(ptr, len,
//! capacity)` triples that hand ownership of a heap allocation to the caller.
//!
//! ## Ownership rules (single-call)
//!
//! - A [`ByteBuffer`] / [`FfiString`] returned across the boundary is **owned by
//!   the caller**. The caller MUST free it exactly once with the matching free
//!   export — [`ndoc_byte_buffer_free`] / [`ndoc_string_free`] — and MUST NOT
//!   read the pointer afterwards.
//! - `data` may be null only when `len == 0` (the empty value); the free
//!   exports tolerate a null pointer.
//! - The `(len, capacity)` pair MUST be passed back to the free export
//!   unchanged: it is the exact `Vec` layout the allocation was built from.
//!   Freeing a buffer that was not produced by this library, or freeing twice,
//!   is undefined behaviour.

use interoptopus::{ffi_function, ffi_type};

/// An owned, heap-allocated byte buffer handed to the caller.
///
/// Built from a `Vec<u8>` (e.g. PDF/SVG/PNG bytes from `nordocs-core`). Free it
/// exactly once with [`ndoc_byte_buffer_free`].
#[ffi_type]
#[repr(C)]
pub struct ByteBuffer {
    /// Pointer to the first byte, or null when `len == 0`.
    pub data: *mut u8,
    /// Number of valid bytes. `u64` (not `usize`) for a fixed cross-platform ABI.
    pub len: u64,
    /// Backing allocation capacity (the `Vec` capacity); needed to free safely.
    pub capacity: u64,
}

impl ByteBuffer {
    /// The empty buffer (null pointer, zero length).
    pub fn empty() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }

    /// Move a `Vec<u8>` out across the boundary, transferring ownership.
    pub fn from_vec(mut bytes: Vec<u8>) -> Self {
        if bytes.is_empty() {
            return Self::empty();
        }
        let data = bytes.as_mut_ptr();
        let len = bytes.len() as u64;
        let capacity = bytes.capacity() as u64;
        std::mem::forget(bytes);
        Self {
            data,
            len,
            capacity,
        }
    }

    /// Reclaim the backing `Vec` so it is dropped.
    ///
    /// # Safety
    /// Must be called at most once per buffer, with the `(len, capacity)`
    /// originally produced by [`ByteBuffer::from_vec`].
    unsafe fn into_owned(self) {
        if !self.data.is_null() {
            drop(Vec::from_raw_parts(
                self.data,
                self.len as usize,
                self.capacity as usize,
            ));
        }
    }
}

impl From<Vec<u8>> for ByteBuffer {
    fn from(bytes: Vec<u8>) -> Self {
        Self::from_vec(bytes)
    }
}

/// An owned, heap-allocated UTF-8 string handed to the caller.
///
/// The bytes in `[data, data + len)` are guaranteed valid UTF-8. Free it exactly
/// once with [`ndoc_string_free`].
#[ffi_type]
#[repr(C)]
#[derive(Debug)]
pub struct FfiString {
    /// Pointer to the first UTF-8 byte, or null when `len == 0`.
    pub data: *mut u8,
    /// Number of UTF-8 bytes (not code points, not NUL-terminated). `u64` for a
    /// fixed cross-platform ABI.
    pub len: u64,
    /// Backing allocation capacity; needed to free safely.
    pub capacity: u64,
}

impl FfiString {
    /// The empty string (null pointer, zero length).
    pub fn empty() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }

    /// Move a `String` out across the boundary, transferring ownership.
    pub fn from_string(s: String) -> Self {
        let buffer = ByteBuffer::from_vec(s.into_bytes());
        Self {
            data: buffer.data,
            len: buffer.len,
            capacity: buffer.capacity,
        }
    }

    /// Reclaim the backing allocation so it is dropped.
    ///
    /// # Safety
    /// Must be called at most once per string, with the `(len, capacity)`
    /// originally produced by [`FfiString::from_string`].
    unsafe fn into_owned(self) {
        if !self.data.is_null() {
            drop(Vec::from_raw_parts(
                self.data,
                self.len as usize,
                self.capacity as usize,
            ));
        }
    }

    /// Borrow the contents as `&str` (test-only helper).
    #[cfg(test)]
    fn as_str(&self) -> &str {
        if self.data.is_null() {
            return "";
        }
        // SAFETY: `from_string` only ever stores valid UTF-8 of length `len`.
        unsafe {
            let slice = std::slice::from_raw_parts(self.data, self.len as usize);
            std::str::from_utf8_unchecked(slice)
        }
    }
}

impl From<String> for FfiString {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

/// Free a [`ByteBuffer`] returned by this library.
///
/// Single-call: see the ownership rules on the [module docs](self). Passing the
/// empty buffer (null pointer) is a no-op. Panics during free are swallowed so
/// nothing unwinds across the ABI.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_byte_buffer_free(buffer: ByteBuffer) {
    // SAFETY: caller upholds the single-call ownership contract.
    let _ = std::panic::catch_unwind(|| unsafe { buffer.into_owned() });
}

/// Free an [`FfiString`] returned by this library.
///
/// Single-call: see the ownership rules on the [module docs](self). Passing the
/// empty string (null pointer) is a no-op. Panics during free are swallowed so
/// nothing unwinds across the ABI.
#[ffi_function]
#[no_mangle]
pub extern "C" fn ndoc_string_free(s: FfiString) {
    // SAFETY: caller upholds the single-call ownership contract.
    // (Parameter named `s`, not `string`, so the generated C# is not a keyword.)
    let _ = std::panic::catch_unwind(|| unsafe { s.into_owned() });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_buffer_round_trips_and_frees() {
        let buffer = ByteBuffer::from(vec![1u8, 2, 3, 4]);
        assert_eq!(buffer.len, 4);
        assert!(!buffer.data.is_null());
        // SAFETY: reading the bytes we just allocated.
        let slice = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len as usize) };
        assert_eq!(slice, &[1, 2, 3, 4]);
        ndoc_byte_buffer_free(buffer);
    }

    #[test]
    fn empty_byte_buffer_is_null_and_free_is_noop() {
        let buffer = ByteBuffer::from(Vec::<u8>::new());
        assert!(buffer.data.is_null());
        assert_eq!(buffer.len, 0);
        ndoc_byte_buffer_free(buffer);
    }

    #[test]
    fn ffi_string_round_trips_and_frees() {
        let s = FfiString::from("héllo".to_string());
        assert_eq!(s.as_str(), "héllo");
        assert_eq!(s.len, "héllo".len() as u64);
        ndoc_string_free(s);
    }

    #[test]
    fn empty_ffi_string_is_null_and_free_is_noop() {
        let s = FfiString::from(String::new());
        assert!(s.data.is_null());
        assert_eq!(s.as_str(), "");
        ndoc_string_free(s);
    }
}
