use std::collections::HashMap;
use libp2p::{
    swarm::{Swarm},
};
use rocksdb::{DBWithThreadMode,SingleThreaded};
use serde::{Deserialize, Serialize};
use crate::p2p::AppBehaviour;
use secp256k1::{Secp256k1, PublicKey, SecretKey};
use crate::rock_storage;
use std::time::UNIX_EPOCH;
use std::time::SystemTime;
use wasmtime::*;
use wasmtime::Val;
use wasmtime_wasi::WasiCtx;
use wasmtime::MemoryType;
use wasmtime::Linker;
use wasmtime::component::Type;
use wasmtime_wasi::sync::WasiCtxBuilder;
use bincode::{serialize, deserialize};
use bincode::{ Error as BincodeError};
use rocksdb::Error as RocksDBError;
use wasm_bindgen::JsCast;
use crate::public_txn;
use crate::rock_storage::StoragePath;
use crate::public_swarm;

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
struct PublicSmartContract {
    contract_address: String,
    balance: f64,
    nonce: u64,
    timestamp:u64,
}
#[repr(C)]
#[derive(Debug)]
pub struct OwnerData {
    ptr: i32,
    len: i32,
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
    fn bytes_to_token_details(data: &[u8]) -> Result<TokenDetails, Box<dyn std::error::Error>> {
        Ok(bincode::deserialize(data)?)
    }
    pub fn transfer_nft_token(&self, 
        contract_path: &DBWithThreadMode<SingleThreaded>, 
        contract_info: &ContractInfo, 
        token_id: i32) -> Result<String, Box<dyn std::error::Error>>
    {   
        let function_name = "transfer";
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
    
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        let wasm_memory = link.get_memory(&mut store, "memory")
        .ok_or_else(|| "Failed to find `memory` export")?;
        let saved_data = rock_storage::get_from_db_vector(contract_path, &contract_info.pub_key).unwrap_or_default();
      
        // 2.2. Set this memory state into the WebAssembly module's memory.
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data);  
        let empty = Self::is_memory_empty(&wasm_memory, &store);
        println!("Memory is empty: {}", empty);   
        // Convert token_id to expected format (assuming u64 for simplicity here)
        // Assuming `link` is of type `wasmtime::Link`
        let result = link.get_typed_func::<i32, i64>(&mut store, function_name)?.call(&mut store, token_id)?;
        println!("RESULT OF TOKEN: {:?}",result);
        let result_to_unpack =Self::decode_token_details(result, &wasm_memory, &mut store)?;
        println!("Unpack result {:?}", result_to_unpack);
        // let result_str = result.to_string();
        // println!("{:?}", result_str);
        // let result_to_u8 = Self::i64_to_u8_array(result);
        // let the_result = Self::decode_token_details(&result_to_u8);
        Ok("".to_string())
    }
    pub fn read_token(&self, 
        contract_path: &DBWithThreadMode<SingleThreaded>, 
        contract_info: &ContractInfo, 
        token_id: i32) -> Result<String, Box<dyn std::error::Error>>
    {   
        let function_name = "read_token";
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
        let mut store = Store::new(&engine, wasi);
    
        let module = Module::from_file(&engine, &contract_info.module_path)?;
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let link = linker.instantiate(&mut store, &module)?;
        let wasm_memory = link.get_memory(&mut store, "memory")
        .ok_or_else(|| "Failed to find `memory` export")?;
        let saved_data = rock_storage::get_from_db_vector(contract_path, &contract_info.pub_key).unwrap_or_default();
      
        // 2.2. Set this memory state into the WebAssembly module's memory.
        wasm_memory.data_mut(&mut store)[..saved_data.len()].copy_from_slice(&saved_data);  
        let empty = Self::is_memory_empty(&wasm_memory, &store);
        println!("Memory is empty: {}", empty);   
        // Convert token_id to expected format (assuming u64 for simplicity here)
        // Assuming `link` is of type `wasmtime::Link`
        let result = link.get_typed_func::<i32, i64>(&mut store, function_name)?.call(&mut store, token_id)?;
        println!("RESULT OF TOKEN: {:?}",result);
        let result_to_unpack =Self::decode_token_details(result, &wasm_memory, &mut store)?;
        println!("Unpack result {:?}", result_to_unpack);
        // let result_str = result.to_string();
        // println!("{:?}", result_str);
        // let result_to_u8 = Self::i64_to_u8_array(result);
        // let the_result = Self::decode_token_details(&result_to_u8);
        Ok("".to_string())
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
        let saved_data = rock_storage::get_from_db_vector(contract_path, pub_key).unwrap_or_default();
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
        let ipfs_data_bytes = "SMTX_IPFS_TEST".as_bytes().len() as i32;
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
        let (ipfs_ptr, ipfs_len) = write_data_to_memory(&wasm_memory, "SMTX_IPFS_TEST", ipfs_memory_offset, &mut store)?;

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
                rock_storage::put_to_db(&contract_path, &contract_info.pub_key, &updated_byte_vector)?;
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
    ) -> Result<u32, Box<dyn std::error::Error>> {
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
                                            // Other processing code here
                                        },
                                        Err(e) => {
                                            println!("Error converting bytes to String: {:?}", e);
                                            // Handle the error appropriately
                                        },
                                    }
                                } else {
                                    println!("Invalid memory access: Out of bounds");
                                    // Handle the out-of-bounds memory access appropriately
                                }
                                
                                // Other processing code here
                            },
                            Err(e) => {
                                println!("Failed to call get_owner_len_by_token_id function: {:?}", e);
                            },
                        }
                    },
                    Err(e) => {
                        println!("Failed to call get_owner_len_by_token_id function: {:?}", e);
                    },
                }
            },
            Err(e) => {
                println!("Failed to call get_owner_ptr function: {:?}", e);
            },
        }        
    Ok(1)
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
    pub fn get_erc20_name(cmd: &str, swarm: &mut Swarm<AppBehaviour>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(data) = cmd.strip_prefix("contract name ") {
            let contract = WasmContract::new("./sample.wasm")?;
            let contract_path = swarm.behaviour().storage_path.get_contract();
    
            let contract_info = ContractInfo {
                module_path: "./sample.wasm".to_string(),
                pub_key: data.to_string(),
            };
    
            let name = contract.read_name(contract_path, &contract_info, &data.to_string())?;
    
            println!("Contract Name: {}", name);
        }
        Ok(())
    }
    pub fn read_name(
        &self,
        contract_path: &DBWithThreadMode<SingleThreaded>,
        contract_info: &ContractInfo,
        pub_key: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Call the 'read_symbol' function using the same mechanism
        let result_bytes = self.read(contract_path, contract_info, pub_key, "read_name")?;

        // Convert the result bytes into a string
        let result_string = String::from_utf8(result_bytes)?;

        Ok(result_string)
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
pub fn create_erc721_contract_official(call_address:&str,private_key:&str)->Result<String, Box<dyn std::error::Error>>{
    let (public_key,private_key) = generate_keypair(); 
    let path = "./contract/db";
    let contract_path = rock_storage::open_db(path);
    match contract_path {
        Ok(contract_db) => {
            let contract_info = ContractInfo {
                module_path: "./sample721.wasm".to_string(),
                pub_key:public_key.to_string(),
            };
        
            let mut contract = WasmContract::new("./sample721.wasm")?;

            println!("Contract successfully created.");
            println!("Successfully instantiated the wasm module.");
        
            let functions = contract.exported_functions();
        
            let the_memory = create_memory(contract.get_store())?;
            let owner_memory_offset = 0;
            let (name_ptr, name_len) = write_data_to_memory(&the_memory, "SUMOTEX-CERT",owner_memory_offset, contract.get_store())?;
            let ipfs_memory_offset = name_ptr + name_len;
            let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, "SMTX", ipfs_memory_offset,contract.get_store())?;
        
            let wasm_params = WasmParams {                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    
                name: "initialize".to_string(),
                args: vec![
                    Val::I32(name_ptr as i32),
                    Val::I32(name_len as i32),
                    Val::I32(symbol_ptr as i32),
                    Val::I32(symbol_len as i32),                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          
                ],
            };
            let mut contract = WasmContract::new("./sample721.wasm")?;
            contract.call_721(&contract_db,&contract_info, &wasm_params)?;
            let the_item = rock_storage::get_from_db(&contract_db,public_key.to_string());
            //TODO: compute value
            public_txn::Txn::create_transactions(call_address.to_string(),&private_key,public_key.to_string(),1000);
            println!("Contract Public Key: {:?}",public_key.to_string());
            println!("The Key Item: {:?}",the_item);
            Ok(public_key.to_string())
            // Process the_item as needed
        }
        Err(e) => {
            // Handle the error appropriately
            eprintln!("Failed to open contract database: {:?}", e);
            Err(e.into())
        }
    }
}

