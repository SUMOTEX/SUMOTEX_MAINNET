use std::collections::HashMap;
use bincode::{serialize_into, deserialize_from};
use serde::{Serialize, Deserialize};
use std::io::Cursor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use bincode::{serialize};

#[derive(Serialize, Deserialize)]
pub struct ERC721Token {
    pub name: String,
    pub symbol: String,
    pub owner_of: HashMap<i32, String>,
    pub token_to_ipfs: HashMap<i32, String>,
    pub token_id: i32,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct TokenDetails {
    pub owner: String,
    pub ipfs_link: String,
}

fn extract_string_from_wasm_memory(ptr: *mut u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}

pub struct GlobalState {
    token_ptr: Option<*mut ERC721Token>,
    global_instance: Option<ERC721Token>,
    token_details_buffer: [u8; 1024],
}

static mut GLOBAL_STATE: GlobalState = GlobalState {
    token_ptr: None,
    global_instance: None,
    token_details_buffer: [0u8; 1024],
};


impl ERC721Token {
    fn deserialize_from_memory(buffer: *const u8, len: usize) -> Result<ERC721Token, Box<dyn std::error::Error>> {
        let reader = unsafe { Cursor::new(std::slice::from_raw_parts(buffer, len)) };
        let token = deserialize_from(reader)?;
        Ok(token)
    }
    #[no_mangle]
    fn serialize_to_memory(token: &ERC721Token, buffer: *mut u8) -> Result<usize, Box<dyn std::error::Error>> {
        let mut writer = unsafe {
            // Assuming the buffer is large enough to hold the serialized data
            Cursor::new(std::slice::from_raw_parts_mut(buffer, 1024))
        };
        serialize_into(&mut writer, token)?;
        Ok(writer.position() as usize)
    }
    #[no_mangle]
    fn to_memory(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut serialized_data = Vec::new();
        serialize_into(&mut serialized_data, self)?;
        Ok(serialized_data)
    }

    #[no_mangle]
    pub extern "C" fn initialize(
        name_ptr: *mut u8,  
        name_len: usize, 
        symbol_ptr: *mut u8, 
        symbol_len: usize,
    ) {
        // Extract name and symbol from wasm memory
        let name = extract_string_from_wasm_memory(name_ptr, name_len);
        let symbol = extract_string_from_wasm_memory(symbol_ptr, symbol_len);
        
        let token = ERC721Token {
            name: name,
            symbol: symbol,
            owner_of: HashMap::new(),
            token_to_ipfs: HashMap::new(),
            token_id: 1,
        };
    
        // Box and convert the token into a raw pointer
        let token_ptr =  Box::into_raw(Box::new(token));
        unsafe {
            GLOBAL_STATE.token_ptr = Some(token_ptr);
        }
    }
    
    #[no_mangle]
    pub extern "C" fn store_token_in_memory(token_ptr: *mut ERC721Token, buffer: *mut u8, buffer_len: usize) -> isize {
        let token = match unsafe { GLOBAL_STATE.token_ptr } {
            Some(ptr) => unsafe { &*ptr },
            None => return -1, // Error value indicating uninitialized TOKEN_PTR.
        };
        
        let mut writer = unsafe {
            Cursor::new(std::slice::from_raw_parts_mut(buffer, buffer_len))
        };
        if let Err(_) = serialize_into(&mut writer, token) {
            return -1;  // Error during serialization
        }
        writer.position() as isize
    }
    #[no_mangle]
    pub extern "C" fn allocate(size: usize) -> *mut u8 {
        let mut buffer = Vec::with_capacity(size);
        let pointer = buffer.as_mut_ptr();
        std::mem::forget(buffer);  // Ensure Vec doesn't deallocate memory when dropped
        pointer
    }
    #[no_mangle]
    pub extern "C" fn deallocate(pointer: *mut u8, size: usize) {
        unsafe {
            let _ = Vec::from_raw_parts(pointer, 0, size);
            // When the Vec<_> is dropped, the associated memory will be deallocated.
        }
    }

