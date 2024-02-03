// gas_calculator.rs
use wasmparser::{Parser, Payload};
use std::collections::{HashSet,HashMap};
use std::fs::File;
use std::io::Read; 
// Define average gas costs for various operations as constants.
// These values are placeholders and should be adjusted based on actual gas costs in your blockchain environment.
const GAS_COST_SIMPLE_TRANSFER: u64 = 210000;
const GAS_COST_CONTRACT_CREATION: u64 = 32000;
const GAS_COST_CONTRACT_INTERACTION: u64 = 45000;
const GAS_COST_TOKEN_TRANSFER: u64 = 50000;
// Add other gas cost constants as needed.
const GAS_PER_BYTE: u64 = 10; // Example cost per byte
const GAS_PER_FUNCTION_CALL: u64 = 100; // Base cost for function call


fn opcode_gas_costs() -> HashMap<String, u64> {
    let mut map = HashMap::new();
    map.insert("CallIndirect".to_string(), 8);
    map.insert("I64Load".to_string(), 4);
    map.insert("I32Const".to_string(), 2);  // Example cost for I32Const
    map.insert("I32Store8".to_string(), 4); // Example cost for I32Store8
    map.insert("I32Load8U".to_string(), 3); // Example cost for I32Load8U
    map.insert("Call".to_string(), 10);     // Example cost for Call
    map.insert("BrIf".to_string(), 5);      // Example cost for BrIf
    map.insert("I32Store16".to_string(), 4); // Example cost for I32Store16
    map.insert("I32Load".to_string(), 3);    // Example cost for I32Load
    map.insert("I32Store".to_string(), 4);   // Example cost for I32Store
    map.insert("I32Load16U".to_string(), 3); // Example cost for I32Load16U
    map.insert("I64Const".to_string(), 2);   // Example cost for I64Const
    map.insert("I64Load8U".to_string(), 3);  // Example cost for I64Load8U
    map.insert("I64Store".to_string(), 4);   // Example cost for I64Store
    map.insert("LocalSet".to_string(), 2);    // Example cost for LocalSet
    map.insert("LocalGet".to_string(), 2);    // Example cost for LocalGet
    map.insert("Br".to_string(), 5);          // Example cost for Br
    map.insert("BrTable".to_string(), 6);     // Example cost for BrTable
    map.insert("I32Add".to_string(), 3);      // Example cost for I32Add
    map.insert("I32Sub".to_string(), 3);      // Example cost for I32Sub
    map.insert("I32Mul".to_string(), 3);      // Example cost for I32Mul
    map.insert("I32DivS".to_string(), 5);     // Example cost for I32DivS
    map.insert("I32DivU".to_string(), 5);     // Example cost for I32DivU
    map.insert("I32RemS".to_string(), 5);     // Example cost for I32RemS
    map.insert("I32RemU".to_string(), 5);     // Example cost for I32RemU
    map.insert("I32And".to_string(), 3);      // Example cost for I32And
    map.insert("I32Or".to_string(), 3);       // Example cost for I32Or
    map.insert("I32Xor".to_string(), 3);      // Example cost for I32Xor
    map.insert("I32Shl".to_string(), 3);      // Example cost for I32Shl
    map.insert("I32ShrS".to_string(), 3);     // Example cost for I32ShrS
    map.insert("I32ShrU".to_string(), 3);     // Example cost for I32ShrU
    map.insert("I32Eqz".to_string(), 2);      // Example cost for I32Eqz
    map.insert("I32Eq".to_string(), 2);       // Example cost for I32Eq
    map.insert("I32Ne".to_string(), 2);       // Example cost for I32Ne
    map.insert("I32LtS".to_string(), 2);      // Example cost for I32LtS
    map.insert("I32LtU".to_string(), 2);      // Example cost for I32LtU
    map.insert("I32GtS".to_string(), 2);      // Example cost for I32GtS
    map.insert("I32GtU".to_string(), 2);      // Example cost for I32GtU
    map.insert("I32LeS".to_string(), 2);      // Example cost for I32LeS
    map.insert("I32LeU".to_string(), 2);      // Example cost for I32LeU
    map.insert("I32GeS".to_string(), 2);      // Example cost for I32GeS
    map.insert("I32GeU".to_string(), 2);      // Example cost for I32GeU
    map.insert("I64Add".to_string(), 3);       // Example cost for I64Add
    map.insert("I64Sub".to_string(), 3);       // Example cost for I64Sub
    map.insert("I64Mul".to_string(), 3);       // Example cost for I64Mul
    map.insert("I64DivS".to_string(), 5);      // Example cost for I64DivS
    map.insert("I64DivU".to_string(), 5);      // Example cost for I64DivU
    map.insert("I64RemS".to_string(), 5);      // Example cost for I64RemS
    map.insert("I64RemU".to_string(), 5);      // Example cost for I64RemU
    map.insert("I64And".to_string(), 3);       // Example cost for I64And
    map.insert("I64Or".to_string(), 3);        // Example cost for I64Or
    map.insert("I64Xor".to_string(), 3);       // Example cost for I64Xor
    map.insert("I64Shl".to_string(), 3);       // Example cost for I64Shl
    map.insert("I64ShrS".to_string(), 3);      // Example cost for I64ShrS
    map.insert("I64ShrU".to_string(), 3);      // Example cost for I64ShrU
    map.insert("I64Eqz".to_string(), 2);       // Example cost for I64Eqz
    map.insert("I64Eq".to_string(), 2);        // Example cost for I64Eq
    map.insert("I64Ne".to_string(), 2);        // Example cost for I64Ne
    map.insert("I64LtS".to_string(), 2);       // Example cost for I64LtS
    map.insert("I64LtU".to_string(), 2);       // Example cost for I64LtU
    map.insert("I64GtS".to_string(), 2);       // Example cost for I64GtS
    map.insert("I64GtU".to_string(), 2);       // Example cost for I64GtU
    map.insert("I64LeS".to_string(), 2);       // Example cost for I64LeS
    map.insert("I64LeU".to_string(), 2);       // Example cost for I64LeU
    map.insert("I64GeS".to_string(), 2);       // Example cost for I64GeS
    map.insert("I64GeU".to_string(), 2);       // Example cost for I64GeU

    // Control flow operations
    map.insert("Unreachable".to_string(), 1);
    map.insert("Nop".to_string(), 1);
    map.insert("Block".to_string(), 2);
    map.insert("Loop".to_string(), 2);
    map.insert("If".to_string(), 2);
    map.insert("Else".to_string(), 1);
    map.insert("End".to_string(), 1);
    map.insert("Br".to_string(), 5);
    map.insert("BrIf".to_string(), 5);
    map.insert("BrTable".to_string(), 6);
    map.insert("Return".to_string(), 5);
    map.insert("Call".to_string(), 10);
    map.insert("CallIndirect".to_string(), 8);

    // Local variables
    map.insert("LocalGet".to_string(), 2);
    map.insert("LocalSet".to_string(), 2);
    map.insert("LocalTee".to_string(), 2);

    // Global variables
    map.insert("GlobalGet".to_string(), 3);
    map.insert("GlobalSet".to_string(), 3);

    // Memory operations
    map.insert("I32Load".to_string(), 3);
    map.insert("I64Load".to_string(), 4);
    map.insert("F32Load".to_string(), 3);
    map.insert("F64Load".to_string(), 4);
    map.insert("I32Load8S".to_string(), 3);
    map.insert("I32Load8U".to_string(), 3);
    map.insert("I32Load16S".to_string(), 3);
    map.insert("I32Load16U".to_string(), 3);
    map.insert("I64Load8S".to_string(), 4);
    map.insert("I64Load8U".to_string(), 4);
    map.insert("I64Load16S".to_string(), 4);
    map.insert("I64Load16U".to_string(), 4);
    map.insert("I64Load32S".to_string(), 4);
    map.insert("I64Load32U".to_string(), 4);
    map.insert("I32Store".to_string(), 4);
    map.insert("I64Store".to_string(), 5);
    map.insert("F32Store".to_string(), 4);
    map.insert("F64Store".to_string(), 5);
    map.insert("I32Store8".to_string(), 4);
    map.insert("I32Store16".to_string(), 4);
    map.insert("I64Store8".to_string(), 5);
    map.insert("I64Store16".to_string(), 5);
    map.insert("I64Store32".to_string(), 5);
    map.insert("MemorySize".to_string(), 2);
    map.insert("MemoryGrow".to_string(), 10);

    // Numeric instructions
    map.insert("I32Const".to_string(), 2);
    map.insert("I64Const".to_string(), 2);
    map.insert("F32Const".to_string(), 2);
    map.insert("F64Const".to_string(), 2);
    map.insert("I32Eqz".to_string(), 2);
    map.insert("I32Eq".to_string(), 2);
    // Conversion operations
    map.insert("I32WrapI64".to_string(), 2);
    map.insert("I32TruncF32S".to_string(), 3);
    map.insert("I32TruncF32U".to_string(), 3);
    map.insert("I32TruncF64S".to_string(), 3);
    map.insert("I32TruncF64U".to_string(), 3);
    map.insert("I64ExtendI32S".to_string(), 2);
    map.insert("I64ExtendI32U".to_string(), 2);
    map
}

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
pub fn calculate_gas_for_contract_creation(wasm_bytes: &[u8]) -> Result<u64, Box<dyn std::error::Error>> {

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

    let total_gas = calculate_gas_by_opcode(&opcodes);
    Ok(total_gas)
}

fn calculate_gas_by_opcode(opcodes: &HashSet<String>)->u64 {
    let mut total_gas = 0;
    let gas_map = opcode_gas_costs();

    for opcode in opcodes {
        // Extract the base opcode name. For example, "Call { function_index: 55 }" becomes "Call".
        let base_opcode = opcode.split_whitespace().next().unwrap_or("");

        if let Some(&gas) = gas_map.get(base_opcode) {
            total_gas += gas;
            //println!("Opcode: {:?}, Gas cost: {}", base_opcode, gas);
        } else {
            println!("No gas cost found for base opcode {}", base_opcode);
        }
    }
    total_gas
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