//TODO:: DELETE, why? Because its a mock key
pub fn create_erc721_contract(cmd:&str,swarm:  &mut Swarm<AppBehaviour>)->Result<(), Box<dyn std::error::Error>>{
    let contract_path = swarm.behaviour().storage_path.get_contract();
    let (public_key,private_key) = generate_keypair();
    //TODO: Call with signature sign
    let (public_caller_key,private_caller_key) = generate_keypair();
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
    // println!("Available Functions: {:?}", functions);
    // for func in functions.iter() {
    //     println!("Exported Function: {}", func);
    // }

    let the_memory = create_memory(contract.get_store())?;
    let owner_memory_offset = 0;
    let (name_ptr, name_len) = write_data_to_memory(&the_memory, "SUMOTEX-CERT",owner_memory_offset, contract.get_store())?;
    let ipfs_memory_offset = name_ptr + name_len;
    let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, "SMTX", ipfs_memory_offset,contract.get_store())?;

    let wasm_params = WasmParams {                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    
        name: "initialize".to_string(),
        args: vec![
            Val::I32(name_ptr as i32),
            Val::I32(name_len as i32),
            Val::I32(symbol_ptr as i32),
            Val::I32(symbol_len as i32),                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          
        ],
    };
    let mut contract = WasmContract::new("./sample721.wasm")?;
    contract.call_721(contract_path,&contract_info, &wasm_params)?;
    let the_item = rock_storage::get_from_db(contract_path,public_key.to_string());
    //TODO: compute value
    //public_txn::Txn::create_transactions(public_caller_key.to_string(),public_key.to_string(),1000);
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
    // println!("Available Functions: {:?}", functions);
    // for func in functions.iter() {
    //     println!("Exported Function: {}", func);
    // }

    let the_memory = create_memory(contract.get_store())?;
    let owner_memory_offset = 0;
    let (name_ptr, name_len) = write_data_to_memory(&the_memory, "SUMOTEX-T",owner_memory_offset, contract.get_store())?;
    let ipfs_memory_offset = name_ptr + name_len; 
    let (symbol_ptr, symbol_len) = write_data_to_memory(&the_memory, "SMTX",ipfs_memory_offset, contract.get_store())?;

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
pub fn get_erc721_supply(cmd:&str,swarm:  &mut Swarm<AppBehaviour>)->Result<(), Box<dyn std::error::Error>>{
    if let Some(data) = cmd.strip_prefix("supply 721 ") {
        let mut contract = WasmContract::new("./sample721.wasm")?;
        let contract_path = swarm.behaviour().storage_path.get_contract();
        let contract_info = ContractInfo {
            module_path: "./sample721.wasm".to_string(),
            pub_key:data.to_string(),
        };
        let result = contract.read_numbers(contract_path,&contract_info,&data.to_string(),"total_tokens");
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
pub fn mint_token(cmd:&str,swarm:  &mut Swarm<AppBehaviour>)->Result<i32, Box<dyn std::error::Error>>{
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.len() == 4 && parts[0] == "mint" && parts[1] == "token" {
        let contract_pub_key = parts[2]; // The contract public key
        let account_key = parts[3]; 
        let mut contract = WasmContract::new("./sample721.wasm")?;
        let contract_path = swarm.behaviour().storage_path.get_contract();
        let contract_info = ContractInfo {
            module_path: "./sample721.wasm".to_string(),
            pub_key:contract_pub_key.to_string(),
        };
        let the_memory = create_memory(contract.get_store())?;
            // Retrieve the account to check if it exists
        let acc_path = swarm.behaviour().storage_path.get_account();
        let account_data = rock_storage::get_from_db(acc_path, &account_key);
        if account_data.is_none() {
            return Err("Account not found".into());
        }
        let result = contract.mint_token(contract_path, &contract_info,account_key,&contract_pub_key.to_string(),"TEST_IPFS");
        match result {
            Ok(token_id) => {
                println!("Mint: {}", token_id);
                let read_result = contract.read_owner_token(contract_path, &contract_info,&contract_pub_key.to_string(),token_id);
                if let Err(e) = read_result {
                    println!("Error after minting, could not read token owner: {}", e);
                    return Err(e);
                }

                Ok(token_id)
            }
            Err(e) => {
                println!("Error reading: {}", e);
                Err(e)
            }
        }
    } else {
        // If the command is not properly formatted, return an error
        Err("Command format not recognized".into())
    }
}

pub fn get_token_owner(cmd:&str, swarm: &mut Swarm<AppBehaviour>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(data) = cmd.strip_prefix("token id ") {
        let mut contract = WasmContract::new("./sample721.wasm")?;
        let contract_path = swarm.behaviour().storage_path.get_contract();
        let contract_info = ContractInfo {
            module_path: "./sample721.wasm".to_string(),
            pub_key: data.to_string(),
        };
        let token_id = "1";
        let token_id_u64: i32 = token_id.parse()?;

        let owner = contract.read_token(contract_path, &contract_info, token_id_u64)?;
        println!("Owner of token {}: {}", token_id_u64.clone(), owner);
    }
    Ok(())
}

pub fn transfer_nft(cmd:&str, swarm: &mut Swarm<AppBehaviour>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(data) = cmd.strip_prefix("token id ") {
        let mut contract = WasmContract::new("./sample721.wasm")?;
        let contract_path = swarm.behaviour().storage_path.get_contract();
        let contract_info = ContractInfo {
            module_path: "./sample721.wasm".to_string(),
            pub_key: data.to_string(),
        };
        let token_id =data.to_string();
        let token_id_u64: i32 = token_id.parse()?;

        let owner = contract.transfer_nft_token(contract_path, &contract_info, token_id_u64)?;
        println!("Owner of token {}: {}", token_id_u64.clone(), owner);
    }
    Ok(())
}

