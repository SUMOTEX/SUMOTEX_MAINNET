use std::collections::HashMap;
use libp2p::{
    floodsub::{Topic},
    swarm::{Swarm},
};
use secp256k1::{Secp256k1, PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use crate::rock_storage;
use crate::p2p::AppBehaviour;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;
use std::fs::File;
use std::ffi::CString;
use wasmtime::*;
use wasmtime::Val;
use wasmtime_wasi::WasiCtx;
use wasmtime::MemoryType;
use wasmtime::Linker;
use wasmtime_wasi::sync::WasiCtxBuilder;

pub fn generate_keypair()->(PublicKey,SecretKey) {
    let secp = Secp256k1::new();
    let mut rng = secp256k1::rand::thread_rng();
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);
    (public_key,secret_key)
}

// Smart contract that is public structure
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PublicSmartContract {
    contract_address: String,
    balance: f64,
    nonce: u64,
    timestamp:u64,
}

impl PublicSmartContract {
    // Creates a new PublicSmartContract
    pub fn new() -> Self {
        let (public_key,private_key)=generate_keypair();
        PublicSmartContract {
            contract_address:public_key.to_string(),
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
    contracts: HashMap<String, PublicSmartContract>,
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
        self.contracts.insert(address.clone(), account);
        address.to_string()
    }
}

pub fn store_smart_contract(cmd:&str,swarm:  &mut Swarm<AppBehaviour>) {
    let contract_path = swarm.behaviour().storage_path.get_contract();
    let (public_key,private_key) = generate_keypair();
    let contract = PublicSmartContract::new();
    println!("Path {:?}",contract_path);
    let address = contract.contract_address.clone();
    let serialized_data = serde_json::to_string(&contract).expect("can jsonify request");
    let _ = rock_storage::store_wasm_in_db(contract_path,&address.to_string(),"/Users/leowyennhan/Desktop/sumotex_mainnet/chain/public_chain/cool.wasm");
    let put_item = rock_storage::get_wasm_from_db(contract_path,&address.to_string());
    println!("Smart Contract stored");
}
#[derive(Debug, Clone)]
enum WasmType {
    I32,
    I64,
    F32,
    F64,
    U8,
    U64,
    Str,
    bool
    // Add more types as needed
}

#[derive(Debug, Clone)]
pub struct FunctionDescriptor {
    name: String,
    param_types: Vec<WasmType>,
    return_type: Option<WasmType>,
}
pub struct WasmContract {
    instance: Instance,
    store: Store<WasiCtx>,
    module: Module,
}

enum ParamValue {
    Int64(i64),
    Int32(i32),
    U64(u64),
    U8(u8),
    Str(String),
}

impl WasmContract {
    pub fn new(module_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
    // Define the WASI functions globally on the `Config`.
        let engine = Engine::default();
        // let mut exports = module.exports();
        // while let Some(foo) = exports.next() {
        //     println!("{}", foo.name());
        // }
        let mut linker = Linker::new(&engine);
        //let mut exports = module.exports();
        // Create a WASI context and put it in a Store; all instances in the store
        // share this context. `WasiCtxBuilder` provides a number of ways to
        // configure what the target program will have access to.
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_args().unwrap()
            .build();
        let mut store = Store::new(&engine, wasi);
        let module = Module::from_file(&engine, module_path)?;
        // while let Some(foo) = exports.next() {
        //         println!("Functions: {}", foo.name());
        // }
        wasmtime_wasi::add_to_linker(&mut linker, |s| s).unwrap();
        let instance = linker.instantiate(&mut store, &module)?;
        linker.module(&mut store, "", &module)?;
        Ok(WasmContract {
            instance,
            store,
            module,
        })
    }
    pub fn get_store(&mut self) -> &mut Store<WasiCtx> {
        &mut self.store
    }
    // pub fn dynamic_call(&mut self, descriptor: &FunctionDescriptor, params: Vec<ParamValue>) -> Result<i64, Box<dyn std::error::Error>> {

