use std::collections::HashMap;

pub struct ERC20Token {
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: u64,
    balances: HashMap<String, u64>,
    allowed: HashMap<String, HashMap<String, u64>>,
}

impl ERC20Token {
    #[no_mangle]
    pub extern "C" fn initialize(name: &str, symbol: &str, decimals: u8, initial_supply: u64) -> Self {
        let mut balances = HashMap::new();
        balances.insert("creator".to_string(), initial_supply);

        ERC20Token {
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals,
            total_supply: initial_supply,
            balances,
            allowed: HashMap::new(),
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
