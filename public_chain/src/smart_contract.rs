use std::collections::HashMap;
use libp2p::{
    swarm::{Swarm},
};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::Read;
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
    wasm_memory:Vec<u8>,
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
    pub fn new(public_key:String,wasm_file:Vec<u8>, wasm_memory: Vec<u8>) -> Self {
        PublicSmartContract {
            contract_address:public_key.to_string(),
            balance: 0.0,
            nonce: 0,
            wasm_file,
            wasm_memory,
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
    pub fn new(pub_key: &str,contract_path: &DBWithThreadMode<SingleThreaded>) -> Result<Self, Box<dyn std::error::Error>> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
        let serialized_contract = rock_storage::get_from_db_vector(&contract_path, pub_key).unwrap_or_default();
        let mut contract: PublicSmartContract = serde_json::from_slice(&serialized_contract[..])
            .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;
        let module = Module::new(&engine, &contract.wasm_file)
        .map_err(|e| format!("Failed to create WASM module from binary data: {:?}", e))?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
        println!("Attempting to instantiate the WebAssembly module...");
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        // ... (Your operations using token_instance)
    
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        let instance = linker.instantiate(&mut store, &module)?;
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
        let saved_data = contract.wasm_memory;
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
        contract.wasm_memory=updated_byte_vector;
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
        let mut file = File::open(contract_info.module_path.clone())?;
        let mut wasm_contents = Vec::new();
        file.read_to_end(&mut wasm_contents)?;
        let contract = PublicSmartContract::new(contract_info.pub_key.clone(),wasm_contents,updated_byte_vector);
        let serialized_contract = serde_json::to_vec(&contract)?;
        rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &serialized_contract)?;
    
        Ok(())
    }
    
    
    pub fn read(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        pub_key: &str,
        function_name: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
        
        let serialized_contract = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
        let mut contract: PublicSmartContract = serde_json::from_slice(&serialized_contract[..])
            .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;
        let module = Module::new(&engine, &contract.wasm_file)
        .map_err(|e| format!("Failed to create WASM module from binary data: {:?}", e))?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);

        println!("Attempting to instantiate the WebAssembly module...");
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
        
        let serialized_contract = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
        let mut contract: PublicSmartContract = serde_json::from_slice(&serialized_contract[..])
            .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;
        let module = Module::new(&engine, &contract.wasm_file)
        .map_err(|e| format!("Failed to create WASM module from binary data: {:?}", e))?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);

        println!("Attempting to instantiate the WebAssembly module...");
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
    pub fn mint_token_dynamic(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        owner:&str,
        pub_key: &str,
        owner_creds:&str,
        owner_name:&str,
        owner_email:&str,
        ipfs_hash: &str
    ) -> Result<i32, Box<dyn std::error::Error>> {
        println!("Initializing engine and linker...");
        println!("Contract name and pub key {:?}",pub_key);
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let serialized_contract = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
        let mut contract: PublicSmartContract = serde_json::from_slice(&serialized_contract[..])
            .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;

        // let mut file = File::create(output_file_path)?;
        // file.write_all(&wasm_contents)?;
        let module = Module::new(&engine, &contract.wasm_file)
        .map_err(|e| format!("Failed to create WASM module from binary data: {:?}", e))?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);

        println!("Attempting to instantiate the WebAssembly module...");
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        let instance = linker.instantiate(&mut store, &module)?;
        // ... (Your operations using token_instance)
    
        println!("Fetching WebAssembly memory...");
        //let wasm_memory = contract.instance.get_memory(&mut store,"memory").ok_or_else(|| "Failed to find `memory` export")?;
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        let saved_data = contract.wasm_memory;
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
        // Print the exported functions
        for export in module.exports() {
            if let ExternType::Func(func_type) = export.ty() {
                println!("Exported function: {} with type {:?}", export.name(), func_type);
            }
        }
        
        // After ensuring the memory is large enough, copy the saved data into the WebAssembly memory within bounds
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data);

        let mint_func = link.get_typed_func::<(i32, i32, i32,i32,i32,i32,i32,i32,i32,i32), u32>(&mut store, "mint")?;


        let new_data_offset = saved_data.len() as i32;
        let owner_data_bytes = owner.as_bytes().len() as i32;
        let ipfs_data_bytes = ipfs_hash.as_bytes().len() as i32;
        let owner_email_data_bytes = owner_email.as_bytes().len() as i32; 
        let owner_name_data_bytes = owner_name.as_bytes().len() as i32; 
        let owner_creds_data_bytes = owner_creds.as_bytes().len() as i32; 
        let required_memory_size_bytes = 
            new_data_offset + 
            owner_data_bytes + 
            ipfs_data_bytes + 
            owner_email_data_bytes +
            owner_name_data_bytes +
            owner_creds_data_bytes ;      
        let mut current_memory_size_bytes = wasm_memory.data_size(&store) as i32; // Size in bytes
        println!("New data offset: {:?}", new_data_offset);
        println!("Owner data bytes: {:?}", owner_data_bytes);
        println!("Owner Email bytes: {:?}", owner_email_data_bytes);
        println!("Owner Name bytes: {:?}", owner_name_data_bytes);
        println!("Owner Creds bytes: {:?}", owner_creds_data_bytes);
        println!("IPFS data bytes: {:?}", ipfs_data_bytes);
        println!("Required memory size bytes: {:?}", required_memory_size_bytes);
        println!("Current memory size bytes: {:?}", current_memory_size_bytes);
        // Check if we need more memory pages and grow memory if needed
        while required_memory_size_bytes > current_memory_size_bytes {
            // Calculate how many more pages are needed, rounding up
            let additional_pages_needed = ((required_memory_size_bytes - current_memory_size_bytes + (64 * 1024 - 1)) / (64 * 1024)) as u32;
        
            // Attempt to grow the memory
            if wasm_memory.grow(&mut store, additional_pages_needed as u64).is_err() {
                // Handle the error if memory growth fails
                return Err("Failed to grow memory".into());
            }
            // Update the current memory size
            current_memory_size_bytes = wasm_memory.data_size(&mut store) as i32;
        }
        println!("New memory size bytes: {:?}", current_memory_size_bytes);
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

        let owner_email_memory_offset =  ipfs_ptr+ipfs_len;
        // Check that the IPFS data fits within memory bounds
        {
            let memory_view = wasm_memory.data_mut(&mut store);
            if owner_email_memory_offset as usize + owner_email_data_bytes as usize > memory_view.len() {
                return Err("OWNER ID data offset or length is out of the current memory bounds.".into());
            }
        } // The scope of memory_view ends here
        
        let (owner_email_ptr,owner_email_len)= write_data_to_memory(&wasm_memory,owner_email,owner_email_memory_offset, &mut store)?;

        let owner_name_memory_offset =  owner_email_ptr+owner_email_len;
        // Check that the IPFS data fits within memory bounds
        {
            let memory_view = wasm_memory.data_mut(&mut store);
            if owner_name_memory_offset as usize + owner_name_data_bytes as usize > memory_view.len() {
                return Err("owner name data offset or length is out of the current memory bounds.".into());
            }
        } // The scope of memory_view ends here
        
        let (owner_name_ptr,owner_name_len)= write_data_to_memory(&wasm_memory,owner_name,owner_name_memory_offset, &mut store)?;

        let owner_creds_memory_offset =  owner_name_ptr+owner_name_len;
        // Check that the IPFS data fits within memory bounds
        {
            let memory_view = wasm_memory.data_mut(&mut store);
            if owner_creds_memory_offset as usize + owner_creds_data_bytes as usize > memory_view.len() {
                return Err("owner name data offset or length is out of the current memory bounds.".into());
            }
        } // The scope of memory_view ends here
        
        let (owner_creds_ptr,owner_creds_len)= write_data_to_memory(&wasm_memory,owner_creds,owner_creds_memory_offset, &mut store)?;

        let mint_result = mint_func.call(&mut store, (
            name_ptr as i32,
            name_len as i32,
            owner_creds_ptr as i32,
            owner_creds_len as i32,
            owner_name_ptr as i32,
            owner_name_len as i32,
            owner_email_ptr as i32,
            owner_email_len as i32,
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
                contract.nonce+=1;
                contract.wasm_memory = wasm_memory.data(&mut store).to_vec(); // Update this only if the WASM memory state is relevant
                let updated_serialized_contract = serde_json::to_vec(&contract)?;
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
    }
    pub fn call_721(
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

        //rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &byte_vector)?;
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
        );   
        let initialise_func = link.get_typed_func::<(i32, i32, i32, i32),()>(&mut store, &wasm_params.name)?;

        let result = initialise_func.call(&mut store,  args_tuple)?;
        //println!("Initialize: {:?}",result);
        let updated_data = wasm_memory.data(&mut store);
        let updated_byte_vector: Vec<u8> = updated_data.to_vec();
        let mut file = File::open(contract_info.module_path.clone())?;
        let mut wasm_contents = Vec::new();
        file.read_to_end(&mut wasm_contents)?;
        let contract = PublicSmartContract::new(contract_info.pub_key.clone(),wasm_contents,updated_byte_vector);
        let serialized_contract = serde_json::to_vec(&contract)?;
        rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &serialized_contract)?;
        Ok(())
    } 
    pub fn read_ipfs_token(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        pub_key: &str,
        id: i32,
    ) -> Result<String, Box<dyn std::error::Error>> {
        println!("Contract name and pub key {:?}",pub_key);
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
        let serialized_contract = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
        let mut contract: PublicSmartContract = serde_json::from_slice(&serialized_contract[..])
            .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;
        let module = Module::new(&engine, &contract.wasm_file)
        .map_err(|e| format!("Failed to create WASM module from binary data: {:?}", e))?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);

        println!("Attempting to instantiate the WebAssembly module...");
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
    let mut contract = WasmContract::new(module_path,path)?;

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
pub fn create_erc721_contract_official(call_address:&str,private_key:&str,contract_name:&str,contract_symbol:&str)->
    Result<(String,String,u128, PublicTxn), Box<dyn std::error::Error>>{
    let (public_key,private_key) = generate_keypair(); 
    let path = "./contract/db";
    let contract_path = rock_storage::open_db(path);
    match contract_path {
        Ok(contract_db) => {
            let contract_info = ContractInfo {
                module_path: "./sample721.wasm".to_string(),
                pub_key:public_key.to_string(),
            };
            //TODO
            let the_gas_cost = match gas_calculator::calculate_gas_for_contract_creation(&[]) {
                Ok(gas_cost) => gas_cost as u128, // Convert u64 to u128
                Err(e) => {
                    // Handle the error, maybe log it and/or return from the function
                    return Err(e.into());
                }
            };
            let mut contract = WasmContract::new("./sample721.wasm",&contract_db)?;
            let functions = contract.exported_functions();
        
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
            let mut contract = WasmContract::new("./sample721.wasm",&contract_db)?;
            contract.call_721(&contract_db,&contract_info, &wasm_params)?;
            let result = public_txn::Txn::create_and_prepare_transaction(
                TransactionType::ContractCreation,
                call_address.to_string(),
                public_key.to_string(),
                the_gas_cost);
            println!("Contract Public Key: {:?}",public_key.to_string());
            // Check the result
            match result {
                Ok((txn_hash_hex, gas_cost, new_txn)) => {
                    // Do something with the values if needed
                    // ...

                    // Return the values from your function
                    Ok((public_key.to_string(),txn_hash_hex, gas_cost, new_txn))
                }
                Err(err) => {
                    // Handle the error if necessary
                    Err(err)
                }
            }

            // Process the_item as needed
        }
        Err(e) => {
            // Handle the error appropriately
            eprintln!("Failed to open contract database: {:?}", e);
            Err(e.into())
        }
    }
}
pub fn create_contract_official(
    call_address: &str,
    private_key: &str,
    contract_name: &str,
    contract_symbol: &str,
    base64_wasm_data: &str, // Add base64-encoded Wasm data parameter
) -> Result<(String, String,u128), Box<dyn std::error::Error>> {
    let (public_key,_) = generate_keypair();
    let path = "./contract/db";
    let contract_path = rock_storage::open_db(path);
    println!("contract path");
    match contract_path {
        Ok(contract_db) => {
            // Decode base64-encoded Wasm data and write it to the file
            let wasm_data = base64::decode(base64_wasm_data)
            .map_err(|e| format!("Error decoding base64-encoded Wasm data: {}", e))?;
           
            let engine = Engine::default();
            let mut linker = Linker::new(&engine);
            let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();


            let module = Module::new(&engine, &wasm_data)
            .map_err(|e| format!("Failed to create WASM module from binary data: {:?}", e))?;
            let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
            let mut store = Store::new(&engine, wasi);
            wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
            let link = linker.instantiate(&mut store, &module)?;


            for export in module.exports() {
                if let ExternType::Func(func_type) = export.ty() {
                    println!("Function Name: {}", export.name());
                    // If you want to print the function signature as well, you can do so here.
                    // This is optional and can be complex because you'll need to interpret
                    // the types (e.g., parameters and return types).
                    println!("Function Signature: {:?}", func_type);
                }
            }
            println!("Contract");
            let the_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
            let owner_memory_offset = 0;
            let (name_ptr, name_len) = write_data_to_memory(&the_memory, contract_name,owner_memory_offset,&mut store)?;
            let ipfs_memory_offset = name_ptr + name_len;
            let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, contract_symbol, ipfs_memory_offset,&mut store)?;
        
            let wasm_params = WasmParams {                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    
                name: "initialize".to_string(),
                args: vec![
                    Val::I32(name_ptr as i32),
                    Val::I32(name_len as i32),
                    Val::I32(symbol_ptr as i32),
                    Val::I32(symbol_len as i32),                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          
                ],
            };
            let the_gas_cost = match gas_calculator::calculate_gas_for_contract_creation(&wasm_data) {
                Ok(gas_cost) => gas_cost as u128, // Convert u64 to u128
                Err(e) => {
                    return Err(e.into());
                }
            };
            println!("Gas Fee: {:?}",the_gas_cost);
            let result = public_txn::Txn::create_and_prepare_transaction(
                TransactionType::ContractCreation,
                call_address.to_string(),
                public_key.to_string(),
                the_gas_cost);
            // Return the values from your function
            match result {
                Ok((txn_hash, gas_cost,body)) => {
                
                    let data = the_memory.data(&mut store);
                    let byte_vector: Vec<u8> = data.to_vec();
                
                    //rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &byte_vector)?;
                
                    // Dynamically calling a function based on the input string
                    let dyn_func = link.get_func(&mut store, "initialize")
                        .ok_or_else(|| format!("Failed to find function `{}`", "initialize"))?;
                
                    let args: Vec<Val> = wasm_params.args.iter().cloned().collect();
                
                    // Prepare a mutable slice for the results if the function returns values
                    let mut results = vec![Val::I32(0); dyn_func.ty(&store).results().len()]; // Adjust the size based on expected results
                
                    dyn_func.call(&mut store, &args[..], &mut results)?;
                
                    let updated_data = the_memory.data(&mut store);
                    let updated_byte_vector: Vec<u8> = updated_data.to_vec();
                    let contract = PublicSmartContract::new(public_key.to_string(),wasm_data,updated_byte_vector);
                    let serialized_contract = serde_json::to_vec(&contract)?;
                    rock_storage::put_to_db(&contract_db, &public_key.to_string(), &serialized_contract)?;
                    //fs::remove_file(&wasm_file_path)
                    //.map_err(|e| format!("Failed to delete Wasm file: {:?}", e))?;            
                    return Ok((public_key.to_string(),txn_hash, gas_cost));
                }
                Err(err) => {
                    println!("{:?}",err);
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
    let mut contract = WasmContract::new("./sample721.wasm",&contract_path)?;
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
    owner_address:&String,
                            private_key:&String,
                            owner_creds:&String,
                            owner_name:&String,
                            owner_email:&String,
                            ipfs_hash:&String)->Result<(i32,String, u128), Box<dyn std::error::Error>>{
        let c_path = "./contract/db";
        let contract_path = match rock_storage::open_db(c_path) {
            Ok(path) => path,
            Err(e) => {
                // Handle the error, maybe log it, and then decide what to do next
                panic!("Failed to open database: {:?}", e); // or use some default value or error handling logic
            }
        };  
        let mut contract = WasmContract::new(contract_address,&contract_path)?;
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
        let serialized_contract = rock_storage::get_from_db_vector(&contract_path, contract_address).unwrap_or_default();
        let mut contract: PublicSmartContract = serde_json::from_slice(&serialized_contract[..])
            .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;
        let module_result = Module::new(&engine, &contract.wasm_file);

        // Handle the result to extract the Module or return an error
        let module = match module_result {
            Ok(module) => module,
            Err(e) => return Err(e.into()), // Convert the error and return if there's an issue creating the Module
        };
        

        let the_gas_cost = match gas_calculator::calculate_gas_for_contract_creation(&contract.wasm_file) {
            Ok(gas_cost) => gas_cost as u128, // Convert u64 to u128
            Err(e) => {
                return Err(e.into());
            }
        };

        println!("Attempting to instantiate the WebAssembly module...");
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        let instance = linker.instantiate(&mut store, &module)?;
        // ... (Your operations using token_instance)
    
        println!("Fetching WebAssembly memory...");
        //let wasm_memory = contract.instance.get_memory(&mut store,"memory").ok_or_else(|| "Failed to find `memory` export")?;
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        let saved_data = contract.wasm_memory;
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
        // Print the exported functions
        // for export in module.exports() {
        //     if let ExternType::Func(func_type) = export.ty() {
        //         println!("Exported function: {} with type {:?}", export.name(), func_type);
        //     }
        // }
        
        // After ensuring the memory is large enough, copy the saved data into the WebAssembly memory within bounds
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data);

        let mint_func = link.get_typed_func::<(i32, i32, i32,i32,i32,i32,i32,i32,i32,i32), u32>(&mut store, "mint")?;


        let new_data_offset = saved_data.len() as i32;
        let owner_data_bytes = owner_address.as_bytes().len() as i32;
        let ipfs_data_bytes = ipfs_hash.as_bytes().len() as i32;
        let owner_email_data_bytes = owner_email.as_bytes().len() as i32; 
        let owner_name_data_bytes = owner_name.as_bytes().len() as i32; 
        let owner_creds_data_bytes = owner_creds.as_bytes().len() as i32; 
        let required_memory_size_bytes = 
            new_data_offset + 
            owner_data_bytes + 
            ipfs_data_bytes + 
            owner_email_data_bytes +
            owner_name_data_bytes +
            owner_creds_data_bytes ;      
        let mut current_memory_size_bytes = wasm_memory.data_size(&store) as i32; // Size in bytes
        println!("New data offset: {:?}", new_data_offset);
        println!("Owner data bytes: {:?}", owner_data_bytes);
        println!("Owner Email bytes: {:?}", owner_email_data_bytes);
        println!("Owner Name bytes: {:?}", owner_name_data_bytes);
        println!("Owner Creds bytes: {:?}", owner_creds_data_bytes);
        println!("IPFS data bytes: {:?}", ipfs_data_bytes);
        println!("Required memory size bytes: {:?}", required_memory_size_bytes);
        println!("Current memory size bytes: {:?}", current_memory_size_bytes);
        // Check if we need more memory pages and grow memory if needed
        while required_memory_size_bytes > current_memory_size_bytes {
            // Calculate how many more pages are needed, rounding up
            let additional_pages_needed = ((required_memory_size_bytes - current_memory_size_bytes + (64 * 1024 - 1)) / (64 * 1024)) as u32;
        
            // Attempt to grow the memory
            if wasm_memory.grow(&mut store, additional_pages_needed as u64).is_err() {
                // Handle the error if memory growth fails
                return Err("Failed to grow memory".into());
            }
            // Update the current memory size
            current_memory_size_bytes = wasm_memory.data_size(&mut store) as i32;
        }
        println!("New memory size bytes: {:?}", current_memory_size_bytes);
        // We need to limit the scope of the memory view so that we can mutably borrow `store` again later
        {
            let memory_view = wasm_memory.data_mut(&mut store);
            // Ensure the new data offset is within the bounds of the current memory size
            if new_data_offset as usize >= memory_view.len() {
                return Err("New data offset is out of the current memory bounds.".into());
            }
        } // `memory_view` goes out of scope here, so we can mutably borrow `store` again


        let (name_ptr, name_len) = write_data_to_memory(&wasm_memory, owner_address, new_data_offset, &mut store)?;
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

        let owner_email_memory_offset =  ipfs_ptr+ipfs_len;
        // Check that the IPFS data fits within memory bounds
        {
            let memory_view = wasm_memory.data_mut(&mut store);
            if owner_email_memory_offset as usize + owner_email_data_bytes as usize > memory_view.len() {
                return Err("OWNER ID data offset or length is out of the current memory bounds.".into());
            }
        } // The scope of memory_view ends here
        
        let (owner_email_ptr,owner_email_len)= write_data_to_memory(&wasm_memory,owner_email,owner_email_memory_offset, &mut store)?;

        let owner_name_memory_offset =  owner_email_ptr+owner_email_len;
        // Check that the IPFS data fits within memory bounds
        {
            let memory_view = wasm_memory.data_mut(&mut store);
            if owner_name_memory_offset as usize + owner_name_data_bytes as usize > memory_view.len() {
                return Err("owner name data offset or length is out of the current memory bounds.".into());
            }
        } // The scope of memory_view ends here
        
        let (owner_name_ptr,owner_name_len)= write_data_to_memory(&wasm_memory,owner_name,owner_name_memory_offset, &mut store)?;

        let owner_creds_memory_offset =  owner_name_ptr+owner_name_len;
        // Check that the IPFS data fits within memory bounds
        {
            let memory_view = wasm_memory.data_mut(&mut store);
            if owner_creds_memory_offset as usize + owner_creds_data_bytes as usize > memory_view.len() {
                return Err("owner name data offset or length is out of the current memory bounds.".into());
            }
        } // The scope of memory_view ends here
        
        let (owner_creds_ptr,owner_creds_len)= write_data_to_memory(&wasm_memory,owner_creds,owner_creds_memory_offset, &mut store)?;

        let mint_result = mint_func.call(&mut store, (
            name_ptr as i32,
            name_len as i32,
            owner_creds_ptr as i32,
            owner_creds_len as i32,
            owner_name_ptr as i32,
            owner_name_len as i32,
            owner_email_ptr as i32,
            owner_email_len as i32,
            ipfs_ptr as i32,
            ipfs_len as i32,
        ));
        match mint_result {
            Ok(token_id) => {
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
                let txn = public_txn::Txn::create_and_prepare_transaction(
                    TransactionType::ContractInteraction,
                    owner_address.to_string(),
                    contract_address.to_string(),
                    the_gas_cost);
                match txn {
                    Ok((txn_hash,gas_cost,new_txn))=>{
                        let _ = public_txn::Txn::sign_and_submit_transaction(owner_address,txn_hash.clone(),&the_official_private_key);
                                println!("Mint: {}", token_id);
                                let updated_data = wasm_memory.data(&mut store);
                                let updated_byte_vector: Vec<u8> = updated_data.to_vec();
                                if saved_data == updated_data.to_vec() {
                                    println!("No change in the WebAssembly memory after mint operation.");
                                } else {
                                    println!("WebAssembly memory updated after mint operation.");
                                }
                                contract.nonce+=1;
                                contract.wasm_memory = wasm_memory.data(&mut store).to_vec(); // Update this only if the WASM memory state is relevant
                                let updated_serialized_contract = serde_json::to_vec(&contract)?;
                                rock_storage::put_to_db(&contract_path, &contract_address, &updated_serialized_contract)?;        
                                println!("Token ID value: {:?}", token_id);
                                //Ok(token_id);
                                return Ok((token_id as i32, txn_hash.clone(), the_gas_cost));
                            }
                    Err(e) => {
                        // Handle the error, for example, by logging or panicking
                        panic!("Failed to serialize transaction: {}", e);
                        return Err(e.into());
                    }
                }

            },
            Err(e) => {
                println!("Mint function failed: {}", e);
                // handle the error
                Err(e.into())
            }
        }
    }
    
// pub fn read_token_by_id(contract_address:&String,id:&i32)->Result<(String,String), Box<dyn std::error::Error>>{
//     let c_path = "./contract/db";
//     let a_path = "./account/db";
//     let contract_path = match rock_storage::open_db(c_path) {
//         Ok(path) => path,
//         Err(e) => {
//             // Handle the error, maybe log it, and then decide what to do next
//             panic!("Failed to open database: {:?}", e); // or use some default value or error handling logic
//         }
//     };
//     let contract = WasmContract::new("./sample721.wasm")?;
//     let contract_info = ContractInfo {
//         module_path: "./sample721.wasm".to_string(),
//         pub_key:contract_address.to_string(),
//     };
//     let read_result = contract.read_owner_token(&contract_path, &contract_info,&contract_address.to_string(),*id);
//     match read_result {
//         Ok(read_items) => {
//             let read_ipfs = match contract.read_ipfs_token(&contract_path, &contract_info, &contract_address.to_string(), *id) {
//                 Ok(ipfs_data) => ipfs_data,
//                 Err(e) => {
//                     println!("Error reading IPFS data: {}", e);
//                     "default value or error info".to_string() // Provide a default value or error information
//                 }
//             };
//             Ok((read_items, read_ipfs)) // Use `read_items` here instead of `read_item`
//         }
//         Err(e) => {
//             println!("Error after minting, could not read token owner: {}", e);
//             Err(e.into())
//         }
//     }
// }
pub fn read_id(contract_address:&String,id:&i32)->Result<(String,String,String,String,String), Box<dyn std::error::Error>>{
    let c_path = "./contract/db";
    let contract_path = match rock_storage::open_db(c_path) {
        Ok(path) => path,
        Err(e) => {
            // Handle the error, maybe log it, and then decide what to do next
            panic!("Failed to open database: {:?}", e); // or use some default value or error handling logic
        }
    };
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
    let mut store = Store::new(&engine, wasi);
    let serialized_contract = rock_storage::get_from_db_vector(&contract_path, contract_address).unwrap_or_default();
    let mut contract: PublicSmartContract = serde_json::from_slice(&serialized_contract[..])
        .map_err(|e| format!("Failed to deserialize contract: {:?}", e))?;
    let module = Module::new(&engine, &contract.wasm_file)
    .map_err(|e| format!("Failed to create WASM module from binary data: {:?}", e))?;
    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
    let mut store = Store::new(&engine, wasi);
    // Assuming `module` is successfully created as shown in your provided code.
    // for export in module.exports() {
    //     if let ExternType::Func(func_type) = export.ty() {
    //         println!("Function Name: {}", export.name());
    //         // If you want to print the function signature as well, you can do so here.
    //         // This is optional and can be complex because you'll need to interpret
    //         // the types (e.g., parameters and return types).
    //         println!("Function Signature: {:?}", func_type);
    //     }
    // }

    println!("Attempting to instantiate the WebAssembly module...");
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
    let link = linker.instantiate(&mut store, &module)?;
    // ... (Your operations using token_instance)

    let wasm_memory = link.get_memory(&mut store, "memory")
        .ok_or_else(|| "Failed to find `memory` export")?;
    let saved_data = contract.wasm_memory;
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
    // // Assuming the memory content is ASCII text
    // if let Ok(text) = std::str::from_utf8(&memory_data) {
    //     println!("Memory Contents: {}", text);
    // } else {
    //     // If the memory contains binary data, you can print it as bytes
    //     println!("Memory Contents (as bytes)");
    // }
    let ipfs_ptr_call = link.get_typed_func::<i32,i32>(&mut store, "get_ipfs_ptr")?;
    let ipfs_ptr_result = ipfs_ptr_call.call(&mut store,  *id)
        .map_err(|e| {
            println!("Error Reading IPFS ptr data: {:?}", e);
            Box::<dyn std::error::Error>::from(e) // Convert the error to a Box<dyn std::error::Error>
        })?;
    let ipfs_len_call= link.get_typed_func::<i32,i32>(&mut store, "get_ipfs_len")?;
    let ipfs_len_result = ipfs_len_call.call(&mut store, *id)
        .map_err(|e| {
            println!("Error IPFS len data: {:?}", e);
            Box::<dyn std::error::Error>::from(e) // Convert the error to a Box<dyn std::error::Error>
        })?;

    let get_owner_len_call = link.get_typed_func::<i32, i64>(&mut store, "get_owner_len")?;
    let owner_len_result = get_owner_len_call.call(&mut store, *id)
        .map_err(|e| {
            println!("Error calling get_owner_len: {:?}", e);
            Box::<dyn std::error::Error>::from(e)
        })?;
    let get_owner_ptr_call = link.get_typed_func::<i32, i32>(&mut store, "get_owner_ptr")?;
    let owner_ptr_result = get_owner_ptr_call.call(&mut store, *id)
        .map_err(|e| {
            println!("Error calling get_owner_ptr: {:?}", e);
            Box::<dyn std::error::Error>::from(e)
        })?; 

    let get_owner_email_len_call = link.get_typed_func::<i32, i32>(&mut store, "get_owner_email_len")?;
    let owner_email_len_result = get_owner_email_len_call.call(&mut store, *id)
        .map_err(|e| {
            println!("Error calling get_owner_email_len_call: {:?}", e);
            Box::<dyn std::error::Error>::from(e)
        })?;
    let get_owner_email_ptr_call = link.get_typed_func::<i32, i32>(&mut store, "get_owner_email_ptr")?;
    let owner_email_ptr_result = get_owner_email_ptr_call.call(&mut store, *id)
        .map_err(|e| {
            println!("Error calling get_owner_email_ptr_call: {:?}", e);
            Box::<dyn std::error::Error>::from(e)
        })?;
    let get_owner_creds_len_call = link.get_typed_func::<i32, i32>(&mut store, "get_owner_creds_len")?;
    let owner_creds_len_result = get_owner_creds_len_call.call(&mut store, *id)
        .map_err(|e| {
            println!("Error calling get_owner_creds_len_call: {:?}", e);
            Box::<dyn std::error::Error>::from(e)
        })?;
    let get_owner_creds_ptr_call = link.get_typed_func::<i32, i32>(&mut store, "get_owner_creds_ptr")?;
    let owner_creds_ptr_result = get_owner_creds_ptr_call.call(&mut store, *id)
        .map_err(|e| {
            println!("Error calling get_owner_creds_len_call: {:?}", e);
            Box::<dyn std::error::Error>::from(e)
        })?;
    let get_owner_name_len_call = link.get_typed_func::<i32, i32>(&mut store, "get_owner_name_len")?;
    let owner_name_len_result = get_owner_name_len_call.call(&mut store, *id)
        .map_err(|e| {
            println!("Error calling get_owner_name_len_call: {:?}", e);
            Box::<dyn std::error::Error>::from(e)
        })?;
    let get_owner_name_ptr_call = link.get_typed_func::<i32, i32>(&mut store, "get_owner_name_ptr")?;
    let owner_name_ptr_result = get_owner_name_ptr_call.call(&mut store, *id)
        .map_err(|e| {
            println!("Error calling name: {:?}", e);
            Box::<dyn std::error::Error>::from(e)
        })?;
    //Second section
    let ipfs_start = ipfs_ptr_result as usize;
    let ipfs_end = ipfs_start + ipfs_len_result as usize;
    let ipfs_data_bytes = wasm_memory.data(&store)[ipfs_start..ipfs_end].to_vec();
    let ipfs_data_string = String::from_utf8(ipfs_data_bytes)
        .map_err(|e| {
            println!("Error converting ipfs data bytes to String: {:?}", e);
            Box::<dyn std::error::Error>::from(e) as Box<dyn std::error::Error> // Explicitly cast the error
        })?;
    // Use owner_ptr_result and owner_len_result to access the owner data in Wasm memory
    let owner_data_start = owner_ptr_result as usize;
    let owner_data_end = owner_data_start + owner_len_result as usize;
    let owner_data_bytes = wasm_memory.data(&store)[owner_data_start..owner_data_end].to_vec();
    let owner_data_string = String::from_utf8(owner_data_bytes)
        .map_err(|e| {
            println!("Error converting owner data bytes to String: {:?}", e);
            Box::<dyn std::error::Error>::from(e) as Box<dyn std::error::Error> // Explicitly cast the error
        })?;

    
    // Do similar for `get_owner_of_ptr` and `get_owner_of_len` if needed
    let owner_email_data_start = owner_email_ptr_result as usize;
    let owner_email_data_end = owner_email_data_start + owner_email_len_result as usize;
    let owner_email_data_bytes = wasm_memory.data(&store)[owner_email_data_start..owner_email_data_end].to_vec();
    let owner_email_data_string = String::from_utf8(owner_email_data_bytes)
        .map_err(|e| {
            println!("Error converting owner data bytes to String: {:?}", e);
            Box::<dyn std::error::Error>::from(e) as Box<dyn std::error::Error> // Explicitly cast the error
        })?;

    //Name section
    let owner_name_data_start = owner_name_ptr_result as usize;
    let owner_name_data_end = owner_name_data_start + owner_name_len_result as usize;
    let owner_name_data_bytes = wasm_memory.data(&store)[owner_name_data_start..owner_name_data_end].to_vec();
    let owner_name_data_string = String::from_utf8(owner_name_data_bytes)
        .map_err(|e| {
            println!("Error converting owner name bytes to String: {:?}", e);
            Box::<dyn std::error::Error>::from(e) as Box<dyn std::error::Error> // Explicitly cast the error
        })?;

    //Creds section
    let owner_creds_data_start = owner_creds_ptr_result as usize;
    let owner_creds_data_end = owner_creds_data_start + owner_creds_len_result as usize;
    let owner_creds_data_bytes = wasm_memory.data(&store)[owner_creds_data_start..owner_creds_data_end].to_vec();
    let owner_creds_data_string = String::from_utf8(owner_creds_data_bytes)
        .map_err(|e| {
            println!("Error converting owner creds bytes to String: {:?}", e);
            Box::<dyn std::error::Error>::from(e) as Box<dyn std::error::Error> // Explicitly cast the error
        })?;
    
    // Finally, return the collected data
    Ok((owner_data_string, ipfs_data_string,owner_name_data_string,owner_creds_data_string, owner_email_data_string))

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

    let mut contract = WasmContract::new("./sample721.wasm",&contract_path)?;
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
    if args_input.len()>0 {
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