    #[no_mangle]
    pub extern "C" fn get_owner_len_by_token_id(token_id: i32) -> u32 {
        let token = match unsafe { GLOBAL_STATE.token_ptr } {
            Some(ptr) => unsafe { &*ptr },
            None => return 0, // Return 0 indicating uninitialized TOKEN_PTR.
        };
    
        if token_id <= 0 {
            return 0; // Return 0 indicating an invalid token ID.
        }
    
        if let Some(owner) = token.owner_of.get(&token_id) {
            return owner.len() as u32; // Return the length of the owner string
        } else {
            return 0; // Owner not found or an error occurred
        }
    }
    
    #[no_mangle]
    pub extern "C" fn get_owner_ptr_by_token_id(token_id: i32) -> u32 {
        let token = match unsafe { GLOBAL_STATE.token_ptr } {
            Some(ptr) => unsafe { &*ptr },
            None => return 0, // Return -1 indicating uninitialized TOKEN_PTR.
        };
    
        if token_id <= 0 {
            return 0; // Invalid token ID
        }
    
        if let Some(owner) = token.owner_of.get(&token_id)  {
            let owner_bytes = owner.as_bytes();
            return owner_bytes.as_ptr() as u32; // Return the pointer to the owner string
        } else {
            0 // Owner not found or an error occurred
        }
    }
    
    #[no_mangle]
    fn write_string_to_memory(s: &str) -> i32 {
        let size = s.len();
        let ptr = Self::allocate(size) as *mut u8;
    
        unsafe {
            std::ptr::copy_nonoverlapping(s.as_ptr(), ptr, size);
        }
    
        ptr as i32
    }
    #[no_mangle]
    pub extern "C" fn load_token_from_memory(buffer: *const u8, len: usize) -> *mut ERC721Token {
        let token = Self::deserialize_from_memory(buffer, len).expect("Failed to deserialize");
        Box::into_raw(Box::new(token))
    }

    #[no_mangle]
    pub extern "C" fn mint(owner_ptr: *const u8, owner_len: usize, ipfs_hash_ptr: *const u8, ipfs_hash_len: usize) -> i32 {
        let token = match unsafe {  GLOBAL_STATE.token_ptr } {
            Some(ptr) => unsafe { &mut *ptr },
            None => return i32::MAX, // Error value indicating uninitialized TOKEN_PTR.
        };
        
        let owner_slice = unsafe { std::slice::from_raw_parts(owner_ptr, owner_len) };
        let owner_str = match std::str::from_utf8(owner_slice) {
            Ok(s) => s,
            Err(_) => return -1, // Error value indicating invalid owner string.
        };
        
        let ipfs_hash_slice = unsafe { std::slice::from_raw_parts(ipfs_hash_ptr, ipfs_hash_len) };
        let ipfs_hash_str = match std::str::from_utf8(ipfs_hash_slice) {
            Ok(s) => s,
            Err(_) => return -1, // Error value indicating invalid IPFS hash string.
        };
        let token_id = token.token_id;
    
        // Check if the token ID already exists in the HashMap and handle accordingly.
        if token.owner_of.contains_key(&token_id) || token.token_to_ipfs.contains_key(&token_id) {
            return -1; // An error value indicating duplicate token ID.
        }

        // Insert the owner and IPFS hash into the HashMap.
        token.owner_of.insert(token_id, owner_str.to_string());
        token.token_to_ipfs.insert(token_id, ipfs_hash_str.to_string());
        
        token.token_id += 1;
        token_id as i32
    }

