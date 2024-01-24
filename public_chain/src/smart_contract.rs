use std::collections::HashMap;
use libp2p::{
    swarm::{Swarm},
};
use std::fs;
use std::fs::File;
use std::io::Write;
use rocksdb::{DBWithThreadMode,SingleThreaded};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::p2p::AppBehaviour;
use secp256k1::{Secp256k1, PublicKey, SecretKey};
use crate::rock_storage;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;
use wasmtime::*;
//use wasmtime::Val;
use wasmtime_wasi::WasiCtx;
use wasmtime::MemoryType;
use wasmtime::Linker;
use wasmtime::component::Type;
use wasmtime_wasi::sync::WasiCtxBuilder;
use bincode::{serialize, deserialize};
use bincode::{ Error as BincodeError};
use rocksdb::Error as RocksDBError;
use wasm_bindgen::JsCast;
extern crate base64;
use base64::{encode, decode};
use crate::gas_calculator;
use crate::public_txn;
use crate::account;
use crate::rock_storage::StoragePath;
use crate::public_swarm;
use crate::public_txn::TransactionType;
use crate::public_txn::PublicTxn;

#[derive(Serialize, Deserialize)]
pub struct ERC721Token {
    pub name: String,
    pub symbol: String,
    pub owner_of: HashMap<u64, String>,  // tokenId -> owner address
    pub token_to_ipfs: HashMap<u64, String>,  // tokenId -> IPFS hash
    pub next_token_id: u64,
}
#[derive(Serialize, Deserialize,Debug, Clone)]
pub struct TokenDetails {
    pub owner: String,
    pub ipfs_link: String,
}
// Smart contract that is public structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicSmartContract {
    contract_address: String,
    balance: f64,
    nonce: u64,
    wasm_file:Vec<u8>,
    timestamp:u64,
}

#[repr(C)]
#[derive(Debug)]
pub struct OwnerData {
    ptr: i32,
    len: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct Parameter {
    name: String,
    #[serde(rename = "type")]
    p_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct FunctionInfo {
    inputs: Vec<Parameter>,
    outputs: Vec<Parameter>,
}



impl PublicSmartContract {
    // Creates a new PublicSmartContract
    pub fn new(public_key:String, wasm_file: Vec<u8>) -> Self {
        PublicSmartContract {
            contract_address:public_key.to_string(),
            balance: 0.0,
            nonce: 0,
            wasm_file,
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

// Account structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Contract {
    public_address: String,
    contract_creator:String,
    balance: f64,   
    nonce: u64
}

pub enum ParamValue {
    Int64(i64),
    Int32(i32),
    U64(u64),
    U8(u8),
    Str(String),
}
pub struct WasmParams {
    pub name: String,
    pub args: Vec<Val>,
    // Add any other params related to the wasm execution.
}

pub fn generate_keypair()->(PublicKey,SecretKey) {
    let secp = Secp256k1::new();
    let mut rng = secp256k1::rand::thread_rng();
    let (secret_key, public_key) = secp.generate_keypair(&mut rng);
    (public_key,secret_key)
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
    pub fn call(
        &mut self,
        contract_path:&DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        wasm_params: &WasmParams,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
        
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;

        let wasm_memory = link
            .get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;

        let data = wasm_memory.data(&mut store);
        let byte_vector: Vec<u8> = data.to_vec();
        let args_tuple = (
            match &wasm_params.args[0] {
                Val::I32(val) => *val,
                _ => return Err("Failed to unwrap i32 from argument 1".into()),
            },
            match &wasm_params.args[1] {
                Val::I32(val) => *val,
                _ => return Err("Failed to unwrap i32 from argument 2".into()),
            },
            match &wasm_params.args[2] {
                Val::I32(val) => *val,
                _ => return Err("Failed to unwrap i32 from argument 3".into()),
            },
            match &wasm_params.args[3] {
                Val::I32(val) => *val,
                _ => return Err("Failed to unwrap i32 from argument 4".into()),
            },
            match &wasm_params.args[4] {
                Val::I32(val) => *val,
                _ => return Err("Failed to unwrap i32 from argument 5".into()),
            },
            match &wasm_params.args[5] {
                Val::I64(val) => *val,
                _ => return Err("Failed to unwrap i64 from argument 6".into()),
            },
        );
        //rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &byte_vector)?;
        let initialise_func = link.get_typed_func::<(i32, i32, i32, i32, i32, i64), ()>(&mut store, &wasm_params.name)?;
        let result = initialise_func.call(&mut store,  args_tuple)?;

        println!("Initialize: {:?}",result);
        let updated_data = wasm_memory.data(&mut store);
        let updated_byte_vector: Vec<u8> = updated_data.to_vec();
        rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &updated_byte_vector)?;
        Ok(())
    }       
    pub fn call_function(
        &mut self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        account_key: &String,
        contract_address: &String,
        function_name:  &String,
        args_input_values:&String,
        args_input: &String,
        args_output:  &String,
    ) -> Result<Vec<i64>, Box<dyn std::error::Error>>  {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);

        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        let wasm_memory = link
        .get_memory(&mut store, "memory")
        .ok_or_else(|| "Failed to find `memory` export")?;

        // Load the current memory state from the database
        let serialized_contract = rock_storage::get_from_db_vector(contract_path, contract_address).unwrap_or_default();
        let mut contract: PublicSmartContract = serde_json::from_slice(&serialized_contract[..])
            .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;
        contract.nonce+=1;
        let saved_data = contract.wasm_file;
        if saved_data.len() > wasm_memory.data_size(&store) {
            let additional_pages_needed = ((saved_data.len() as u64 + (64 * 1024 - 1)) / (64 * 1024)) - (wasm_memory.data_size(&store) as u64 / (64 * 1024));
            wasm_memory.grow(&mut store, additional_pages_needed).map_err(|_| "Failed to grow memory")?;
        }
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data);

        let data = wasm_memory.data(&mut store);
        let byte_vector: Vec<u8> = data.to_vec();
        //rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &byte_vector)?;
        let input_types: Vec<Parameter> = serde_json::from_str(args_input)?;
        // Parse the output types from the JSON string
        println!("Input Types: {:?}",input_types);
        let output_types: Vec<Parameter> = serde_json::from_str(args_output)?;
        println!("Output Types: {:?}",output_types);

        // Calculate the number of required outputs
        let output_values_count = output_types.len();
        let args_input_values: serde_json::Value = serde_json::from_str(args_input_values)?;
        println!("Input Arg Values: {:?}",args_input_values);
        let arg_values: Result<Vec<Val>, _> = args_input_values.as_array()
            .ok_or("args_input_values is not an array")?
            .iter()
            .zip(input_types.iter())
            .map(|(value, param)| {
                match param.p_type.as_str() {
                    "i32" => value.as_i64().map(|i| Val::I32(i as i32)).ok_or("Invalid i32 value"),
                    "usize" => value.as_u64().map(|u| Val::I64(u as i64)).ok_or("Invalid usize value"),
                    "isize" => value.as_i64().map(Val::I64).ok_or("Invalid isize value"),
                    _ => Err("Unsupported type"),
                }
            })
            .collect();
        let arg_values = arg_values?; 
        let mut outputs = vec![wasmtime::Val::I32(0); output_values_count]; // Adjust the size and type as needed
        let func = link.get_func(&mut store, function_name)
            .ok_or_else(|| format!("Function '{}' not found", function_name))?;

        // Using the instance to call the function
        func.call(&mut store, &arg_values, &mut outputs)?;
        // Convert the outputs to the desired format
        let result_values: Vec<i64> = outputs.iter().map(|val| {
            match val {
                Val::I32(i) => *i as i64, // Convert i32 to i64
                Val::I64(i) => *i,        // Already i64, no conversion needed
                _ => 0i64,                // Use 0i64 as a placeholder for unsupported types
            }
        }).collect();
        
        //let result_values = func.call(&mut store, &arg_values)?;
        let updated_data = wasm_memory.data(&mut store);
        let updated_byte_vector: Vec<u8> = updated_data.to_vec();
        contract.wasm_file=updated_byte_vector;
        let updated_serialized_contract = serde_json::to_vec(&contract)
        .map_err(|e| format!("Failed to serialize updated contract: {:?}", e))?;
        rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &updated_serialized_contract)?;
    
        Ok(result_values)
    }
    pub fn get_store(&mut self) -> &mut Store<WasiCtx> {
        &mut self.store
    }  
    pub fn create_contract(
        &mut self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        wasm_params: &WasmParams // Added parameter to specify the function name
    ) -> Result<(), Box<dyn std::error::Error>> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
    
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
    
