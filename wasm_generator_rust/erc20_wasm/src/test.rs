use std::slice;

#[no_mangle]
pub extern "C" fn write_string_to_memory(data: *const u8, offset: i32, len: i32) {
    // Assuming the default WebAssembly memory is used
    let offset_ptr = offset as *mut u8;
    let memory = unsafe { slice::from_raw_parts_mut(offset_ptr, len as usize) };
    let data_slice = unsafe { slice::from_raw_parts(data, len as usize) };
    
    // Copy the data from the input buffer to the WebAssembly memory
    memory.copy_from_slice(data_slice);
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
