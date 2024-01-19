use std::collections::HashMap;
use bincode::{serialize_into, deserialize_from};
use serde::{Serialize, Deserialize};
use std::io::Cursor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use bincode::{serialize};
use erc20_macro::generate_abi;
use std::ffi::{CString, CStr};

#[derive(Serialize, Deserialize,Clone)]
pub struct ERC721Token {
    pub name: String,
    pub symbol: String,
    pub owner_of: Vec<String>,        // Vector to store owner addresses
    pub token_to_ipfs: Vec<String>,    // Vector to store IPFS hashes
    //pub owner_of: HashMap<u64, String>,  // tokenId -> owner address
    //pub token_to_ipfs: HashMap<u64, String>,  // tokenId -> IPFS hash
    pub token_id: u64,
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
    token_details_buffer: Vec<u8>, // Use a dynamic Vec<u8> for the buffer
}

static mut GLOBAL_STATE: GlobalState = GlobalState {
    token_ptr: None,
    token_details_buffer: Vec::new(), // Initialize with an empty Vec
};

#[generate_abi]
impl ERC721Token {

    fn deserialize_from_memory(buffer: *const u8, len: usize) -> Result<ERC721Token, Box<dyn std::error::Error>> {
        let reader = unsafe { Cursor::new(std::slice::from_raw_parts(buffer, len)) };
        let token = deserialize_from(reader)?;
        Ok(token)
    }

    #[no_mangle]
    pub fn to_memory(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let serialized_data = serialize(self)?;
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
            owner_of: Vec::new(),
            token_to_ipfs: Vec::new(),
            token_id: 0,
        };