        let wasm_memory = link
            .get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
    
        let data = wasm_memory.data(&mut store);
        let byte_vector: Vec<u8> = data.to_vec();
    
        //rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &byte_vector)?;
    
        // Dynamically calling a function based on the input string
        let dyn_func = link.get_func(&mut store, "initialize")
            .ok_or_else(|| format!("Failed to find function `{}`", "initialize"))?;
    
        let args: Vec<Val> = wasm_params.args.iter().cloned().collect();
    
        // Prepare a mutable slice for the results if the function returns values
        let mut results = vec![Val::I32(0); dyn_func.ty(&store).results().len()]; // Adjust the size based on expected results
    
        dyn_func.call(&mut store, &args[..], &mut results)?;
    
        let updated_data = wasm_memory.data(&mut store);
        let updated_byte_vector: Vec<u8> = updated_data.to_vec();
        let updated_byte_vector_copy: Vec<u8> = updated_data.to_vec();
        let contract = PublicSmartContract::new(contract_info.pub_key.clone(),updated_byte_vector_copy);
        let serialized_contract = serde_json::to_vec(&contract)?;
        rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &serialized_contract)?;
    
        Ok(())
    }
    
    
    pub fn read(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        pub_key: &str,
        function_name: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
        
        // 1. Instantiate the WebAssembly module.
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        
        // 2. Get the WebAssembly memory.
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        
        //let data = wasm_memory.data(&store);
        
        // 3. Call the desired function in the WebAssembly module by its name.
        let result = link.get_typed_func::<_, (i32, i32)>(&mut store, function_name)?
        .call(&mut store, ())?;
    
        let data = wasm_memory.data(&store);
        let start_idx = result.0 as usize;
        let length = result.1 as usize;
    
        // 4. Slice the data using the returned pointer and length.
        let result_data = data[start_idx..start_idx + length].to_vec();
            
        // 5. Return the extracted data.
        Ok(result_data)
    }
    pub fn read_numbers(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        pub_key: &str,
        function_name: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
        
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        let saved_data = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
    
        let current_memory_size = wasm_memory.data_size(&store);
        let required_memory_size = saved_data.len();
        
        if required_memory_size > current_memory_size {
            let additional_pages_needed = (required_memory_size - current_memory_size + (64 * 1024 - 1)) / (64 * 1024);
            wasm_memory.grow(&mut store, additional_pages_needed as u64)
                .map_err(|_| "Failed to grow memory")?;
        }
    
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data);
        
        let result = link.get_typed_func::<_, i64>(&mut store, function_name)?
        .call(&mut store, ())?;
        
        Ok(result)
    }
    
    pub fn test_write(&self, 
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        pub_key: &str)-> Result<(), Box<dyn std::error::Error>>
        {   
            println!("Initializing engine and linker...");
            println!("Contract name and pub key {:?}", pub_key);
            
            let engine = Engine::default();
            let mut linker = Linker::new(&engine);
            let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
            let mut store = Store::new(&engine, wasi);
            
            println!("Attempting to instantiate the WebAssembly module...");
            let module = Module::from_file(&engine, &contract_info.module_path)?;
            wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
            let link = linker.instantiate(&mut store, &module)?;
            
            println!("Fetching WebAssembly memory...");
            let wasm_memory = link.get_memory(&mut store, "memory").ok_or("Failed to find `memory` export")?;
            for export in module.exports() {
                println!("Exported item: {}", export.name());
            }
            
            let input_string = "Hello, WebAssembly!";
            let bytes = input_string.as_bytes();
            
            // Assuming the start of your static memory is where you want to write
            let offset = 0;

            println!("Attempting to read from WebAssembly memory...");
            let offset_func = link.get_typed_func::<(), i32>(&mut store, "get_string_offset")?;
            let ptr = offset_func.call(&mut store, ())?  as usize;
        
            let length_func = link.get_typed_func::<(), i32>(&mut store, "get_string_length")?;
            let len = length_func.call(&mut store, ())?  as usize;
        
            //println!("Read from WebAssembly memory: {}", string_value);
            // Convert the pointer back to a Rust slice and then to a string for printing
        
            let data = wasm_memory.data(&store)[ptr..(ptr + len)].to_vec();
            let output_string = String::from_utf8(data)?;
            println!("String: {}", output_string);
            // let slice = &memory_data[pointer_offset as usize..(pointer_offset as usize + bytes.len())];
            // let output_string = std::str::from_utf8(slice).unwrap();
            //println!("Read from WebAssembly memory: {}", output_string);
            
            Ok(())
            
        }

    pub fn decode_token_details(
        encoded_value: i64,
        wasm_memory: &wasmtime::Memory,
        store: &mut wasmtime::Store<WasiCtx>,
    ) -> Result<TokenDetails, Box<dyn std::error::Error>> {
        // Extract pointer and length
        let pointer = (encoded_value >> 32) as usize;
        let length = (encoded_value & 0xFFFFFFFF) as usize;
    
        // Validate pointer and length
        let mem_size = wasm_memory.data(&mut *store).len();
        let data = &wasm_memory.data(&mut *store)[pointer..pointer + length];
        println!("Data: {:?}", &data);
        if pointer + length > mem_size {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Pointer and length exceed WebAssembly memory size.",
            )));
        }
        // Deserialize the data
        bincode::deserialize(data).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
    
    pub fn is_memory_empty(wasm_memory: &wasmtime::Memory, store: &wasmtime::Store<WasiCtx>) -> bool {
        for byte in wasm_memory.data(store) {
            if *byte != 0 {
                return false;
            }
        }
        true
    }

    pub fn mint_token(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        owner:&str,
        pub_key: &str,
        ipfs_hash: &str
    ) -> Result<i32, Box<dyn std::error::Error>> {
        println!("Initializing engine and linker...");
        println!("Contract name and pub key {:?}",pub_key);
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let module = Module::from_file(&engine, "./sample721.wasm")?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);


        println!("Attempting to instantiate the WebAssembly module...");
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        let instance = linker.instantiate(&mut store, &module)?;
        //let contract = WasmContract::new(instance)?;
        // ... (Your operations using token_instance)
    
        println!("Fetching WebAssembly memory...");
        //let wasm_memory = contract.instance.get_memory(&mut store,"memory").ok_or_else(|| "Failed to find `memory` export")?;
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        let serialized_contract = rock_storage::get_from_db_vector(contract_path, contract_address).unwrap_or_default();
        let mut contract: PublicSmartContract = serde_json::from_slice(&serialized_contract[..])
            .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;
        contract.nonce+=1;
        let saved_data = contract.wasm_file;
        let saved_data_length = saved_data.len() as u64;
        let current_memory_size_bytes = wasm_memory.data_size(&store) as u64;
        if saved_data.len() > wasm_memory.data_size(&store) {
            // If not, calculate how many additional memory pages are needed
            // Calculate the number of additional pages needed, rounding up
            let additional_pages_needed = ((saved_data_length + (64 * 1024 - 1)) / (64 * 1024)) - (current_memory_size_bytes / (64 * 1024));
            // Attempt to grow the WebAssembly memory by the required number of pages
            if saved_data_length > current_memory_size_bytes {
                wasm_memory.grow(&mut store, additional_pages_needed).map_err(|_| "Failed to grow memory")?;
            }
        }
        // After ensuring the memory is large enough, copy the saved data into the WebAssembly memory within bounds
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data);


        println!("Attempting to call mint function in the WebAssembly module...");
    
        let mint_func = match link.get_typed_func::<(i32,i32,i32,i32), u32>(&mut store, "mint") {
            Ok(func) => func,
            Err(err) => {
                println!("Error retrieving the 'mint' function: {}", err);
                return Err(err.into());
            }
        };
        let new_data_offset = saved_data.len() as i32;
        let owner_data_bytes = owner.as_bytes().len() as i32;
        let ipfs_data_bytes = ipfs_hash.as_bytes().len() as i32;
        let required_memory_size_bytes = new_data_offset + owner_data_bytes + ipfs_data_bytes;        

        let current_memory_size_bytes = wasm_memory.data_size(&store) as i32; // Size in bytes
        // Check if we need more memory pages and grow memory if needed
        if required_memory_size_bytes > current_memory_size_bytes {
            // Calculate how many more pages are needed, rounding up
            let additional_pages_needed = ((required_memory_size_bytes - current_memory_size_bytes + (64 * 1024 - 1)) / (64 * 1024)) as u32;
            // Grow the memory
            wasm_memory.grow(&mut store, additional_pages_needed as u64).map_err(|_| "Failed to grow memory")?;

        }
        // We need to limit the scope of the memory view so that we can mutably borrow `store` again later
        {
            let memory_view = wasm_memory.data_mut(&mut store);
            // Ensure the new data offset is within the bounds of the current memory size
            if new_data_offset as usize >= memory_view.len() {
                return Err("New data offset is out of the current memory bounds.".into());
            }
        } // `memory_view` goes out of scope here, so we can mutably borrow `store` again


        let (name_ptr, name_len) = write_data_to_memory(&wasm_memory, owner, new_data_offset, &mut store)?;
        println!("Owner data pointer: {:?}, length: {:?}", name_ptr, name_len);
        let ipfs_memory_offset = name_ptr + name_len;

        // Check that the IPFS data fits within memory bounds
        {
            let memory_view = wasm_memory.data_mut(&mut store);
            if ipfs_memory_offset as usize + ipfs_data_bytes as usize > memory_view.len() {
                return Err("IPFS data offset or length is out of the current memory bounds.".into());
            }
        } // The scope of memory_view ends here
        
        // Write the IPFS hash
        let (ipfs_ptr, ipfs_len) = write_data_to_memory(&wasm_memory, ipfs_hash, ipfs_memory_offset, &mut store)?;

        let mint_result = mint_func.call(&mut store, (
            name_ptr as i32,
            name_len as i32,
            ipfs_ptr as i32,
            ipfs_len as i32,
        ));
        match mint_result {
            Ok(token_id) => {
                let updated_data = wasm_memory.data(&mut store);
                let updated_byte_vector: Vec<u8> = updated_data.to_vec();
                if saved_data == updated_data.to_vec() {
                    println!("No change in the WebAssembly memory after mint operation.");
                } else {
                    println!("WebAssembly memory updated after mint operation.");
                }
                contract.wasm_file = updated_data.to_vec()
                let updated_serialized_contract = serde_json::to_vec(&contract)
                .map_err(|e| format!("Failed to serialize updated contract: {:?}", e))?;
                rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &updated_serialized_contract)?;        
                println!("Token ID value: {:?}", token_id);
                //Ok(token_id);
                Ok(token_id as i32)
                // rest of the logic here
            },
            Err(e) => {
                println!("Mint function failed: {}", e);
                // handle the error
                Err(e.into())
            }
        }
    }
    pub fn read_owner_token(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        pub_key: &str,
        id: i32,
    ) -> Result<String, Box<dyn std::error::Error>> {
        println!("Initializing engine and linker...");
        println!("Contract name and pub key {:?}",pub_key);
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
    
        println!("Attempting to instantiate the WebAssembly module...");
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        // ... (Your operations using token_instance)
    
        println!("Fetching WebAssembly memory...");
    
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        let saved_data = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
        // 2.2. Set this memory state into the WebAssembly module's memory.
        let current_memory_size = wasm_memory.data_size(&store); // Current memory size in bytes
        // Convert `current_memory_size` from `usize` to `u64` for the calculation.
        let current_memory_size_pages = (current_memory_size as u64) / (64 * 1024);
        let required_pages = (saved_data.len() as u64 + (64 * 1024 - 1)) / (64 * 1024);
        let additional_pages_needed = if required_pages > current_memory_size_pages {
            required_pages - current_memory_size_pages
        } else {
            0  // No additional pages are needed.
        };

        if additional_pages_needed > 0 {
            wasm_memory.grow(&mut store, additional_pages_needed as u64).map_err(|_| "Failed to grow memory")?;
        }

        if saved_data.len() > wasm_memory.data(&mut store).len() {
            return Err("Saved data is larger than the available WASM memory.".into());
        }
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data); 
        let memory_data = wasm_memory.data(&store);
        // Assuming the memory content is ASCII text
        if let Ok(text) = std::str::from_utf8(&memory_data) {
            println!("Memory Contents: {}", text);
        } else {
            // If the memory contains binary data, you can print it as bytes
            println!("Memory Contents (as bytes)");
        }
        println!("Attempting to call owner data function in the WebAssembly module...");
        println!("Token ID being queried: {}", id);
        match link.get_typed_func::<i32, i32>(&mut store, "get_owner_ptr") {
            Ok(func) => {
                let ptr_func = func;
                let ptr = ptr_func.call(&mut store, id)?;
        
                // Check if the function returned an error indicator
                if ptr == 0 {
                    return Err("Error occurred while fetching pointer in length function.".into());
                }
                println!("Pointer: {}", ptr);
        
                match link.get_typed_func::<i32, i64>(&mut store, "get_owner_len") {
                    Ok(func) => {
                        match func.call(&mut store, id) {
                            Ok(token_result) => {
                                println!("Len Contract: {:?}", token_result);
                                let start = ptr as usize;
                                let end = start + token_result as usize;
                                if end <= wasm_memory.data(&store).len() {
                                    let data = wasm_memory.data(&store)[start..end].to_vec();
                 
                                    match String::from_utf8(data) {
                                        Ok(name) => {
                                            println!("Owner: {:?}", name);
                                            return Ok(name);
                                            // Other processing code here
                                        },
                                        Err(e) => {
                                            println!("Error converting bytes to String: {:?}", e);
                                            return Err(e.into());
                                            // Handle the error appropriatelyv
                                        },
                                    }
                                } else {
                                    println!("Invalid memory access: Out of bounds");
                                    return Err("error".into());
                                    // Handle the out-of-bounds memory access appropriately
                                }
                                
                                // Other processing code here
                            },
                            Err(e) => {
                                println!("Failed to call get_owner_len_by_token_id function: {:?}", e);
                                return Err(e.into());
                            },
                        }
                    },
                    Err(e) => {
                        println!("Failed to call get_owner_len_by_token_id function: {:?}", e);
                        return Err(e.into());
                    },
                }
            },
            Err(e) => {
                println!("Failed to call get_owner_ptr function: {:?}", e);
                return Err(e.into());
            },
        }        
    Ok("".to_string())
    }
    pub fn read_ipfs_token(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        pub_key: &str,
        id: i32,
    ) -> Result<String, Box<dyn std::error::Error>> {
        println!("Contract name and pub key {:?}",pub_key);
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
    
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        // ... (Your operations using token_instance)
    
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        let saved_data = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
        // 2.2. Set this memory state into the WebAssembly module's memory.
        let current_memory_size = wasm_memory.data_size(&store); // Current memory size in bytes
        // Convert `current_memory_size` from `usize` to `u64` for the calculation.
        let current_memory_size_pages = (current_memory_size as u64) / (64 * 1024);
        let required_pages = (saved_data.len() as u64 + (64 * 1024 - 1)) / (64 * 1024);
        let additional_pages_needed = if required_pages > current_memory_size_pages {
            required_pages - current_memory_size_pages
        } else {
            0  // No additional pages are needed.
        };

        if additional_pages_needed > 0 {
            wasm_memory.grow(&mut store, additional_pages_needed as u64).map_err(|_| "Failed to grow memory")?;
        }

        if saved_data.len() > wasm_memory.data(&mut store).len() {
            return Err("Saved data is larger than the available WASM memory.".into());
        }
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data); 
        let memory_data = wasm_memory.data(&store);
        // Assuming the memory content is ASCII text
        if let Ok(text) = std::str::from_utf8(&memory_data) {
            println!("Memory Contents: {}", text);
        } else {
            // If the memory contains binary data, you can print it as bytes
            println!("Memory Contents (as bytes)");
        }
        match link.get_typed_func::<i32, i32>(&mut store, "get_ipfs_ptr") {
            Ok(func) => {
                let ptr_func = func;
                let ptr = ptr_func.call(&mut store, id)?;
        
                // Check if the function returned an error indicator
                if ptr == 0 {
                    return Err("Error occurred while fetching pointer in length function.".into());
                }
                println!("Pointer: {}", ptr);
        
                match link.get_typed_func::<i32, i64>(&mut store, "get_ipfs_len") {
                    Ok(func) => {
                        match func.call(&mut store, id) {
                            Ok(token_result) => {
                                println!("Len Contract: {:?}", token_result);
                                let start = ptr as usize;
                                let end = start + token_result as usize;
                                if end <= wasm_memory.data(&store).len() {
                                    let data = wasm_memory.data(&store)[start..end].to_vec();
                 
                                    match String::from_utf8(data) {
                                        Ok(name) => {
                                            println!("IPFS: {:?}", name);
                                            return Ok(name);
                                            // Other processing code here
                                        },
                                        Err(e) => {
                                            println!("Error converting bytes to String: {:?}", e);
                                            return Err(e.into());
                                            // Handle the error appropriately
                                        },
                                    }
                                } else {
                                    println!("Invalid memory access: Out of bounds");
                                    return Err("error".into());
                                    // Handle the out-of-bounds memory access appropriately
                                }
                                
                                // Other processing code here
                            },
                            Err(e) => {
                                println!("Failed to call get_ipfs_len function: {:?}", e);
                                return Err(e.into());
                            },
                        }
                    },
                    Err(e) => {
                        println!("Failed to call get_ptr_len function: {:?}", e);
                        return Err(e.into());
                    },
                }
            },
            Err(e) => {
                println!("Failed to call get_ipfs_len function: {:?}", e);
                return Err(e.into());
            },
        }        
    }
    pub fn exported_functions(&self) -> Vec<String> {
        self.module.exports().map(|export| export.name().to_string()).collect()
    }
    pub fn get_ipfs_link(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        token_id: u64,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        
        let result_bytes = self.read(contract_path, contract_info, &contract_info.pub_key, "get_ipfs_link")?;
        let result_string = String::from_utf8(result_bytes)?;

        Ok(Some(result_string))
    }
    // pub fn get_erc20_name(cmd: &str, swarm: &mut Swarm<AppBehaviour>) -> Result<(), Box<dyn std::error::Error>> {
    //     if let Some(data) = cmd.strip_prefix("contract name ") {
    //         let contract = WasmContract::new("./sample.wasm")?;
    //         let contract_path = swarm.behaviour().storage_path.get_contract();
    
    //         let contract_info = ContractInfo {
    //             module_path: "./sample.wasm".to_string(),
    //             pub_key: data.to_string(),
    //         };
    
    //         let name = contract.read_name(contract_path, &contract_info, &data.to_string())?;
    
    //         println!("Contract Name: {}", name);
    //     }
    //     Ok(())
    // }
    // pub fn read_name(
    //     &self,
    //     contract_path: &DBWithThreadMode<SingleThreaded>,
    //     contract_info: &ContractInfo,
    //     pub_key: &str,
    // ) -> Result<String, Box<dyn std::error::Error>> {
    //     // Call the 'read_symbol' function using the same mechanism
    //     let result_bytes = self.read(contract_path, contract_info, pub_key, "read_name")?;

    //     // Convert the result bytes into a string
    //     let result_string = String::from_utf8(result_bytes)?;

    //     Ok(result_string)
    // }    
}

