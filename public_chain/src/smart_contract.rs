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
use wasmtime::*;
use wasmtime_wasi::WasiCtx;
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
    u8,
    u64,
    str,
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

    pub fn call<T, R>(&mut self, func_name: &str, args: T) -> Result<R, Box<dyn std::error::Error>>
    where
        T: wasmtime::WasmTy,
        R: wasmtime::WasmTy,
    {
        let func: TypedFunc<T, R> = self.instance.get_typed_func(&mut self.store, func_name)?;
        Ok(func.call(&mut self.store, args)?)
    }
    pub fn dynamic_call(&mut self, descriptor: &FunctionDescriptor, params: Vec<i64>) -> Result<i64, Box<dyn std::error::Error>> {
        match descriptor.param_types.len() {
            0 => {
                let func: TypedFunc<(), i64> = self.instance.get_typed_func(&mut self.store, &descriptor.name)?;
                Ok(func.call(&mut self.store, ())?)
            },
            1 => {
                let func: TypedFunc<i64, i64> = self.instance.get_typed_func(&mut self.store, &descriptor.name)?;
                Ok(func.call(&mut self.store, params[0])?)
            },
            2 => {
                let func: TypedFunc<(i64, i64), i64> = self.instance.get_typed_func(&mut self.store, &descriptor.name)?;
                Ok(func.call(&mut self.store, (params[0], params[1]))?)
            },
            _ => Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unsupported number of parameters"))),
        }
    }
    pub fn exported_functions(&self) -> Vec<String> {
        self.module.exports().map(|export| export.name().to_string()).collect()
    }
}
pub fn read_wasm_file(module_path: &str) -> Result<(), Box<dyn std::error::Error>>{
    match WasmContract::new(module_path){
        Ok(contract) => {
            println!("Contract successfully created.");
            println!("Successfully instantiated the wasm module.");
            // Print the exported functions
            let functions = contract.exported_functions();
            println!("Available Functions: {:?}", functions);
            for func in functions.iter() {
                println!("Exported Function: {}", func);
            }

            // let create_test_token = FunctionDescriptor {
            //     name: "initialize".to_string(),
            //     param_types: vec![WasmType::str, WasmType::str, WasmType::u8, WasmType::u64],
            //     return_type: None,
            // };
            // println!("{:?}",create_test_token);
            // let result = contract.dynamic_call(
            //     &create_test_token,
            //     vec![
            //         "SUMO_TOKEN".to_string(),
            //         "SMTX".to_string(),
            //         18,
            //         1000
            //     ]
            // );
            // match result {
            //     Ok(val) => println!("Function returned: {}", val),
            //     Err(e) => eprintln!("Error calling function: {}", e),
            // }
        },
        Err(e) => {
            eprintln!("Error creating contract: {}", e);
        }
    };
    // match result {
    //     Ok(contract) => println!("Successfully created contract!"),
    //     Err(e) => eprintln!("Error: {}", e),
    // }

  // Example: Let's say you want to allow users to interactively call these functions
    // loop {
    //     println!("Which function do you want to call? (Type 'exit' to quit)");

    //     let mut input = String::new();
    //     std::io::stdin().read_line(&mut input)?;
    //     let input = input.trim();

    //     if input == "exit" {
    //         break;
    //     }

    //     if !functions.contains(&input.to_string()) {
    //         println!("Function not found!");
    //         continue;
    //     }

    //     // Here, you would introspect the type of the function 
    //     // and prompt the user for the right number and type of arguments.
    //     // This is a bit complex since Wasm function introspection is not straightforward.

    //     // For demonstration purposes, we're just showing how you'd call 'balance_of'
    //     if input == "balance_of" {
    //         // This is a hardcoded example, in a real-world scenario, you'd dynamically
    //         // determine the number and type of arguments based on the function signature.
    //         println!("Enter the owner address:");
    //         let mut owner = String::new();
    //         std::io::stdin().read_line(&mut owner)?;
    //         let owner = owner.trim();

    //         // Calling the function
    //         let balance: u64 = contract.call("balance_of", owner)?;
    //         println!("Balance of {}: {}", owner, balance);
    //     } else {
    //         println!("Function calling for {} is not implemented in this demo.", input);
    //     }
    // }
    // Here's how you can call the functions
    // let balance: u64 = contract.call("balance_of", "creator")?;
    // println!("Creator's balance: {}", balance);

    // Similarly, you can call other functions

    Ok(())


}

    // let engine = Engine::default();
    // let module = Module::from_file(&engine, "./sample.wasm").unwrap();
    // let mut exports = module.exports();
    // while let Some(foo) = exports.next() {
    //     println!("Functions: {}", foo.name());
    // }
    // let mut linker = Linker::new(&engine);
    // let wasi = WasiCtxBuilder::new()
    //     .inherit_stdio()
    //     .inherit_args().unwrap()
    //     .build();
    // let mut store = Store::new(&engine, wasi);
    // //wasmtime_wasi::add_to_linker(&mut linker, |s| s).unwrap();
    // let link = linker.instantiate(&mut store, &module).unwrap();
    // let mut contract = module::new("./sample.wasm");
    // contract.new_token(1000);
    // let balance = contract.balance_of("creator");
    // println!("Creator's balance: {}", balance);
    //let add_fn = link.get_typed_func::<(u32, u32), u32>(&mut store, "add").unwrap();
    //let add: wasmtime::TypedFunc<(u32, u32), u32> = link.get_typed_func(&mut store, "add").unwrap();
    //let result = add.call(&mut store, (1, 2)).unwrap();
    //println!("{:?}",result);

    // let engine = wasmtime::Engine::default();
    // let store = wasmtime::Store::new(&engine);
    // let module = wasmtime::Module::from_file(&store, "Users/leowyennhan/Desktop/sumotex_mainnet/chain/public_chain/cool.wasm")?;
    // let instance = wasmtime::Instance::new(&store, &module, &[host_function.into()]).expect("Failed to create wasmtime instance");
    // let set_data = instance.get_func("set_data").expect("function not found");
    // let args = [wasmtime::Val::from(data_to_store)]; // your data as appropriate type
    // let results = set_data.call(&args)?;
    // let host_function = wasmtime::Func::wrap(store, |caller: Caller<'_>, arg: i32| {
    //     // Your host function logic here.
    // });
    // //let instance = wasmtime::Instance::new(&module, &[host_function.into()]);
    // println!("{:?}",host_function);   