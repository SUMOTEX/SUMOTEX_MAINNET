extern crate proc_macro;
use proc_macro::TokenStream;
use syn::Pat;
use syn::FnArg;
use quote::quote;
// use syn::{Item,parse_macro_input};

use syn::{parse_macro_input, ItemImpl, ImplItem};
use std::fs::File;
use std::io::prelude::*;


#[proc_macro_attribute]
pub fn generate_abi(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let implementation = parse_macro_input!(input as ItemImpl);
    let mut functions = vec![];

    for item in &implementation.items {
        if let ImplItem::Method(method) = item {
            let function_name = &method.sig.ident;
            let mut inputs = vec![];

            for input in method.sig.inputs.iter() {
                match input {
                    FnArg::Typed(arg) => {
                        let arg_name = match &*arg.pat {
                            Pat::Ident(pat_ident) => &pat_ident.ident,
                            _ => continue,
                        };
                        let type_name = quote! { #arg.ty }.to_string();
                        inputs.push(format!(r#"{{"name": "{}", "type": "{}"}}"#, arg_name, type_name));
                    },
                    _ => {},
                }
            }
            functions.push(format!(r#"{{
                "name": "{}",
                "type": "function",
                "inputs": [{}]
            }}"#, function_name, inputs.join(",")));
        }
    }

    eprintln!("ABI {:?}",functions);
    let abi_string = format!(r#"[{}]"#, functions.join(","));
    let expanded = quote! {
        const ABI: &str = #abi_string;
    };
    let mut file = File::create("./abi.json").unwrap();
    file.write_all(abi_string.as_bytes()).unwrap();
    TokenStream::from(expanded)

}