pub fn create_memory(store: &mut Store<WasiCtx>) -> Result<Memory, Box<dyn std::error::Error>> {
    let memory_type = MemoryType::new(1, None); // 1 page
    let memory = Memory::new(store, memory_type)?;
    Ok(memory)
}

// Part 2: Write data to the memory
pub fn write_data_to_memory(memory: &Memory, input: &str, offset: i32, store: &mut Store<WasiCtx>) -> Result<(i32, i32), Box<dyn std::error::Error>> {
    let input_bytes = input.as_bytes();
    let data = memory.data_mut(store);

    if (offset as usize) + input_bytes.len() > data.len() {
        return Err("Not enough memory allocated".into());
    }

    let start = offset as usize;
    data[start..start + input_bytes.len()].copy_from_slice(input_bytes);

    Ok((offset, input_bytes.len() as i32))
}

fn val_to_param_value(val: Val) -> ParamValue {
    match val {
        Val::I32(i) => ParamValue::Int32(i),
        Val::I64(i) => ParamValue::Int64(i),
        // ... handle other cases as needed
        _ => panic!("Unsupported conversion"),  // Or provide a better error handling mechanism.
    }
}
// Outside the WasmContract impl
pub struct ContractInfo {
    pub module_path: String,
    pub pub_key: String,
}

