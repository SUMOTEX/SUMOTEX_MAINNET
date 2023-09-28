extern crate proc_macro;

pub fn generate_abi_string() -> String {
    // This function generates ABI as a String
    // ... your ABI generation logic here
    // println!("{}", ABI);
    // fs::write("path_to_your_directory/abi.json", ABI).expect("Unable to write ABI to file");
    format!(r#"[{}]"#, functions.join(","))
}