    //     let memory_export = self.instance.get_export(&mut self.store, "memory").ok_or("Failed to find 'memory' export")?;
    //     let memory = if let wasmtime::Extern::Memory(memory) = memory_export {
    //         memory
    //     } else {
    //         return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Memory export not found")));
    //     };
        
    //     let wasm_params: Vec<Val> = params.into_iter().flat_map(|param| {
    //         match param {
    //             ParamValue::Int32(i) => vec![Val::I32(i)].into_iter(),
    //             ParamValue::Int64(i) => vec![Val::I64(i)].into_iter(),
    //             ParamValue::Str(s) => {
    //                 let (ptr, len) = self.write_string_to_memory(&memory, &s);
    //                 vec![Val::I32(ptr), Val::I32(len)].into_iter() // Produces an iterator over two items.
    //             },
    //         }
    //     }).collect();
    //     let typed_func = self.instance.get_func(&mut self.store, &descriptor.name)
    //         .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Function not found"))?
    //         .typed::<&[Val], &[Val]>(&self.store)
    //         .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
    //     let results = typed_func.call(&mut self.store, &wasm_params)?;
        
    //     match results.get(0) {
    //         Some(Val::I64(i)) => Ok(i),
    //         _ => Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unexpected return type from the function")))
    //     }
    // }
    pub fn exported_functions(&self) -> Vec<String> {
        self.module.exports().map(|export| export.name().to_string()).collect()
    }
}

pub fn create_memory(store: &mut Store<WasiCtx>) -> Result<Memory, Box<dyn std::error::Error>> {
    let memory_type = MemoryType::new(1, None); // 1 page
    let memory = Memory::new(store, memory_type)?;
    Ok(memory)
}

// Part 2: Write data to the memory
pub fn write_data_to_memory(memory: &Memory, input: &str, store: &mut Store<WasiCtx>) -> Result<(i32, i32), Box<dyn std::error::Error>> {
    let input_bytes = input.as_bytes();
    let data = memory.data_mut(store); // Use the store here

    for (i, byte) in input_bytes.iter().enumerate() {
        data[i] = *byte;
    }

    Ok((0, input_bytes.len() as i32)) // Placeholder for data_ptr, replace 0 with the actual pointer if possible
}
pub fn read_wasm_file(module_path: &str) -> Result<(), Box<dyn std::error::Error>>{
    match WasmContract::new(module_path){
        Ok(mut contract) => {
            println!("Contract successfully created.");
            println!("Successfully instantiated the wasm module.");
            // Print the exported functions
            let functions = contract.exported_functions();
            println!("Available Functions: {:?}", functions);
            for func in functions.iter() {
                println!("Exported Function: {}", func);
            }
            let the_memory = create_memory(contract.get_store()).unwrap();
            // Convert strings to Wasm memory pointers and lengths.
            let (name_ptr, name_len) = match write_data_to_memory(&the_memory, "SUMOTEX-T",contract.get_store()) {
                Ok((ptr, len)) => (ptr, len),
                Err(e) => return Err(e.into()),
            };
            
            let (symbol_ptr, symbol_len) = match write_data_to_memory(&the_memory, "SMTX",contract.get_store()) {
                Ok((ptr, len)) => (ptr, len),
                Err(e) => return Err(e.into()),
            };

            let args = vec![
                Val::I32(name_ptr),
                Val::I32(name_len),
                Val::I32(symbol_ptr),
                Val::I32(symbol_len),
                Val::I32(8),
                Val::I64(1000000),
            ];    
            println!("{:?}",args);
            // let result = contract.prepare_and_call(&mut contract, "initialize", args_data);
            // match result {
            //     Ok(val) => println!("Function returned: {:?}", val),
            //     Err(err) => eprintln!("Error: {}", err),
            // }
        },
        Err(e) => {
            eprintln!("Error creating contract: {}", e);
        }
    };

    Ok(())
}