pub fn read_wasm_file(module_path: &str,path:&DBWithThreadMode<SingleThreaded>, pub_key: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut contract = WasmContract::new(module_path)?;

    println!("Contract successfully created.");
    println!("Successfully instantiated the wasm module.");

    // Print the exported functions
    //let functions = contract.exported_functions();
    // println!("Available Functions: {:?}", functions);
    // for func in functions.iter() {
    //     println!("Exported Function: {}", func);
    // }

    let the_memory = create_memory(contract.get_store())?;
    let owner_memory_offset = 0;
    let (name_ptr, name_len) = write_data_to_memory(&the_memory, "SUMOTEX-T",owner_memory_offset, contract.get_store())?;
    let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, "SMTX",owner_memory_offset, contract.get_store())?;

    let vals = vec![
        Val::I32(name_ptr as i32),
        Val::I64(name_len as i64),
        Val::I32(symbol_ptr as i32),
        Val::I64(symbol_len as i64),
        Val::I32(8),
        Val::I64(1000000),
    ];

    let args: Vec<ParamValue> = vals.into_iter().map(val_to_param_value).collect();
    // Commented out the below call as it seems unfinished, uncomment and complete when ready
    // let result = contract.call(&pub_key, "initialize", args); 

    Ok(())
}
// pub fn create_erc721_contract_official(call_address:&str,private_key:&str,contract_name:&str,contract_symbol:&str)->
//     Result<(String,String,u128, PublicTxn), Box<dyn std::error::Error>>{
//     let (public_key,private_key) = generate_keypair(); 
//     let path = "./contract/db";
//     let contract_path = rock_storage::open_db(path);
//     match contract_path {
//         Ok(contract_db) => {
//             let contract_info = ContractInfo {
//                 module_path: "./sample721.wasm".to_string(),
//                 pub_key:public_key.to_string(),
//             };
//             let _ = gas_calculator::calculate_gas_for_contract_creation("./sample721.wasm");
//             let mut contract = WasmContract::new("./sample721.wasm")?;
//             let functions = contract.exported_functions();
        
