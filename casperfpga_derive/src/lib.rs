use casper_utils::bitstream::fpg::{
    read_fpg_file,
    FpgDevice,
};
use kstring::KString;
use proc_macro::TokenStream;
use quote::quote;
use std::{
    collections::HashMap,
    path::PathBuf,
};
use syn::{
    parse::{
        Parse,
        ParseStream,
    },
    parse_macro_input,
    DeriveInput,
    Ident,
    LitStr,
    Token,
};

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
    let attr = match syn::parse::<syn::Lit>(attr).expect("Error parsing attribute") {
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

struct FpgFpga {
    name: Ident,
    filename: LitStr,
}

impl Parse for FpgFpga {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![,]>()?;
        let filename = input.parse()?;
        Ok(FpgFpga { name, filename })
    }
}

fn kind_to_type(kind: &str) -> Option<proc_macro2::TokenStream> {
    match kind {
        "xps:sw_reg" => Some(quote!(
            casperfpga::yellow_blocks::swreg::SoftwareRegister::<T>
        )),
        "xps:ten_gbe" => Some(quote!(casperfpga::yellow_blocks::ten_gbe::TenGbE::<T>)),
        // Ignore the types that don't have mappings to yellow block implementations
        _ => None,
    }
}

fn dev_to_constructor(name: &str, dev: &FpgDevice) -> Option<proc_macro2::TokenStream> {
    if let Some(ty) = kind_to_type(&dev.kind) {
        let ident = syn::parse_str::<Ident>(name).unwrap_or_else(|_| {
            panic!("FPGA register name `{name}` is not a valid rust identifier")
        });
        macro_rules! from_fpg {
            () => {
                Some(quote! {let #ident = #ty::from_fpg(tweak.clone(), #name)?;})
            };
            ($($key:ident),+) => {{
                $(let $key = dev.metadata.get(stringify!($key)).expect("Malformed FPG metadata");)+
                Some(quote! {let #ident = #ty::from_fpg(tweak.clone(), #name, $(#$key,)+)?;})
            }};
        }
        // These need to match the key order from the device's `from_fpg` method
        match dev.kind.as_str() {
            "xps:sw_reg" => from_fpg!(io_dir, bin_pts, arith_types),
            "xps:ten_gbe" => from_fpg!(),
            // Ignore the types that don't have mappings to yellow block implementations
            _ => None,
        }
    } else {
        None
    }
}

fn generate_struct_fields(devices: &HashMap<KString, FpgDevice>) -> Vec<proc_macro2::TokenStream> {
    devices
        .iter()
        .filter_map(|(name, dev)| {
            // Make sure this is actually a register
            dev.register.as_ref()?;
            // Construct the token stream
            let ident = syn::parse_str::<Ident>(name.as_str()).unwrap_or_else(|_| {
                panic!("FPGA register name `{name}` is not a valid rust identifier")
            });
            kind_to_type(&dev.kind).map(|ty| {
                quote! {
                    #ident: #ty
                }
            })
        })
        .collect()
}

fn generate_field_names(devices: &HashMap<KString, FpgDevice>) -> Vec<Ident> {
    devices
        .iter()
        .filter_map(|(name, dev)| {
            // Make sure this is actually a register
            dev.register.as_ref()?;
            // Construct the token stream
            let ident = syn::parse_str::<Ident>(name.as_str()).unwrap_or_else(|_| {
                panic!("FPGA register name `{name}` is not a valid rust identifier")
            });
            kind_to_type(&dev.kind).map(|_| ident)
        })
        .collect()
}

fn generate_constructors(devices: &HashMap<KString, FpgDevice>) -> Vec<proc_macro2::TokenStream> {
    devices
        .iter()
        .filter_map(|(name, dev)| {
            // Make sure this is actually a register
            dev.register.as_ref()?;
            // Construct the token stream
            dev_to_constructor(name, dev)
        })
        .collect()
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
        struct #name<T> {
            transport: std::sync::Arc<T>,
            #(#struct_fields),*
        }

        impl<T> #name<T>
        where
            T: casperfpga::transport::Transport
        {
            pub fn new(transport: T) -> anyhow::Result<Self> {
                // Create the Arc for the transport
                let tarc = std::sync::Arc::new(transport);
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
