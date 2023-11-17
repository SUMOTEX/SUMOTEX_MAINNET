use secp256k1::{Secp256k1, PublicKey, SecretKey};
use libp2p::{
    swarm::{Swarm},
};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use secp256k1::Message;
use secp256k1::Error as Secp256k1Error;
use crate::p2p::AppBehaviour;
use crate::rock_storage;


// Token structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SMTXToken {
    name: String,
    symbol: String,
    decimals: u8,
    max_supply: u64,
    total_supply: u64
}
#[derive(Debug)]
pub enum SigningError {
    Secp256k1Error(Secp256k1Error),
    MessageCreationError,
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
            total_supply: 0
        }
    }
}
pub fn mint(mut token_data: SMTXToken, recipient: &str, amount: u64) -> Result<SMTXToken, &'static str> {
    if token_data.total_supply + amount > token_data.max_supply {
        Err("Max Total Supply minted")
    } else {
        token_data.total_supply += amount;
        // Logic to credit amount to recipient
        Ok(token_data)
    }
}



