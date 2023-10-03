use std::collections::HashMap;
pub trait ERC20Storage {
    fn set_total_supply(&mut self, value: u64);
    fn get_total_supply(&self) -> u64;

    fn set_balance(&mut self, owner: &str, balance: u64);
    fn get_balance(&self, owner: &str) -> u64;

    fn set_allowance(&mut self, owner: &str, spender: &str, value: u64);
    fn get_allowance(&self, owner: &str, spender: &str) -> u64;
}



pub struct InMemoryERC20Storage {
    total_supply: u64,
    balances: HashMap<String, u64>,
    allowances: HashMap<String, HashMap<String, u64>>,
}

impl ERC20Storage for InMemoryERC20Storage {
    pub fn set_total_supply(&mut self, value: u64) {
        self.total_supply = value;
    }

    pub fn get_total_supply(&self) -> u64 {
        self.total_supply
    }

    pub fn set_balance(&mut self, owner: &str, balance: u64) {
        self.balances.insert(owner.to_string(), balance);
    }

    pub fn get_balance(&self, owner: &str) -> u64 {
        self.balances.get(owner).unwrap_or(&0).to_owned()
    }

    pub fn set_allowance(&mut self, owner: &str, spender: &str, value: u64) {
        let allowances = self.allowances.entry(owner.to_string()).or_insert_with(HashMap::new);
        allowances.insert(spender.to_string(), value);
    }

    pub fn get_allowance(&self, owner: &str, spender: &str) -> u64 {
        if let Some(allowances) = self.allowances.get(owner) {
            *allowances.get(spender).unwrap_or(&0)
        } else {
            0
        }
    }
}
