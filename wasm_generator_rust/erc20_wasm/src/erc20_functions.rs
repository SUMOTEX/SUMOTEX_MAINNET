use std::collections::HashMap;
use bincode::{serialize_into, deserialize_from};
use std::io::Cursor;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct ERC20Token {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: u64,
    pub balances: HashMap<String, u64>,
    pub allowed: HashMap<String, HashMap<String, u64>>,
}

fn extract_string_from_wasm_memory(ptr: *mut u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}
static mut TOKEN_PTR: Option<*mut ERC20Token> = None;


impl ERC20Token {
    fn deserialize_from_memory(buffer: *const u8, len: usize) -> Result<ERC20Token, Box<dyn std::error::Error>> {
        let reader = unsafe { Cursor::new(std::slice::from_raw_parts(buffer, len)) };
        let token = deserialize_from(reader)?;
        Ok(token)
    }

    fn serialize_to_memory(token: &ERC20Token, buffer: *mut u8) -> Result<usize, Box<dyn std::error::Error>> {
        let mut writer = unsafe {
            // Assuming the buffer is large enough to hold the serialized data
            Cursor::new(std::slice::from_raw_parts_mut(buffer, 1024))
        };
        serialize_into(&mut writer, token)?;
        Ok(writer.position() as usize)
    }

    #[no_mangle]
    pub extern "C" fn initialize(name_ptr: *mut u8, name_len: usize, symbol_ptr: *mut u8, symbol_len: usize, decimals: u8, initial_supply: u64) -> *mut u8 {
        let name = extract_string_from_wasm_memory(name_ptr, name_len);
        let symbol = extract_string_from_wasm_memory(symbol_ptr, symbol_len);

        let mut balances = HashMap::new();
        balances.insert("Contract_Owner".to_string(), initial_supply);

        let token = ERC20Token {
            name, symbol, decimals, total_supply: initial_supply, balances, allowed: HashMap::new()
        };

        let serialized_data = token.to_memory();

        // Allocate space for the serialized token and copy data to it
        let buf = unsafe {
            let layout = std::alloc::Layout::from_size_align(serialized_data.len(), 1).unwrap();
            std::alloc::alloc(layout)
        };
        
        unsafe {
            std::ptr::copy_nonoverlapping(serialized_data.as_ptr(), buf, serialized_data.len());
        }
        buf
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
    pub extern "C" fn balance_of(&self, owner: &String) -> u64 {
        *self.balances.get(owner).unwrap_or(&0)
    }


    #[no_mangle]
    pub extern "C" fn transfer(self, to: String, value: u64, from: String) -> Result<ERC20Token, &'static str> {
        let sender_balance = self.balance_of(&from);
        if sender_balance < value {
            return Err("Insufficient balance");
        }

        let receiver_balance = self.balance_of(&to);
        let mut new_balances = self.balances.clone();
        new_balances.insert(from, sender_balance - value);
        new_balances.insert(to, receiver_balance + value);

        Ok(ERC20Token {
            name: self.name,
            symbol: self.symbol,
            decimals: self.decimals,
            total_supply: self.total_supply,
            balances: new_balances,
            allowed: self.allowed,
        })
    }



    #[no_mangle]
    pub extern "C" fn approve(self, spender: String, value: u64, owner: String) -> Result<ERC20Token, &'static str> {
        let mut new_allowed = self.allowed.clone();
        let allowances = new_allowed.entry(owner).or_insert(HashMap::new());
        allowances.insert(spender, value);

        Ok(ERC20Token {
            name: self.name,
            symbol: self.symbol,
            decimals: self.decimals,
            total_supply: self.total_supply,
            balances: self.balances,
            allowed: new_allowed,
        })
    }

    #[no_mangle]
    pub extern "C" fn allowance(&self, owner: &String, spender: &String) -> u64 {
        if let Some(allowances) = self.allowed.get(owner) {
            *allowances.get(spender).unwrap_or(&0)
        } else {
            0
        }
    }

    #[no_mangle]
    pub extern "C" fn transfer_from(self, from: String, to: String, value: u64, spender: String) -> Result<ERC20Token, &'static str> {
        let allowance = self.allowance(&from, &spender);
        if allowance < value {
            return Err("Allowance exceeded");
        }
        
        let result_token = self.transfer(to.clone(), value, from.clone())?;
        let mut new_allowed = result_token.allowed.clone();
        if let Some(allowances) = new_allowed.get_mut(&from) {
            allowances.insert(spender, allowance - value);
        }

        Ok(result_token)
    }

    #[no_mangle]
    pub extern "C" fn read_name(&self, len: usize) -> *mut u8 {
        let name_bytes = self.name.as_bytes();
        let result_str = String::from_utf8_lossy(&name_bytes[..std::cmp::min(name_bytes.len(), len)]).to_string();
    
        // Serialize the string into a JSON format
        let serialized = serde_json::to_string(&result_str).unwrap();
        let bytes = serialized.into_bytes();
    
        // Create a buffer to hold the bytes
        let len = bytes.len();
        let buf = unsafe {
            std::alloc::alloc(std::alloc::Layout::from_size_align(len, 1).unwrap())
        };
        
        // Copy the bytes to the buffer and return its pointer
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, len);
        }
        buf
    }
    #[no_mangle]
    pub extern "C" fn read_symbol(self, len: usize) -> String {
        let symbol_bytes = self.symbol.as_bytes();
        String::from_utf8_lossy(&symbol_bytes[..std::cmp::min(symbol_bytes.len(), len)]).to_string()
    }
    #[no_mangle]
    pub extern "C" fn total_supply() -> i64 {
        unsafe {
            if let Some(token_ptr) = TOKEN_PTR {
                (*token_ptr).total_supply as i64
            } else {
                // Return an error value or handle the case where the token is not initialized
                -1
            }
        }
    }
    #[no_mangle]
    pub extern "C" fn read_decimals(self) -> u8 {
        self.decimals
    }
}
