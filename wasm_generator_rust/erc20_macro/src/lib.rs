extern crate proc_macro;
use proc_macro::TokenStream;
use syn::Pat;
use syn::FnArg;
use quote::quote;
// use syn::{Item,parse_macro_input};
use quote::ToTokens;
use syn::{parse_macro_input, ItemImpl, ImplItem};
use std::fs::File;
use std::io::prelude::*;
use syn::ReturnType;

#[proc_macro_attribute]
pub fn generate_abi(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let implementation = parse_macro_input!(input as ItemImpl);
    let mut functions = vec![];

    for item in &implementation.items {
        if let ImplItem::Method(method) = item {
            let function_name = &method.sig.ident;
            let mut inputs = vec![];
            let mut outputs = vec![];

            for input in method.sig.inputs.iter() {
                match input {
                    FnArg::Typed(arg) => {
                        let arg_name = match &*arg.pat {
                            Pat::Ident(pat_ident) => &pat_ident.ident,
                            _ => continue,
                        };
                        let type_name = quote! { #arg.ty }.to_string();
                        // Remove the .ty suffix
                        let clean_type_name = type_name.trim_end_matches(".ty").to_string();
                        let clean_type = clean_type_name.split_whitespace().last().unwrap_or_default();
                        inputs.push(format!(r#"{{"name": "{}", "type": "{}"}}"#, arg_name, clean_type));
                    },
                    _ => {},
                }
            }

            let return_type = match &method.sig.output {
                ReturnType::Type(_, ty) => {
                    // Remove the .ty suffix
                    let clean_return_type = quote! { #ty }.to_string().trim_end_matches(".ty").to_string();
                    let clean_type = clean_return_type.split_whitespace().last().unwrap_or_default();
                    outputs.push(format!(r#"{{"name": "{}", "type": "{}"}}"#, "output", clean_type));
                },
                ReturnType::Default => {
                    // No output
                }
            };

            functions.push(format!(r#"{{
                "name": "{}",
                "type": "function",
                "inputs": [{}],
                "outputs": [{}]
            }}"#, function_name, inputs.join(","), outputs.join(",")));
        }
    }

    println!("ABI {:?}", functions);
    let abi_string = format!(r#"[{}]"#, functions.join(","));
    let expanded = quote! {
        const ABI: &str = #abi_string;
    };
    let mut file = File::create("./abi.json").unwrap();
    file.write_all(abi_string.as_bytes()).unwrap();
    TokenStream::from(expanded)
}


// #[proc_macro_attribute]
// pub fn generate_abi(_attrs: TokenStream, input: TokenStream) -> TokenStream {
//     let implementation = parse_macro_input!(input as ItemImpl);
//     let mut functions = vec![];

//     for item in &implementation.items {
//         if let ImplItem::Method(method) = item {
//             let function_name = &method.sig.ident;
//             let mut inputs = vec![];

//             for input in method.sig.inputs.iter() {
//                 match input {
//                     FnArg::Typed(arg) => {
//                         let arg_name = match &*arg.pat {
//                             Pat::Ident(pat_ident) => &pat_ident.ident,
//                             _ => continue,
//                         };
//                         let type_name = format!("{}", arg.ty.to_token_stream());
//                         inputs.push(format!(r#"{{"name": "{}", "type": "{}"}}"#, arg_name, type_name));
//                     },
//                     _ => {},
//                 }
//             }
//             functions.push(format!(r#"{{
//                 "name": "{}",
//                 "type": "function",
//                 "inputs": [{}]
//             }}"#, function_name, inputs.join(",")));
//         }
//     }

//     eprintln!("ABI {:?}",functions);
//     let abi_string = format!(r#"[{}]"#, functions.join(","));
//     let expanded = quote! {
//         const ABI: &str = #abi_string;
//     };
//     let mut file = File::create("./test_abi.json").unwrap();
//     file.write_all(abi_string.as_bytes()).unwrap();
//     TokenStream::from(expanded)
// }