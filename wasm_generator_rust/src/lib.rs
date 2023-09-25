use std::collections::HashMap;

pub struct ERC20Token {
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: u64,
    balances: HashMap<String, u64>,
    allowed: HashMap<String, HashMap<String, u64>>,
}
    
pub fn extract_string_from_wasm_memory(ptr: *mut u8, len: usize) -> String {
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).to_string()
}
impl ERC20Token {
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
}
