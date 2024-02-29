use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use secp256k1::Error as Secp256k1Error;

// Token structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SMTXToken {
    name: String,
    symbol: String,
    decimals: u8,
    max_supply: u64,
    total_supply: u64,
    balances: HashMap<String, u64>, // Add balances field
}
#[derive(Debug)]
pub enum SigningError {
    Secp256k1Error(Secp256k1Error),
    MessageCreationError,
}
#[derive(Debug)]
pub enum TokenError {
    MaxSupplyExceeded,
}
impl From<Secp256k1Error> for SigningError {
    fn from(err: Secp256k1Error) -> Self {
        SigningError::Secp256k1Error(err)
    }
}

impl SMTXToken {
    pub fn new(name: String, symbol: String, decimals: u8, max_supply: u64) -> Self {
        SMTXToken {
            name,
            symbol,
            decimals,
            max_supply,
            total_supply: 0,
            balances: HashMap::new(), 
        }
    }

    pub fn mint(&mut self, recipient: &str, amount: u64) -> Result<(), TokenError> {
        if self.total_supply + amount > self.max_supply {
            return Err(TokenError::MaxSupplyExceeded);
        }

    
        // Increment the recipient's balance
        let recipient_balance = self.balances.entry(recipient.to_string()).or_insert(0);
        *recipient_balance += amount;
    
        self.total_supply += amount;
    
        Ok(())
    }
    
    // Other token-related functions can be added here
}


