use std::collections::HashMap;
use bincode::{serialize_into, deserialize_from};
use std::io::Cursor;
use erc20_macro::generate_abi;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct ERC20Token {
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: u64,
    balances: HashMap<String, u64>,
    allowed: HashMap<String, HashMap<String, u64>>,
}
    
fn extract_string_from_wasm_memory(ptr: *mut u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}

#[generate_abi]
impl ERC20Token {
    fn deserialize_from_memory(buffer: *const u8, len: usize) -> Result<ERC20Token, Box<dyn std::error::Error>> {
        let reader = unsafe { Cursor::new(std::slice::from_raw_parts(buffer, len)) };
        let token = deserialize_from(reader)?;
        Ok(token)
    }
    fn serialize_to_memory(token: &ERC20Token, buffer: *mut u8) -> Result<usize, Box<dyn std::error::Error>> {
        let mut writer = unsafe {
            // Assuming the buffer is large enough to hold the serialized data
            Cursor::new(std::slice::from_raw_parts_mut(buffer, 1024)) // replace 1024 with suitable max size
        };
        serialize_into(&mut writer, &token)?;
        Ok(writer.position() as usize)
    }

    #[no_mangle]
    pub extern "C" fn initialize(
        name_ptr: *mut u8, 
        name_len: usize, 
        symbol_ptr: *mut u8, 
        symbol_len: usize,
        decimals: u8, 
        initial_supply: u64
    ) -> *mut ERC20Token {
        // Extract name and symbol from wasm memory
        let name = extract_string_from_wasm_memory(name_ptr, name_len);
        let symbol = extract_string_from_wasm_memory(symbol_ptr, symbol_len);
    
        let mut balances = HashMap::new();
        balances.insert("Contract_Owner".to_string(), initial_supply);
    
        let token = ERC20Token {
            name: name,
            symbol: symbol,
            decimals,
            total_supply: initial_supply,
            balances,
            allowed: HashMap::new(),
        };
    
        let token_ptr = Box::into_raw(Box::new(token));
    
        token_ptr
    }

    #[no_mangle]
    pub extern "C" fn store_token_in_memory(token_ptr: *mut ERC20Token, buffer: *mut u8) -> usize {
        let token = unsafe { &*token_ptr };
        let len = Self::serialize_to_memory(token, buffer).expect("Failed to serialize");
        len
    }

    #[no_mangle]
    pub extern "C" fn load_token_from_memory(buffer: *const u8, len: usize) -> *mut ERC20Token {
        let token = Self::deserialize_from_memory(buffer, len).expect("Failed to deserialize");
        Box::into_raw(Box::new(token))
    }
    #[no_mangle]
    pub extern "C" fn destroy(token_ptr: *mut ERC20Token) {
        // Deallocate the memory when you're done with the ERC20Token instance
        unsafe {
            Box::from_raw(token_ptr);
        }
    }
    #[no_mangle]
    pub extern "C" fn balance_of(&self, owner: &str) -> u64 {
        *self.balances.get(owner).unwrap_or(&0)
    }

    #[no_mangle]
    pub extern "C" fn transfer(&mut self, to: &str, value: u64, from: &str) -> bool {
        let sender_balance = self.balance_of(from);
        if sender_balance < value {
            return false;
        }

        let receiver_balance = self.balance_of(to);
        self.balances.insert(from.to_string(), sender_balance - value);
        self.balances.insert(to.to_string(), receiver_balance + value);
        true
    }

    #[no_mangle]
    pub extern "C" fn approve(&mut self, spender: &str, value: u64, owner: &str) -> bool {
        let allowances = self.allowed.entry(owner.to_string()).or_insert(HashMap::new());
        allowances.insert(spender.to_string(), value);
        true
    }

    #[no_mangle]
    pub extern "C" fn allowance(&self, owner: &str, spender: &str) -> u64 {
        if let Some(allowances) = self.allowed.get(owner) {
            *allowances.get(spender).unwrap_or(&0)
        } else {
            0
        }
    }

    #[no_mangle]
    pub extern "C" fn transfer_from(&mut self, from: &str, to: &str, value: u64, spender: &str) -> bool {
        let allowance = self.allowance(from, spender);
        if allowance < value {
            return false;
        }
        
        if !self.transfer(to, value, from) {
            return false;
        }

        let allowances = self.allowed.get_mut(from).unwrap();
        allowances.insert(spender.to_string(), allowance - value);

        true
    }
    #[no_mangle]
    pub extern "C" fn read_name(token_ptr: *mut ERC20Token, buffer: *mut u8, len: usize) -> usize {
        let token = unsafe { &*token_ptr };
        let name_bytes = token.name.as_bytes();

        let to_write = std::cmp::min(name_bytes.len(), len);

        unsafe {
            std::ptr::copy(name_bytes.as_ptr(), buffer, to_write);
        }

        to_write
    }

}
