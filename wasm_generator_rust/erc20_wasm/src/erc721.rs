use std::collections::HashMap;
use bincode::{serialize_into, deserialize_from};
use serde::{Serialize, Deserialize};
use std::io::Cursor;

#[derive(Serialize, Deserialize)]
pub struct ERC721Token {
    pub name: String,
    pub symbol: String,
    pub owner_of: HashMap<u64, String>,  // tokenId -> owner address
    pub token_to_ipfs: HashMap<u64, String>,  // tokenId -> IPFS hash
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

        let serialized_data = token.to_memory().expect("Failed to serialize");

        let buf = unsafe {
            let layout = std::alloc::Layout::from_size_align(serialized_data.len(), 1).unwrap();
            std::alloc::alloc(layout)
        };
        
        unsafe {
            std::ptr::copy_nonoverlapping(serialized_data.as_ptr(), buf, serialized_data.len());
        }
        //buf
        //Box::into_raw(Box::new(token))
    }

    #[no_mangle]
    pub extern "C" fn mint(&mut self, owner: String, ipfs_hash: String) -> u64 {
        let token_id = self.next_token_id;
        self.owner_of.insert(token_id, owner.clone());
        self.token_to_ipfs.insert(token_id, ipfs_hash);
        self.next_token_id += 1;
        token_id
    }

    #[no_mangle]
    pub extern "C" fn transfer(&mut self, from: String, to: String, token_id: u64) -> Result<(), &'static str> {
        match self.owner_of.get(&token_id) {
            Some(current_owner) if *current_owner == from => {
                self.owner_of.insert(token_id, to);
                Ok(())
            },
            _ => Err("Transfer not allowed"),
        }
    }

    #[no_mangle]
    pub extern "C" fn owner_of(&self, token_id: u64) -> Option<String> {
        self.owner_of.get(&token_id).cloned()
    }

    #[no_mangle]
    pub extern "C" fn get_ipfs_link(&self, token_id: u64) -> Option<String> {
        self.token_to_ipfs.get(&token_id).cloned()
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
    pub extern "C" fn total_tokens(&self) -> u64 {
        self.next_token_id - 1
    }
}

// Rest of the utility functions and other methods...
