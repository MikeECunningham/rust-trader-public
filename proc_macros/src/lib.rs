use proc_macro::TokenStream;
use quote::*;
use syn::{ parse_macro_input, ItemStruct};
use std::collections::BTreeMap;


/// Derives a signature generation on the fields of a struct
#[proc_macro_derive(BybitSignable)]
pub fn derive_bybit_signable(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let ItemStruct {
        attrs: _,
        vis: _,
        struct_token: _,
        ident,
        generics: _,
        fields,
        semi_token: _
    } = input;
    let mut sig_fields: BTreeMap<String, String> = BTreeMap::new();
    let mut format_str = String::from("");
    let mut format_args = quote!{};
    // Identifier for the struct we are deriving
    // Iterate over all of the fields in the derived struct and sort them before making the serialize string
    for field in fields.iter() {
        let ident_str = field.ident.as_ref().unwrap().to_string();
        let field_val = String::from("self.") + &ident_str;
        sig_fields.insert(ident_str, field_val);
    }
    // Add timestamp and key to the list
    sig_fields.insert(String::from("api_key"), String::from("api_key"));

    for (field_name, mut field_value) in sig_fields {
        // If this field is the signature, skip signing it
        if field_name == "sign" { continue; }
        // If this isn't the first field, add an & before the ident
        if format_str.len() > 0 { format_str += "&"; }
        format_str += field_name.as_str();
        format_str += "={}";
        if field_value.starts_with("self.") {
            format_args.append_all(quote!{ self.});
            field_value = field_value[5..].to_string();
        }
        format_args.append(format_ident!("{}", field_value));
        format_args.append_all(quote!{,});
    }
    let mut new_fields = quote!{};
    let mut copy_fields = quote!{};
    for field in fields {
        new_fields.append_all(quote!{pub #field,});
        let ident = field.ident.expect("All fields must have an identifier");
        copy_fields.append_all(quote!{#ident,})
    }
    let tokens = quote! {

        impl #ident {
            pub fn get_signed_data(&mut self, secret: String, api_key: String) -> Result<String, SignRequestError>{
                use hmac::{Hmac, Mac};
                use sha2::{Sha256};
                use crate::config::CONFIG;
                let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
                mac.update(format!(#format_str, #format_args).as_bytes());
                self.sign = format!("{:X}", mac.finalize().into_bytes());
                self.api_key = api_key;
                let res = serde_json::to_string(&self)?;
                return Ok(res);
            }
        }
    };
    return TokenStream::from(tokens);
}

/// Derives a signature generation on the fields of a struct
#[proc_macro_derive(BinanceSignable)]
pub fn derive_binance_signable(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let ItemStruct {
        attrs: _,
        vis: _,
        struct_token: _,
        ident,
        generics: _,
        fields,
        semi_token: _
    } = input;
    let mut sig_fields: Vec<(String, String)> = Vec::new();
    let mut format_str = String::from("");
    let mut format_args = quote!{};
    // Identifier for the struct we are deriving
    // Iterate over all of the fields in the derived struct and sort them before making the serialize string
    for field in fields.iter() {
        let ident_str = field.ident.as_ref().unwrap().to_string();
        let field_val = String::from("self.") + &ident_str;
        sig_fields.push((ident_str, field_val));
    }

    for (field_name, mut field_value) in sig_fields {
        // If this field is the signature, skip signing it
        if field_name == "signature" { continue; }
        // If this isn't the first field, add an & before the ident
        if format_str.len() > 0 { format_str += "&"; }
        format_str += field_name.as_str();
        format_str += "={}";
        if field_value.starts_with("self.") {
            format_args.append_all(quote!{ self.});
            field_value = field_value[5..].to_string();
        }
        format_args.append(format_ident!("{}", field_value));
        format_args.append_all(quote!{,});
    }
    let mut new_fields = quote!{};
    let mut copy_fields = quote!{};
    for field in fields {
        new_fields.append_all(quote!{pub #field,});
        let ident = field.ident.expect("All fields must have an identifier");
        copy_fields.append_all(quote!{#ident,})
    }
    let tokens = quote! {

        impl #ident {

            pub fn get_signed_data(&mut self, secret: String) -> Result<String, SignRequestError>{
                use hmac::{Hmac, Mac};
                use sha2::{Sha256};
                use crate::config::CONFIG;
                use serde_urlencoded;
                let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
                let querystring = serde_urlencoded::to_string(&self)?;
                mac.update(querystring.as_bytes());
                let signature = format!("{:X}", mac.finalize().into_bytes());
                return Ok(format!("{}&signature={}", querystring, signature));
            }
        }
    };
    return TokenStream::from(tokens);
}