//             let the_memory = create_memory(contract.get_store())?;
//             let owner_memory_offset = 0;
//             let (name_ptr, name_len) = write_data_to_memory(&the_memory, contract_name,owner_memory_offset, contract.get_store())?;
//             let ipfs_memory_offset = name_ptr + name_len;
//             let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, contract_symbol, ipfs_memory_offset,contract.get_store())?;
        
//             let wasm_params = WasmParams {                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    
//                 name: "initialize".to_string(),
//                 args: vec![
//                     Val::I32(name_ptr as i32),
//                     Val::I32(name_len as i32),
//                     Val::I32(symbol_ptr as i32),
//                     Val::I32(symbol_len as i32),                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          
//                 ],
//             };
//             let mut contract = WasmContract::new("./sample721.wasm")?;
//             contract.call_721(&contract_db,&contract_info, &wasm_params)?;
//             let the_item = rock_storage::get_from_db(&contract_db,public_key.to_string());
//             let result = public_txn::Txn::create_and_prepare_transaction(
//                 TransactionType::ContractCreation,
//                 call_address.to_string(),
//                 public_key.to_string(),
//                 1000);
//             println!("Contract Public Key: {:?}",public_key.to_string());
//             // Check the result
//             match result {
//                 Ok((txn_hash_hex, gas_cost, new_txn)) => {
//                     // Do something with the values if needed
//                     // ...

