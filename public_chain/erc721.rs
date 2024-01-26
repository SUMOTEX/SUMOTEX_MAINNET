use std::collections::HashMap;
use bincode::{serialize_into, deserialize_from};
use serde::{Serialize, Deserialize};
use std::io::Cursor;

#[derive(Serialize, Deserialize)]
pub struct ERC721Token {
    pub name: String,
    pub symbol: String,
    pub owner_of: HashMap<i32, String>,
    pub owner_id:HashMap<i32, String>,
    pub token_to_ipfs: HashMap<i32, String>,
    pub next_token_id: i32,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct TokenDetails {
    pub owner_id:String,
    pub owner: String,
    pub ipfs_link: String,
}
fn extract_string_from_wasm_memory(ptr: *mut u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}
fn token_details_to_bytes(details: &TokenDetails) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Ok(bincode::serialize(details)?)
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
            owner_id:HashMap::new(),
            owner_of: HashMap::new(),
            token_to_ipfs: HashMap::new(),
            next_token_id: 1,
        };
    
        // Box and convert the token into a raw pointer
        let token_ptr =  Box::into_raw(Box::new(token));
        unsafe {
            TOKEN_PTR = Some(token_ptr);
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
    pub extern "C" fn mint(
         owner_ptr: *const u8,
         owner_len: usize,
         owner_id_ptr: *const u8,
         owner_id_len: usize, 
         ipfs_hash_ptr: *const u8,
         ipfs_hash_len: usize) -> i32 {
        let token = unsafe {
            if let Some(ptr) = TOKEN_PTR {
                &mut *ptr
            } else {
                return i32::MAX; // Error value indicating uninitialized TOKEN_PTR.
            }
        };

        let owner_slice = unsafe { std::slice::from_raw_parts(owner_ptr, owner_len) };
        let owner_str = std::str::from_utf8(owner_slice).expect("Failed to convert owner");

        let owner_id_slice = unsafe { std::slice::from_raw_parts(owner_id_ptr, owner_id_len) };
        let owner_id_str = std::str::from_utf8(owner_id_slice).expect("Failed to convert owner id");
        
        let ipfs_hash_slice = unsafe { std::slice::from_raw_parts(ipfs_hash_ptr, ipfs_hash_len) };
        let ipfs_hash_str = std::str::from_utf8(ipfs_hash_slice).expect("Failed to convert ipfs_hash");
    
        let token_id = token.next_token_id;
        token.owner_id.insert(token_id,owner_id_str.to_string());
        token.owner_of.insert(token_id, owner_str.to_string());
        token.token_to_ipfs.insert(token_id, ipfs_hash_str.to_string());
        token.next_token_id += 1;
    
        token_id as i32
    }
    pub fn convert_details_to_f64(details: &TokenDetails) -> f64 {
        // Implement your logic to convert TokenDetails into an f64 here
        // For example, you can calculate the length of the owner's name and use it as an f64.
        let owner_name_length = details.owner.len() as f64;
        
        // You can add more logic here based on your specific requirements.
        
        // Return the calculated value as an f64.
        owner_name_length
    }
    #[no_mangle]
    pub extern "C" fn read_token(token_id: i32) -> f64 {
        let instance = unsafe {
            GLOBAL_INSTANCE.as_ref().expect("Instance not initialized")
        };
        if let Some(details) = instance.get_token_details(token_id) {
            // Assuming you have some logic to convert TokenDetails into an f64
            let result_as_f64 = Self::convert_details_to_f64(&details);
            return result_as_f64;
        }
        0.0 // Return a default value if details are not found
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
