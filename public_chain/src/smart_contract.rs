use std::collections::HashMap;
use libp2p::{
    swarm::{Swarm},
};
use rocksdb::{DB,Error,DBWithThreadMode,SingleThreaded};
use serde::{Deserialize, Serialize};
use crate::p2p::AppBehaviour;
use secp256k1::{Secp256k1, PublicKey, SecretKey};
use crate::rock_storage;
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
    pub fn get_store(&mut self) -> &mut Store<WasiCtx> {
        &mut self.store
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

        //rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &byte_vector)?;
        let args_tuple = (
            wasm_params.args[0].unwrap_i32(),
            wasm_params.args[1].unwrap_i32(),
            wasm_params.args[2].unwrap_i32(),
            wasm_params.args[3].unwrap_i32(),
            wasm_params.args[4].unwrap_i32(),
            wasm_params.args[5].unwrap_i64(),
        );
        let initialise_func = link.get_typed_func::<(i32, i32, i32, i32, i32, i64), ()>(&mut store, &wasm_params.name)?;
        let result = initialise_func.call(&mut store,  args_tuple)?;
        println!("Initialize: {:?}",result);
        let updated_data = wasm_memory.data(&mut store);
        let updated_byte_vector: Vec<u8> = updated_data.to_vec();
        rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &updated_byte_vector)?;
        Ok(())
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
            wasm_params.args[0].unwrap_i32(),
            wasm_params.args[1].unwrap_i32(),
            wasm_params.args[2].unwrap_i32(),
            wasm_params.args[3].unwrap_i32(),
        );
        let initialise_func = link.get_typed_func::<(i32, i32, i32, i32),()>(&mut store, &wasm_params.name)?;

        let result = initialise_func.call(&mut store,  args_tuple)?;
        println!("Initialize: {:?}",result);
        let updated_data = wasm_memory.data(&mut store);
        let updated_byte_vector: Vec<u8> = updated_data.to_vec();
        rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &updated_byte_vector)?;
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
        
        // 1. Instantiate the WebAssembly module.
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        
        // 2. Get the WebAssembly memory.
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        let saved_data = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
        // 2.2. Set this memory state into the WebAssembly module's memory.
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data);
    
        // 3. Call the desired function in the WebAssembly module by its name.
        let result = link.get_typed_func::<_, i64>(&mut store, function_name)?
        .call(&mut store, ())?;
    
        Ok(result)
    }
    pub fn mint_token(
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
        
        // 1. Instantiate the WebAssembly module.
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        
        // 2. Get the WebAssembly memory.
        let wasm_memory = link.get_memory(&mut store, "memory")
            .ok_or_else(|| "Failed to find `memory` export")?;
        let saved_data = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
        // 2.2. Set this memory state into the WebAssembly module's memory.
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data);
    
        // 3. Call the desired function in the WebAssembly module by its name.
        let result = link.get_typed_func::<_, i64>(&mut store, function_name)?
        .call(&mut store, ())?;
    
        Ok(result)
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
pub fn write_data_to_memory(memory: &Memory, input: &str, store: &mut Store<WasiCtx>) -> Result<(i32, i32), Box<dyn std::error::Error>> {
    let input_bytes = input.as_bytes();
    let data = memory.data_mut(store); // Use the store here

    for (i, byte) in input_bytes.iter().enumerate() {
        data[i] = *byte;
    }

    Ok((0, input_bytes.len() as i32)) // Placeholder for data_ptr, replace 0 with the actual pointer if possible
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
    let functions = contract.exported_functions();
    println!("Available Functions: {:?}", functions);
    for func in functions.iter() {
        println!("Exported Function: {}", func);
    }

    let the_memory = create_memory(contract.get_store())?;

    let (name_ptr, name_len) = write_data_to_memory(&the_memory, "SUMOTEX-T", contract.get_store())?;
    let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, "SMTX", contract.get_store())?;

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

