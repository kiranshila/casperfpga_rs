#![deny(clippy::all)]
#![warn(clippy::pedantic)]

mod fpg;

use casper_utils::design_sources::fpg::read_fpg_file;
use fpg::{generate_constructors, generate_field_names, generate_struct_fields, FpgFpga};
use proc_macro::TokenStream;
use quote::quote;
use std::path::PathBuf;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(CasperSerde)]
/// Derived on a [`PackedStruct`] to shim in our serde methods on packed structs
pub fn derive_casper_serde(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);
    let block_name = input.ident;
    let generated = quote! {
        impl Serialize for #block_name {
            type Chunk = <Self as PackedStruct>::ByteArray;

            fn serialize(&self) -> Self::Chunk {
                self.pack().expect("Packing failed, this shouldn't happen")
            }
        }

        impl Deserialize for #block_name {
            type Chunk = <Self as PackedStruct>::ByteArray;

            fn deserialize(chunk: Self::Chunk) -> anyhow::Result<Self> {
                Ok(Self::unpack(&chunk)?)
            }
        }
    };
    TokenStream::from(generated)
}

#[proc_macro_attribute]
/// Implement the Address trait on this struct, allowing for automatic addressing when reading and
/// writing
/// # Panics
/// Panics on bad address literals
#[allow(clippy::manual_flatten)]
#[allow(clippy::manual_let_else)]
pub fn address(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = match syn::parse::<syn::Lit>(attr).expect("Error parsing attribute") {
        syn::Lit::Int(v) => v,
        _ => panic!("The address must be a literal integer (hopefully a u8)"),
    };
    let num = attr;
    // Get the struct name this address is for
    let item = parse_macro_input!(item as DeriveInput);
    let ident = item.clone().ident;

    let generated = quote! {
        impl Address for #ident {
            fn addr() -> u16 {
                #num as u16
            }
        }
        #item
    };
    TokenStream::from(generated)
}

#[proc_macro]
/// Generates a fully-typed and specified FPGA instance using the object definitions from a given
/// fpg file.
pub fn fpga_from_fpg(tokens: TokenStream) -> TokenStream {
    let FpgFpga { name, filename } = parse_macro_input!(tokens as FpgFpga);
    let filename = filename.value();

    let fpg = read_fpg_file(PathBuf::from(filename)).expect("Couldn't read FPG file");

    let struct_fields = generate_struct_fields(&fpg.devices);
    let field_names = generate_field_names(&fpg.devices);
    let constructors = generate_constructors(&fpg.devices);

    // For every device in the fpg file, create a typed entry in the struct
    let generated = quote! {
        #[derive(Debug)]
        pub struct #name<T> {
            pub transport: std::sync::Arc<std::sync::Mutex<T>>,
            #(#struct_fields),*
        }

        impl<T> #name<T>
        where
            T: casperfpga::transport::Transport
        {
            pub fn new(transport: T) -> anyhow::Result<Self> {
                // Create the Arc Mutex for the transport
                let tarc = std::sync::Arc::new(std::sync::Mutex::new(transport));
                // And create the weak to pass to the yellow blocks
                let tweak = std::sync::Arc::downgrade(&tarc);
                // For every fpg device, run its `from_fpg` method
                #(#constructors)*
                // We probably want to actualy enforce that we program the FPGA at some point
                Ok(Self {transport: tarc, #(#field_names,)*})
            }
        }
    };
    TokenStream::from(generated)
}