//                     // Return the values from your function
//                     Ok((public_key.to_string(),txn_hash_hex, gas_cost, new_txn))
//                 }
//                 Err(err) => {
//                     // Handle the error if necessary
//                     Err(err)
//                 }
//             }

//             // Process the_item as needed
//         }
//         Err(e) => {
//             // Handle the error appropriately
//             eprintln!("Failed to open contract database: {:?}", e);
//             Err(e.into())
//         }
//     }
// }
pub fn create_contract_official(
    call_address: &str,
    private_key: &str,
    contract_name: &str,
    contract_symbol: &str,
    base64_wasm_data: &str, // Add base64-encoded Wasm data parameter
) -> Result<(String, String,u128), Box<dyn std::error::Error>> {
    let (public_key, private_key) = generate_keypair();
    let path = "./contract/db";
    let contract_path = rock_storage::open_db(path);

    match contract_path {
        Ok(contract_db) => {
            // Generate a unique contract address or use an existing one1
            // Create a Wasm file with the contract address as the file name
            let wasm_file_name = format!("{}.wasm", public_key);
            let wasm_file_path = format!("./{}", wasm_file_name);

            // Decode base64-encoded Wasm data and write it to the file
            let wasm_data = base64::decode(base64_wasm_data)
                .map_err(|e| format!("Error decoding base64-encoded Wasm data: {}", e))?;
            let mut wasm_file = File::create(&wasm_file_path)
                .map_err(|e| format!("Error creating Wasm file: {}", e))?;
            wasm_file
                .write_all(&wasm_data)
                .map_err(|e| format!("Error writing Wasm data to file: {}", e))?;

            // Rest of your function code...
            let contract_info = ContractInfo {
                module_path: wasm_file_path.clone(), // Use the file path
                pub_key: public_key.to_string(),
            };
            let mut contract = WasmContract::new(&wasm_file_path)?;

            // let functions = contract.exported_functions();
            // for func_name in functions {
            //     println!("Exported Function: {}", func_name);
            // }
            let the_memory = create_memory(contract.get_store())?;
            let owner_memory_offset = 0;
            let (name_ptr, name_len) = write_data_to_memory(&the_memory, contract_name,owner_memory_offset, contract.get_store())?;
            let ipfs_memory_offset = name_ptr + name_len;
            let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, contract_symbol, ipfs_memory_offset,contract.get_store())?;
        
            let wasm_params = WasmParams {                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    
                name: "initialize".to_string(),
                args: vec![
                    Val::I32(name_ptr as i32),
                    Val::I32(name_len as i32),
                    Val::I32(symbol_ptr as i32),
                    Val::I32(symbol_len as i32),                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          
                ],
            };
            let _  = gas_calculator::calculate_gas_for_contract_creation(&wasm_file_path); // Use the file path

            let result = public_txn::Txn::create_and_prepare_transaction(
                TransactionType::ContractCreation,
                call_address.to_string(),
                public_key.to_string(),
                1000);
            // Return the values from your function
            match result {
                Ok((txn_hash, gas_cost,body)) => {
                    let result = contract.create_contract(&contract_db,&contract_info, &wasm_params)?;
                    fs::remove_file(&wasm_file_path)
                    .map_err(|e| format!("Failed to delete Wasm file: {:?}", e))?;            
                    return Ok((public_key.to_string(),txn_hash, gas_cost));
                }
                Err(err) => {
                    // Handle the error if necessary
                    Err(err)
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open contract database: {:?}", e);
            Err(e.into())
        }
    }
}

pub fn read_total_token_erc721(contract_address:&String)->Result<i64, Box<dyn std::error::Error>>{
    let c_path = "./contract/db";
    let a_path = "./account/db";
    let contract_path = match rock_storage::open_db(c_path) {
        Ok(path) => path,
        Err(e) => {
            // Handle the error, maybe log it, and then decide what to do next
            panic!("Failed to open database: {:?}", e); // or use some default value or error handling logic
        }
    };
    let acc_path = match rock_storage::open_db(a_path) {
        Ok(path) => path,
        Err(e) => {
            // Handle the error, maybe log it, and then decide what to do next
            panic!("Failed to open database: {:?}", e); // or use some default value or error handling logic
        }
    };
    let mut contract = WasmContract::new("./sample721.wasm")?;
    let contract_info = ContractInfo {
        module_path: "./sample721.wasm".to_string(),
        pub_key:contract_address.to_string(),
    };
    let result = contract.read_numbers(&contract_path,&contract_info,contract_address,"total_tokens");
    match result {
        Ok(value) => {
            Ok(value)
        }
        Err(e) => {
            Err(e)
        }
    }
}

pub fn mint_token_official(contract_address:&String,
                            account_key:&String,
                            private_key:&String,
                            ipfs:&String)->Result<(i32,String, u128), Box<dyn std::error::Error>>{
        let c_path = "./contract/db";
        let contract_path = match rock_storage::open_db(c_path) {
            Ok(path) => path,
            Err(e) => {
                // Handle the error, maybe log it, and then decide what to do next
                panic!("Failed to open database: {:?}", e); // or use some default value or error handling logic
            }
        };
        let mut contract = WasmContract::new("./sample721.wasm")?;
        let contract_info = ContractInfo {
            module_path: "./sample721.wasm".to_string(),
            pub_key:contract_address.to_string(),
        };
        let the_memory = create_memory(contract.get_store())?;
        // Retrieve the account to check if it exists
        // println!("Acc Key {:?}",account_key);
        // let account_data = rock_storage::get_from_db(&acc_path, account_key);
        // println!("Acc Data {:?}",account_data);
        // if account_data.is_none() {
        //     return Err("Account not found".into());
        // }
        let txn = public_txn::Txn::create_and_prepare_transaction(
            TransactionType::ContractInteraction,
            account_key.to_string(),
            contract_address.to_string(),
            1000);
        match txn {
            Ok((txn_hash,gas_cost,new_txn)) => {
                let private_key_bytes = match hex::decode(&private_key) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        panic!("Failed to decode private key: {:?}", e);
                    }
                };
                // Attempt to create a SecretKey from the decoded bytes
                let the_official_private_key = match SecretKey::from_slice(&private_key_bytes) {
                    Ok(key) => key,
                    Err(e) => {
                        panic!("Failed to create SecretKey: {:?}", e);
                    }
                };
                let _ = public_txn::Txn::sign_and_submit_transaction(account_key,txn_hash.clone(),&the_official_private_key);
                let result = contract.mint_token(&contract_path, &contract_info,account_key,&contract_address.to_string(),ipfs);
                match result {
                    Ok(token_id) => {
                        println!("Mint: {}", token_id);
                        return Ok((token_id, txn_hash.clone(), gas_cost));
                    }
                    Err(e) => {
                        println!("Error minting token: {}", e);
                        Err(e)
                    }
                }
            }
            Err(txn_err) => {
                println!("Error creating transaction: {}", txn_err);
                // Handle the error case for transaction creation.
                // You might want to perform cleanup or other actions.
                Err(txn_err.into())
            }
        }
        // let read_result = contract.read_owner_token(contract_path, &contract_info,&contract_address.to_string(),token_id);
        // if let Err(e) = read_result {
        //     println!("Error after minting, could not read token owner: {}", e);
        //     return Err(e);
        // }    
    }
    