    #[no_mangle]
    pub extern "C" fn verify_minted_data(token_id: i32, owner_ptr: *const u8, owner_len: usize, ipfs_hash_ptr: *const u8, ipfs_hash_len: usize) -> bool {
        let token = match unsafe {  GLOBAL_STATE.token_ptr } {
            Some(ptr) => unsafe { &*ptr },
            None => return false,  // Error value indicating uninitialized TOKEN_PTR.
        };
    
        // Convert pointers to slices
        let owner_slice = unsafe { std::slice::from_raw_parts(owner_ptr, owner_len) };
        let ipfs_hash_slice = unsafe { std::slice::from_raw_parts(ipfs_hash_ptr, ipfs_hash_len) };
    
        // Validate against HashMaps
        match token.owner_of.get(&token_id) {
            Some(stored_owner) => {
                if stored_owner.as_bytes() != owner_slice {
                    return false;  // Owner data mismatch
                }
            },
            None => return false,  // Token ID not found
        }
    
        match token.token_to_ipfs.get(&token_id) {
            Some(stored_ipfs_hash) => {
                if stored_ipfs_hash.as_bytes() != ipfs_hash_slice {
                    return false;  // IPFS hash data mismatch
                }
            },
            None => return false,  // Token ID not found
        }
    
        true  // All checks passed
    }
    
    #[no_mangle]
    pub extern "C" fn read_token(token_id: i32) -> i64 {
        let token = match unsafe {  GLOBAL_STATE.token_ptr } {
            Some(ptr) => unsafe { &*ptr },
            None => return -1, // Return -1 indicating uninitialized TOKEN_PTR.
        };
    
        if let Some(details) = token.get_token_details(token_id) {
            // Convert TokenDetails into u8 bytes
            let encoded_value = Self::encode_token_details(&details);
            
            if encoded_value.is_empty() {
                return -2; // Return -2 indicating an issue during encoding.
            }
    
            let length = encoded_value.len();
            // Pack the usize length and the first u8 value into an i64
            let packed_result = ((length as i64) << 32) | (encoded_value[0] as i64);
            packed_result
        } else {
            // If details are not found, return -1
            -1
        }
    }
    
    pub fn encode_token_details(details: &TokenDetails) -> Vec<u8> {
        let encoded_bytes = serialize(details).expect("Encoding failed");
        encoded_bytes
    }

    fn get_token_details(&self, token_id: i32) -> Option<TokenDetails> {
        let owner = self.owner_of.get(&token_id)?.clone();
        let ipfs_link = self.token_to_ipfs.get(&token_id)?.clone();
        Some(TokenDetails { owner, ipfs_link })
    }
    #[no_mangle]
    pub extern "C" fn transfer(&mut self, 
        from_ptr: *const u8, 
        from_len: usize, 
        to_ptr: *const u8, 
        to_len: usize, 
        token_id: i32) -> Result<(), &'static str> {
        // Convert raw pointers to Rust strings
        let from_slice = unsafe { std::slice::from_raw_parts(from_ptr, from_len) };
        let from_str = std::str::from_utf8(from_slice).expect("Failed to convert from");

        let to_slice = unsafe { std::slice::from_raw_parts(to_ptr, to_len) };
        let to_str = std::str::from_utf8(to_slice).expect("Failed to convert to");

        match self.owner_of.get(&token_id) {
        Some(current_owner) if *current_owner == from_str => {
        self.owner_of.insert(token_id, to_str.to_string());
        Ok(())
        },
        _ => Err("Transfer not allowed"),
        }
    }

    #[no_mangle]
    pub extern "C" fn read_name(&self, buffer: *mut u8, buffer_len: usize) -> isize {
        self.string_to_buffer(&self.name, buffer, buffer_len)
    }

    #[no_mangle]
    pub extern "C" fn read_symbol(&self, buffer: *mut u8, buffer_len: usize) -> isize {
        self.string_to_buffer(&self.symbol, buffer, buffer_len)
    }
    fn string_to_buffer(&self, source: &str, buffer: *mut u8, buffer_len: usize) -> isize {
        let bytes = source.as_bytes();
        if bytes.len() > buffer_len {
            return -1;  // Buffer too small
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len());
        }
        bytes.len() as isize
    }
    #[no_mangle]
    pub extern "C" fn total_tokens() -> i64 {
        unsafe {
            if let Some(token_ptr) =  GLOBAL_STATE.token_ptr {
                ((*token_ptr).token_id - 1) as i64
            } else {
                // Return an error value or handle the case where the token is not initialized
                -1
            }
        }
    }
}
