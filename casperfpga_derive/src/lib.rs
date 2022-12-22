use proc_macro::TokenStream;
use quote::quote;
use syn::{parse, parse_macro_input, DeriveInput};

#[proc_macro_derive(CasperSerde)]
/// Derived on a packed_struct to shim in our serde methods on packed structs
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
pub fn address(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = match parse::<syn::Lit>(attr).expect("Error parsing attribute") {
        syn::Lit::Int(v) => v,
        _ => panic!("The address must be a literal integer (hopefully a u8)"),
    };
    let num = attr;
    // Get the struct name this address is for
    let item = parse_macro_input!(item as DeriveInput);
    let ident = item.clone().ident;

    let generated = quote! {
        impl #ident {
            fn addr() -> u8 {
                #num as u8
            }
        }
        #item
    };
    TokenStream::from(generated)
}