pub fn read_contract(contract_address: &String) -> Result<PublicSmartContract, Box<dyn std::error::Error>> {
        let c_path = "./contract/db";
        let contract_path = rock_storage::open_db(c_path)
            .map_err(|e| format!("Failed to open database: {:?}", e))?;
    
        let serialized_contract = rock_storage::get_from_db_vector(&contract_path, contract_address)
            .ok_or_else(|| format!("Contract not found for address: {:?}", contract_address))?;
    
        if serialized_contract.is_empty() {
            return Err("Contract data is empty".into());
        }
    
        let contract: PublicSmartContract = serde_json::from_slice(&serialized_contract)
            .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;
    
        Ok(contract)
    }
    
pub fn read_token_by_id(contract_address:&String,id:&i32)->Result<(String,String), Box<dyn std::error::Error>>{
    let c_path = "./contract/db";
    let a_path = "./account/db";
    let contract_path = match rock_storage::open_db(c_path) {
        Ok(path) => path,
        Err(e) => {
            // Handle the error, maybe log it, and then decide what to do next
            panic!("Failed to open database: {:?}", e); // or use some default value or error handling logic
        }
    };
    let contract = WasmContract::new("./sample721.wasm")?;
    let contract_info = ContractInfo {
        module_path: "./sample721.wasm".to_string(),
        pub_key:contract_address.to_string(),
    };
    let read_result = contract.read_owner_token(&contract_path, &contract_info,&contract_address.to_string(),*id);
    match read_result {
        Ok(read_items) => {
            let read_ipfs = match contract.read_ipfs_token(&contract_path, &contract_info, &contract_address.to_string(), *id) {
                Ok(ipfs_data) => ipfs_data,
                Err(e) => {
                    println!("Error reading IPFS data: {}", e);
                    "default value or error info".to_string() // Provide a default value or error information
                }
            };
            Ok((read_items, read_ipfs)) // Use `read_items` here instead of `read_item`
        }
        Err(e) => {
            println!("Error after minting, could not read token owner: {}", e);
            Err(e.into())
        }
    }
}


