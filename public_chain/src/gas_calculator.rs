// gas_calculator.rs
use wasmparser::{Parser, Payload, Operator};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read; 
use std::fs;
// Define average gas costs for various operations as constants.
// These values are placeholders and should be adjusted based on actual gas costs in your blockchain environment.
const GAS_COST_SIMPLE_TRANSFER: u64 = 21000;
const GAS_COST_CONTRACT_CREATION: u64 = 32000;
const GAS_COST_CONTRACT_INTERACTION: u64 = 45000;
const GAS_COST_TOKEN_TRANSFER: u64 = 50000;
// Add other gas cost constants as needed.
const GAS_PER_BYTE: u64 = 10; // Example cost per byte
const GAS_PER_FUNCTION_CALL: u64 = 100; // Base cost for function call


struct GasCalculator {
    base_cost: u64,
    per_byte_cost: u64,
    per_instruction_cost: u64,
}

impl GasCalculator {
    fn new(base_cost: u64, per_byte_cost: u64, per_instruction_cost: u64) -> Self {
        GasCalculator {
            base_cost,
            per_byte_cost,
            per_instruction_cost,
        }
    }

    fn calculate(&self, memory_size: usize, instructions: u64) -> u64 {
        self.base_cost + (memory_size as u64 * self.per_byte_cost) + (instructions * self.per_instruction_cost)
    }
}
pub fn disassemble_wasm(file_path: &str) ->Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let mut wasm_bytes = Vec::new();
    file.read_to_end(&mut wasm_bytes)?;

    let parser = Parser::new(0);
    let mut opcodes = HashSet::new();

    for payload in parser.parse_all(&wasm_bytes) {
        match payload? {
            Payload::CodeSectionEntry(body) => {
                let operators = body.get_operators_reader()?;
                for operator in operators {
                    let op = operator?;
                    opcodes.insert(format!("{:?}", op));
                }
            },
            _ => {}
        }
    }

    // Print the unique opcodes
    println!("Unique opcodes count: {}", opcodes.len());
    for opcode in &opcodes {
        println!("{}", opcode);
    }


    Ok(())
}
/// Calculate gas for contract creation.
pub fn calculate_gas_for_contract_creation(data: &[u8],function_call: u64) -> u64 {
    let data_size_gas = data.len() as u64; // Example, replace with actual gas calculation
    let function_call_gas = function_call; // Example, replace with actual gas calculation

    let mut gas_used = 0;
    gas_used += data_size_gas;
    gas_used += function_call_gas;

    // Now return the calculated gas
    gas_used
}

/// Calculate gas for interacting with a contract.
pub fn calculate_gas_for_contract_interaction(data: &[u8],function_cost:u64) -> u64 {
    let data_size_gas = data.len() as u64; // Example, replace with actual gas calculation
    let function_call_gas = function_cost; // Example, replace with actual gas calculation

    let mut gas_used = 0;
    gas_used += data_size_gas;
    gas_used += function_call_gas;

    // Now return the calculated gas
    gas_used
}

/// Calculate gas for a token transfer.
pub fn calculate_gas_for_token_transfer() -> u64 {
    GAS_COST_TOKEN_TRANSFER
}