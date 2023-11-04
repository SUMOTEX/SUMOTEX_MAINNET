use std::collections::HashMap;
use bincode::{serialize_into, deserialize_from};
use serde::{Serialize, Deserialize};
use std::io::Cursor;

#[derive(Serialize, Deserialize)]
pub struct ERC721Token {
    pub name: String,
    pub symbol: String,
    pub owners: Vec<String>,        // Vector to store owner addresses
    pub ipfs_hashes: Vec<String>,    // Vector to store IPFS hashes
    //pub owner_of: HashMap<u64, String>,  // tokenId -> owner address
    //pub token_to_ipfs: HashMap<u64, String>,  // tokenId -> IPFS hash
    pub next_token_id: u64,
}
fn extract_string_from_wasm_memory(ptr: *mut u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}
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
    ) -> *mut ERC721Token {
        // Extract name and symbol from wasm memory
        let name = extract_string_from_wasm_memory(name_ptr, name_len);
        let symbol = extract_string_from_wasm_memory(symbol_ptr, symbol_len);
        
        let token = ERC721Token {
            name: name,
            symbol: symbol,
            owner_of: Vec::new(),
            token_to_ipfs: Vec::new(),
            next_token_id: 1,
        };
    
        // Box and convert the token into a raw pointer
        Box::into_raw(Box::new(token))
    }
    #[no_mangle]
    pub extern "C" fn mint(&mut self, owner_ptr: *const u8, owner_len: usize, ipfs_hash_ptr: *const u8, ipfs_hash_len: usize) -> u32 {
        // Convert raw pointers to Rust strings using from_utf8_lossy
        let owner_slice = unsafe { std::slice::from_raw_parts(owner_ptr, owner_len) };
        let owner_str = String::from_utf8_lossy(owner_slice).to_string();
        
        let ipfs_hash_slice = unsafe { std::slice::from_raw_parts(ipfs_hash_ptr, ipfs_hash_len) };
        let ipfs_hash_str = String::from_utf8_lossy(ipfs_hash_slice).to_string();
        
        let token_id = self.next_token_id;
        self.owners.push(owner_str);
        self.ipfs_hashes.push(ipfs_hash_str);
        self.next_token_id += 1;
        token_id as u32
    }
    // #[no_mangle]
    // pub extern "C" fn mint(&mut self, owner_ptr: *const u8, owner_len: usize, ipfs_hash_ptr: *const u8, ipfs_hash_len: usize) -> u32 {
    //     // Convert raw pointers to Rust strings using from_utf8_lossy
    //     let owner_slice = unsafe { std::slice::from_raw_parts(owner_ptr, owner_len) };
    //     let owner_str = String::from_utf8_lossy(owner_slice);
        
    //     let ipfs_hash_slice = unsafe { std::slice::from_raw_parts(ipfs_hash_ptr, ipfs_hash_len) };
    //     let ipfs_hash_str = String::from_utf8_lossy(ipfs_hash_slice);
        
    //     let token_id = self.next_token_id;
    //     self.owner_of.insert(token_id, owner_str.to_string());
    //     self.token_to_ipfs.insert(token_id, ipfs_hash_str.to_string());
    //     self.next_token_id += 1;
    //     token_id as u32
    // }
    
    #[no_mangle]
    pub extern "C" fn transfer(&mut self, 
        from_ptr: *const u8, 
        from_len: usize, 
        to_ptr: *const u8, 
        to_len: usize, 
        token_id: u64) -> Result<(), &'static str> {
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
    pub extern "C" fn read_name(&self) -> String {
        self.name.clone()
    }

    #[no_mangle]
    pub extern "C" fn read_symbol(&self) -> String {
        self.symbol.clone()
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