pub fn call_contract_function(
    contract_address: &String,
    account_key: &String,
    private_key: &String,
    function_name: &String,
    args_input_values:&String,
    args_input:  &String,
    args_output: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    let c_path = "./contract/db";
    let a_path = "./account/db";
    let contract_path = match rock_storage::open_db(c_path) {
        Ok(path) => path,
        Err(e) => {
            panic!("Failed to open database: {:?}", e);
        }
    };

    let mut contract = WasmContract::new("./sample721.wasm")?;
    let contract_info = ContractInfo {
        module_path: "./sample721.wasm".to_string(),
        pub_key: contract_address.to_string(),
    };
    let the_memory = create_memory(contract.get_store())?;
    let _account_data = match account::get_account_no_swarm(&account_key) {
        Ok(Some(acc)) => acc,
        Ok(None) => return Err("Account not found".into()), // or handle this case as needed
        Err(e) => return Err(e.into()), // error while fetching account
    };
    if(args_input.len()>0){
        let txn = public_txn::Txn::create_and_prepare_transaction(
            TransactionType::ContractInteraction,
            account_key.to_string(),
            contract_address.to_string(),
            1000,
        );
        match txn {
            Ok((txn_hash, _gas_cost, _new_txn)) => {
                let private_key_bytes = match hex::decode(&private_key) {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        panic!("Failed to decode private key: {:?}", e);
                    }
                };
    
                let the_official_private_key = match SecretKey::from_slice(&private_key_bytes) {
                    Ok(key) => key,
                    Err(e) => {
                        panic!("Failed to create SecretKey: {:?}", e);
                    }
                };
    
                let _ = public_txn::Txn::sign_and_submit_transaction(account_key, txn_hash.clone(), &the_official_private_key);
    
                let result = contract.call_function(
                    &contract_path,
                    &contract_info,
                    account_key,
                    &contract_address.to_string(),
                    function_name,
                    args_input_values,
                    args_input,
                    args_output
                );
    
                match result {
                    Ok(result_map) => {
                        println!("Function {} result: {:?}", function_name, result_map);
                        Ok(())
                    }
                    Err(e) => {
                        println!("Error calling function {}: {}", function_name, e);
                        Err(e)
                    }
                }
            }
            Err(txn_err) => {
                println!("Error creating transaction: {}", txn_err);
                Err(txn_err.into())
            }
        }
    }else{
        let result = contract.call_function(
            &contract_path,
            &contract_info,
            account_key,
            &contract_address.to_string(),
            function_name,
            args_input_values,
            args_input,
            args_output
        );

        match result {
            Ok(result_map) => {
                println!("Function {} result: {:?}", function_name, result_map);
                Ok(())
            }
            Err(e) => {
                println!("Error calling function {}: {}", function_name, e);
                Err(e)
            }
        }
    }



}