pub fn create_erc721_contract(cmd:&str,swarm:  &mut Swarm<AppBehaviour>)->Result<(), Box<dyn std::error::Error>>{
    let contract_path = swarm.behaviour().storage_path.get_contract();
    let (public_key,private_key) = generate_keypair();
    read_wasm_file("./sample721.wasm",contract_path,public_key.to_string());
    // Reading from DB and deserializing
    let contract_info = ContractInfo {
        module_path: "./sample721.wasm".to_string(),
        pub_key:public_key.to_string(),
    };
    let mut contract = WasmContract::new("./sample721.wasm")?;

    println!("Contract successfully created.");
    println!("Successfully instantiated the wasm module.");

    // Print the exported functions
    let functions = contract.exported_functions();
    println!("Available Functions: {:?}", functions);
    for func in functions.iter() {
        println!("Exported Function: {}", func);
    }

    let the_memory = create_memory(contract.get_store())?;
    let (name_ptr, name_len) = write_data_to_memory(&the_memory, "SUMOTEX-CERT", contract.get_store())?;
    let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, "SMTX", contract.get_store())?;

    let wasm_params = WasmParams {
        name: "initialize".to_string(),
        args: vec![
            Val::I32(name_ptr as i32),
            Val::I32(name_len as i32),
            Val::I32(symbol_ptr as i32),
            Val::I32(symbol_len as i32),
        ],
        // ... other params as needed.
    };
    //Initialise
    let mut contract = WasmContract::new("./sample721.wasm")?;
    contract.call_721(contract_path,&contract_info, &wasm_params)?;
    let the_item = rock_storage::get_from_db(contract_path,public_key.to_string());
    println!("Contract Public Key: {:?}",public_key.to_string());
    println!("The Key Item: {:?}",the_item);
    Ok(())
}
pub fn create_erc20_contract(cmd:&str,swarm:  &mut Swarm<AppBehaviour>)->Result<(), Box<dyn std::error::Error>>{
    let contract_path = swarm.behaviour().storage_path.get_contract();
    let (public_key,private_key) = generate_keypair();
    read_wasm_file("./sample.wasm",contract_path,public_key.to_string());
    // Reading from DB and deserializing
    let contract_info = ContractInfo {
        module_path: "./sample.wasm".to_string(),
        pub_key:public_key.to_string(),
    };
    let mut contract = WasmContract::new("./sample.wasm")?;

    println!("Contract successfully created.");
    println!("Successfully instantiated the wasm module.");

    // Print the exported functions
    let functions = contract.exported_functions();
    println!("Available Functions: {:?}", functions);
    for func in functions.iter() {
        println!("Exported Function: {}", func);
    }

    let the_memory = create_memory(contract.get_store())?;
    let (name_ptr, name_len) = write_data_to_memory(&the_memory, "SUMOTEX-T", contract.get_store())?;
    let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, "SMTX", contract.get_store())?;

    let wasm_params = WasmParams {
        name: "initialize".to_string(),
        args: vec![
            Val::I32(name_ptr as i32),
            Val::I32(name_len as i32),
            Val::I32(symbol_ptr as i32),
            Val::I32(symbol_len as i32),
            Val::I32(8),
            Val::I64(1000000),
        ],
        // ... other params as needed.
    };
    //Initialise
    let mut contract = WasmContract::new("./sample.wasm")?;
    contract.call(contract_path,&contract_info, &wasm_params)?;
    let the_item = rock_storage::get_from_db(contract_path,public_key.to_string());
    println!("Contract Public Key: {:?}",public_key.to_string());
    println!("The Key Item: {:?}",the_item);
    Ok(())
}
pub fn get_erc20_supply(cmd:&str,swarm:  &mut Swarm<AppBehaviour>)->Result<(), Box<dyn std::error::Error>>{
    if let Some(data) = cmd.strip_prefix("contract key ") {
        let mut contract = WasmContract::new("./sample.wasm")?;
        let contract_path = swarm.behaviour().storage_path.get_contract();
        // Reading from DB and deserializing
        let contract_info = ContractInfo {
            module_path: "./sample.wasm".to_string(),
            pub_key:data.to_string(),
        };
        let result = contract.read_numbers(contract_path,&contract_info,&data.to_string(),"total_supply");
        match result {
            Ok(value) => {
                println!("Total Supply: {}", value);
            }
            Err(e) => {
                println!("Error reading: {}", e);
            }
        }
    }
    Ok(())
}
pub fn mint_token(cmd:&str,swarm:  &mut Swarm<AppBehaviour>)->Result<(), Box<dyn std::error::Error>>{
    if let Some(data) = cmd.strip_prefix("mint token ") {
        let mut contract = WasmContract::new("./sample721.wasm")?;
        let contract_path = swarm.behaviour().storage_path.get_contract();
        // Reading from DB and deserializing
        let contract_info = ContractInfo {
            module_path: "./sample721.wasm".to_string(),
            pub_key:data.to_string(),
        };
        let result = contract.mint_token(contract_path,&contract_info,&data.to_string(),"mint");
        match result {
            Ok(value) => {
                println!("Total Supply: {}", value);
            }
            Err(e) => {
                println!("Error reading: {}", e);
            }
        }
    }
    Ok(())
}