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
    pub next_token_id: i32,
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

static mut TOKEN_PTR: Option<*mut ERC721Token> = None;
static mut GLOBAL_INSTANCE: Option<ERC721Token> = None;

impl ERC721Token {
    fn deserialize_from_memory(buffer: *const u8, len: usize) -> Result<ERC721Token, Box<dyn std::error::Error>> {
        let reader = unsafe { Cursor::new(std::slice::from_raw_parts(buffer, len)) };
        let token = deserialize_from(reader)?;
        Ok(token)
    }

    fn serialize_to_memory(token: &ERC721Token, buffer: *mut u8) -> Result<usize, Box<dyn std::error::Error>> {
        let mut writer = unsafe {
            // Assuming the buffer is large enough to hold the serialized data
            Cursor::new(std::slice::from_raw_parts_mut(buffer, 1024))
        };
        serialize_into(&mut writer, token)?;
        Ok(writer.position() as usize)
    }
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
            next_token_id: 1,
        };
    
        // Box and convert the token into a raw pointer
        let token_ptr =  Box::into_raw(Box::new(token));
        unsafe {
            TOKEN_PTR = Some(token_ptr);
            //GLOBAL_INSTANCE = Some(*Box::from_raw(token_ptr));
        }
    }

    #[no_mangle]
    pub extern "C" fn store_token_in_memory(token_ptr: *mut ERC721Token, buffer: *mut u8, buffer_len: usize) -> isize {
        let token = unsafe { &*token_ptr };
        let mut writer = unsafe {
            Cursor::new(std::slice::from_raw_parts_mut(buffer, buffer_len))
        };
        if let Err(_) = serialize_into(&mut writer, token) {
            return -1;  // Error during serialization
        }
        writer.position() as isize
    }


    #[no_mangle]
    pub extern "C" fn load_token_from_memory(buffer: *const u8, len: usize) -> *mut ERC721Token {
        let token = Self::deserialize_from_memory(buffer, len).expect("Failed to deserialize");
        Box::into_raw(Box::new(token))
    }
    #[no_mangle]
    pub extern "C" fn mint(owner_ptr: *const u8, owner_len: usize, ipfs_hash_ptr: *const u8, ipfs_hash_len: usize) -> i32 {
        let token = match unsafe { TOKEN_PTR } {
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
    
        let token_id = token.next_token_id;
    
        // Check if the token ID already exists in the HashMap and handle accordingly.
        if token.owner_of.contains_key(&token_id) || token.token_to_ipfs.contains_key(&token_id) {
            return -1; // An error value indicating duplicate token ID.
        }
    
        // Insert the owner and IPFS hash into the HashMap.
        token.owner_of.insert(token_id, owner_str.to_string());
        token.token_to_ipfs.insert(token_id, ipfs_hash_str.to_string());
        
        token.next_token_id += 1;
    
        token_id as i32
    }
    
    
    // #[no_mangle]
    // pub extern "C" fn mint(owner_ptr: *const u8, owner_len: usize, ipfs_hash_ptr: *const u8, ipfs_hash_len: usize) -> i32 {
    //     let token = unsafe {
    //         if let Some(ptr) = TOKEN_PTR {
    //             &mut *ptr
    //         } else {
    //             return i32::MAX; // Error value indicating uninitialized TOKEN_PTR.
    //         }
    //     };
        
    //     let owner_slice = unsafe { std::slice::from_raw_parts(owner_ptr, owner_len) };
    //     let owner_str = std::str::from_utf8(owner_slice).expect("Failed to convert owner");
        
    //     let ipfs_hash_slice = unsafe { std::slice::from_raw_parts(ipfs_hash_ptr, ipfs_hash_len) };
    //     let ipfs_hash_str = std::str::from_utf8(ipfs_hash_slice).expect("Failed to convert ipfs_hash");
    
    //     let token_id = token.next_token_id;
    //     token.owner_of.insert(token_id, owner_str.to_string());
    //     token.token_to_ipfs.insert(token_id, ipfs_hash_str.to_string());
    //     token.next_token_id += 1;
    
    //     token_id as i32
    // }
    #[no_mangle]
    pub extern "C" fn read_token(token_id: i32) -> i64 {
        let token = match unsafe { TOKEN_PTR } {
            Some(ptr) => unsafe { &*ptr },
            None => return -1, // Return -1 indicating uninitialized TOKEN_PTR.
        };
    
        if let Some(details) = token.get_token_details(token_id) {
            // Convert TokenDetails into u8 bytes
            let encoded_value = Self::encode_token_details(&details);
            let length = encoded_value.len();
    
            // Pack the u8 and usize values into an i64
            let packed_result = ((length as i64) << 32) | (encoded_value[0] as i64);
            packed_result
        } else {
            // If details are not found, return -1
            -1
        }
    }
    
    pub fn convert_details_to_i64(details: &TokenDetails) -> i64 {
        // Implement your logic to convert TokenDetails into an i64 value here
        // For example, using the length of the owner string as the value
        details.owner.len() as i64
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
    pub extern "C" fn owner_of(&self, token_id: i32) -> Option<String> {
        self.owner_of.get(&token_id).cloned()
        
    }

    #[no_mangle]
    pub extern "C" fn get_ipfs_link(&self, token_id: i32) -> Option<String> {
        self.token_to_ipfs.get(&token_id).cloned()
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
            if let Some(token_ptr) = TOKEN_PTR {
                ((*token_ptr).next_token_id - 1) as i64
            } else {
                // Return an error value or handle the case where the token is not initialized
                -1
            }
        }
    }
}
