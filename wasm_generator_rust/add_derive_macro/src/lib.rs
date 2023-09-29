extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Item};

#[proc_macro_attribute]
pub fn add_derive(_attrs: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Item);

    let expanded = match input {
        Item::Struct(mut item_struct) => {
            // Manipulate the struct. For example, add a derive:
            item_struct.attrs.push(syn::parse_quote!(#[derive(Debug)]));
            quote! { #item_struct }
        },
        _ => panic!("Expected a struct"),
    };

    TokenStream::from(expanded)
}