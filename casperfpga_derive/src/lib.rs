#![deny(clippy::all)]
#![warn(clippy::pedantic)]

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

fn fixed_type(dev: &FpgDevice) -> proc_macro2::TokenStream {
    let bin_pts: u32 = dev
        .metadata
        .get("bin_pts")
        .expect("Malformed FPG metadata")
        .parse()
        .expect("Binary point wasn't a number?");
    let frac_ident = syn::parse_str::<Ident>(&format!("U{bin_pts}")).unwrap();
    match dev.metadata.get("arith_types").unwrap().as_str() {
        "0" => quote! {fixed::FixedU32::<fixed::types::extra::#frac_ident>},
        "1" => quote! {fixed::FixedI32::<fixed::types::extra::#frac_ident>},
        _ => unreachable!(),
    }
}

fn disambiguate_sw_reg(dev: &FpgDevice) -> proc_macro2::TokenStream {
    // Unfortunatley, software registers are not uniquely determined by their fpg type, we need
    // additional metadata to know what rust types they become
    match dev.metadata.get("arith_types").unwrap().as_str() {
        "0" | "1" => {
            let fixed_ty = fixed_type(dev);
            quote!(casperfpga::yellow_blocks::swreg::FixedSoftwareRegister::<T, #fixed_ty>)
        }
        "2" => quote!(casperfpga::yellow_blocks::swreg::BooleanSoftwareRegister::<T>),
        _ => unreachable!(),
    }
}

fn kind_to_type(dev: &FpgDevice) -> Option<proc_macro2::TokenStream> {
    match dev.kind.as_str() {
        "xps:sw_reg" => Some(disambiguate_sw_reg(dev)),
        "xps:ten_gbe" => Some(quote!(casperfpga::yellow_blocks::ten_gbe::TenGbE::<T>)),
        "xps:snap_adc" => Some(quote!(casperfpga::yellow_blocks::snapadc::SnapAdc::<T>)),
        // Ignore the types that don't have mappings to yellow block implementations
        _ => None,
    }
}

fn dev_to_constructor(
    name: &str,
    devices: &HashMap<KString, FpgDevice>,
) -> Option<proc_macro2::TokenStream> {
    // So, some devices will require entries from *other* devices, like SNAP ADCs needing to know
    // the clock source, so we'll pass in a single key to the device map and the map itself, so we
    // can look up other entires

    let dev = devices.get(name).unwrap();

    if let Some(ty) = kind_to_type(dev) {
        let ident = syn::parse_str::<Ident>(name).ok()?;
        // Build the constructor for the given device using its `from_fpg` method.
        // Follows the informal contract that it begins with the weak transport pointer
        // and the name of the device.
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
            "xps:sw_reg" => match dev.metadata.get("arith_types").unwrap().as_str() {
                "0" | "1" => from_fpg!(io_dir, bitwidths),
                "2" => from_fpg!(io_dir),
                _ => unreachable!(),
            },
            "xps:ten_gbe" => from_fpg!(),
            "xps:snap_adc" => {
                let snap = devices
                    .get("SNAP")
                    .expect("SNAP ADC entries must accompany a SNAP entry");
                // Do we need the FPGA clock rate too?
                let src = snap
                    .metadata
                    .get("clk_src")
                    .expect("SNAP must have clk_src entry");

                // get the rest of the entires manually  - maybe clean this up by updating the macro
                let adc_resolution = dev
                    .metadata
                    .get("adc_resolution")
                    .expect("Malformed FPG metadata");

                let sample_rate = dev
                    .metadata
                    .get("sample_rate")
                    .expect("Malformed FPG metadata");

                let snap_inputs = dev
                    .metadata
                    .get("snap_inputs")
                    .expect("Malformed FPG metadata");

                Some(quote! {
                    let #ident = #ty::from_fpg(tweak.clone(), #name, #adc_resolution, #sample_rate, #snap_inputs, #src)?;
                })
            }
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
            // Construct the token stream
            kind_to_type(dev).map(|ty| {
                let ident = syn::parse_str::<Ident>(name.as_str()).unwrap_or_else(|_| {
                    panic!("FPGA register name `{name}` is not a valid rust identifier")
                });
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
            kind_to_type(dev).map(|_| {
                syn::parse_str::<Ident>(name.as_str()).unwrap_or_else(|_| {
                    panic!("FPGA register name `{name}` is not a valid rust identifier")
                })
            })
        })
        .collect()
}

fn generate_constructors(devices: &HashMap<KString, FpgDevice>) -> Vec<proc_macro2::TokenStream> {
    devices
        .iter()
        .filter_map(|(name, _)| dev_to_constructor(name, devices))
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
            transport: std::sync::Arc<std::sync::Mutex<T>>,
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
