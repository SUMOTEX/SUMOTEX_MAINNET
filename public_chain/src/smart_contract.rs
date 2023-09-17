use std::collections::HashMap;
use secp256k1::{Secp256k1, PublicKey, SecretKey};

pub fn generate_keypair()->PublicKey {
    let secp = Secp256k1::new();
    let mut rng = secp256k1::rand::thread_rng();
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);
    public_key
}

// Smart contract that is public structure
struct PublicSmartContract {
    contract_address: PublicKey,
    balance: f64,
    nonce: u64,
    timestamp:u64,
}

impl PublicSmartContract {
    // Creates a new PublicSmartContract
    pub fn new(contract_address: PublicKey) -> Self {
        PublicSmartContract {
            contract_address,
            balance: 0.0,
            nonce: 0,
            timestamp: Self::current_timestamp(),
        }
    }
    // Deposit an amount into the contract
    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    // Withdraw an amount from the contract, returns true if successful
    pub fn withdraw(&mut self, amount: f64) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }

    // Increment the nonce, typically called when a transaction is made
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }

    // Get current timestamp in seconds
    fn current_timestamp() -> u64 {
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        since_the_epoch.as_secs()
    }
}

// Sample Blockchain representation with accounts
struct SmartContracts {
    contracts: HashMap<PublicKey, Account>,
}

impl SmartContracts {
    // Creates a new blockchain instance
    fn new() -> Self {
        SmartContracts {
            contracts: HashMap::new(),
        }
    }

    // Adds a new account to the blockchain
    fn add_contract(&mut self) -> String {
        let account = PublicSmartContract::new();
        let address = account.contract_address.clone();
        self.accounts.insert(address.clone(), account);
        address.to_string()
    }

    // Gets a reference to an account given an address
    fn get_account(&self, address: &String) -> Option<&Account> {
        // Step 1: Decode hex string to bytes
        let bytes = hex::decode(address).ok()?;

        // Step 2: Convert bytes to PublicKey
        let public_key = PublicKey::from_slice(&bytes).ok()?;
        self.accounts.get(&public_key)
    }
}
pub fn generate_smart_contract() {

}