        // Box and convert the token into a raw pointer
        let token_ptr = Box::into_raw(Box::new(token));
        unsafe {
            GLOBAL_STATE.token_ptr = Some(token_ptr);
        }
    }

    #[no_mangle]
    pub extern "C" fn mint(
        owner_ptr: *const u8,
        owner_len: usize,
        ipfs_hash_ptr: *const u8,
        ipfs_hash_len: usize,
    ) -> u32 {
        let token = match unsafe { GLOBAL_STATE.token_ptr } {
            Some(ptr) => unsafe { &mut *ptr },
            None => {
                println!("Mint: Failed to mint, uninitialized TOKEN_PTR.");
                return u32::MAX; // Error value indicating uninitialized TOKEN_PTR.
            }
        };
        // Convert raw pointers to Rust strings using from_utf8_lossy
        let owner_slice = unsafe { std::slice::from_raw_parts(owner_ptr, owner_len) };
        let owner_str = String::from_utf8_lossy(owner_slice).to_string();
        println!("Mint: Owner Address String Length: {}", owner_str.len());
        if owner_str.is_empty() {
            println!("Mint: Error - Attempted to mint with an empty owner address.");
            return u32::MAX; // Indicate an error condition.
        }
        let ipfs_hash_slice = unsafe { std::slice::from_raw_parts(ipfs_hash_ptr, ipfs_hash_len) };
        let ipfs_hash_str = String::from_utf8_lossy(ipfs_hash_slice).to_string();
    
        let token_id = token.token_id;
        token.owner_of.push(owner_str); // Add the owner to the vector
        token.token_to_ipfs.push(ipfs_hash_str); // Add the IPFS hash to the vector
        token.token_id += 1;
        
        println!("Mint: Successfully minted token with ID: {}", token_id);
    
        token_id as u32
    }
    
    
    #[no_mangle]
    pub fn get_token_or_default() -> ERC721Token {
        unsafe {
            if let Some(token_ptr) = GLOBAL_STATE.token_ptr {
                // Return a clone of the owned ERC721Token
                (*token_ptr).clone()
            } else {
                // Return a default instance when the token is not initialized
                ERC721Token {
                    name: String::new(),
                    symbol: String::new(),
                    owner_of: Vec::new(),
                    token_to_ipfs: Vec::new(),
                    token_id: 0,
                }
            }
        }
    }
    #[no_mangle]
    pub extern "C" fn get_owner_len(token_id: i32) -> i64 {
        // Retrieve the pointer to the ERC721Token from GLOBAL_STATE.
        let token_ptr = unsafe { GLOBAL_STATE.token_ptr };
    
        // If the token pointer is None, the contract is not initialized.
        if token_ptr.is_none() {
            return -1; // Indicate that the token is not initialized.
        }
    
        // Safety: We have already checked that the pointer is not None.
        let token = unsafe { &*token_ptr.unwrap() };
    
        // Check if the token_id is non-negative and within the bounds of the owner_of vector.
        if token_id >= 0 && (token_id as usize) < token.owner_of.len() {
            // Use token_id as an index directly thanks to zero-based indexing.
            token.owner_of[token_id as usize].len() as i64
        } else {
            -1 // Token ID is out of bounds.
        }
    }
    
    #[no_mangle]
    pub extern "C" fn get_owner_ptr(token_id: i32) -> *const i8 {
        // Using get_token_or_default is risky here; we should use GLOBAL_STATE directly.
        let token_ptr = unsafe { GLOBAL_STATE.token_ptr };
        if token_ptr.is_none() {
            return std::ptr::null(); // Token not initialized.
        }
        
        let token = unsafe { &*token_ptr.unwrap() };
        if token_id >= 0 && (token_id as usize) < token.owner_of.len() {
            let owner = &token.owner_of[token_id as usize];
            owner.as_ptr() as *const i8 // Get pointer to the existing string data.
        } else {
            std::ptr::null() // Token ID out of bounds.
        }
    }
    #[no_mangle]
    pub extern "C" fn get_ipfs_len(token_id: i32) -> i32 {
        let token_ptr = unsafe { GLOBAL_STATE.token_ptr };
        if let Some(ptr) = token_ptr {
            let token = unsafe { &*ptr };

            if token_id >= 0 && (token_id as usize) < token.token_to_ipfs.len() {
                let ipfs_hash = &token.token_to_ipfs[token_id as usize];
                ipfs_hash.len() as i32
            } else {
                -1 // Token ID out of bounds or not found.
            }
        } else {
            -1 // Token not initialized.
        }
    }
    #[no_mangle]
    pub extern "C" fn get_ipfs_ptr(token_id: i32) -> *const i8 {
        let token_ptr = unsafe { GLOBAL_STATE.token_ptr };
        if let Some(ptr) = token_ptr {
            let token = unsafe { &*ptr };

            if token_id >= 0 && (token_id as usize) < token.token_to_ipfs.len() {
                let ipfs_hash = &token.token_to_ipfs[token_id as usize];
                ipfs_hash.as_ptr() as *const i8
            } else {
                std::ptr::null() // Token ID out of bounds or not found.
            }
        } else {
            std::ptr::null() // Token not initialized.
        }
    }

    pub fn encode_token_details(details: &TokenDetails) -> Vec<u8> {
        let encoded_bytes = serialize(details).expect("Encoding failed");
        encoded_bytes
    }


    #[no_mangle]
    pub extern "C" fn transfer(&mut self, 
        from: String, to: String, token_id: i32
        token_id: i32) -> Result<(), &'static str >{
        // Convert raw pointers to Rust strings
        let from_ptr = from.as_ptr();
        let from_len = from.len();
    
        let to_ptr = to.as_ptr();
        let to_len = to.len();
        let from_slice = unsafe { std::slice::from_raw_parts(from_ptr, from_len) };
        let from_str = std::str::from_utf8(from_slice).expect("Failed to convert from");
    
        let to_slice = unsafe { std::slice::from_raw_parts(to_ptr, to_len) };
        let to_str = std::str::from_utf8(to_slice).expect("Failed to convert to");
    
        // Convert the i32 token_id to usize
        let token_id_usize = token_id as usize;
    
        // Check if the token_id is within bounds
        if token_id_usize < self.owner_of.len() {
            let current_owner = &self.owner_of[token_id_usize];
    
            if current_owner == from_str {
                self.owner_of[token_id_usize] = to_str.to_string();
                Ok(())
            } else {
                Err("Transfer not allowed")
            }
        } else {
            Err("Token not found")
        }
    }
    #[no_mangle]
    pub extern "C" fn read_name(&self, buffer: *mut u8, buffer_len: usize) -> isize {
        println!("Read Name: Reading name into buffer...");
        let bytes = self.name.as_bytes();
    
        if bytes.len() > buffer_len {
            println!("Read Name: Buffer too small to hold the name");
            return -1;  // Buffer too small
        }
        
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len());
        }
    
        println!("Read Name: Successfully read the name into the buffer");
    
        bytes.len() as isize
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
