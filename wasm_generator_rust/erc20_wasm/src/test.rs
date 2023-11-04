use std::slice;
use std::mem;
use std::ffi::{CString, CStr};
use std::os::raw::c_void;

#[no_mangle]
pub extern "C" fn alloc() -> *mut c_void {
    let mut buf = Vec::with_capacity(1024);
    let ptr = buf.as_mut_ptr();

    mem::forget(buf);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn dealloc(ptr: *mut c_void) {
    let _ = Vec::from_raw_parts(ptr, 0, 1024);
}

#[no_mangle]
pub unsafe extern "C" fn greet(ptr: *mut u8) {
    let str_content = CStr::from_ptr(ptr as *const i8).to_str().unwrap();
    let mut string_content = String::from("Hello, ");

    string_content.push_str(str_content);
    string_content.push_str("!");

    let c_headers = CString::new(string_content).unwrap();

    let bytes = c_headers.as_bytes_with_nul();

    let header_bytes = std::slice::from_raw_parts_mut(ptr, 1024);
    header_bytes[..bytes.len()].copy_from_slice(bytes);
}

#[no_mangle]
pub extern "C" fn get_string_offset() -> i32 {
    // Prepare a string
    let string = "Hello, WebAssembly!";
    
    // Calculate the offset of the string
    string.as_ptr() as i32
}

#[no_mangle]
pub extern "C" fn get_string_length() -> i32 {
    // Prepare a string
    let string = "Hello, WebAssembly!";
    
    // Get the length of the string
    string.len() as i32
}
