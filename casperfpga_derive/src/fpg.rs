//! Methods/Macros for translating fpg files into Rust datatypes

use casper_utils::design_sources::Device;
use kstring::KString;
use quote::quote;
use std::collections::HashMap;
use syn::{
    parse::{
        Parse,
        ParseStream,
    },
    Ident,
    LitStr,
    Token,
};

pub(crate) struct FpgFpga {
    pub name: Ident,
    pub filename: LitStr,
}

impl Parse for FpgFpga {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![,]>()?;
        let filename = input.parse()?;
        Ok(FpgFpga { name, filename })
    }
}

fn fixed_type(dev: &Device) -> proc_macro2::TokenStream {
    let bin_pts: u32 = dev
        .metadata
        .get("bin_pts")
        .expect("Malformed FPG metadata")
        .parse()
        .expect("Binary point wasn't a number");
    let frac_ident = syn::parse_str::<Ident>(&format!("U{bin_pts}")).unwrap();
    match dev.metadata.get("arith_types").unwrap().as_str() {
        "0" => quote! {fixed::FixedU32::<fixed::types::extra::#frac_ident>},
        "1" => quote! {fixed::FixedI32::<fixed::types::extra::#frac_ident>},
        _ => unreachable!(),
    }
}

fn disambiguate_sw_reg(dev: &Device) -> proc_macro2::TokenStream {
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

fn disambiguate_snapshot(dev: &Device) -> proc_macro2::TokenStream {
    let width: u32 = dev
        .metadata
        .get("data_width")
        .expect("Malformed FPG metadata")
        .parse()
        .expect("Snapshot datawidth wasn't a number?");
    let ty = match width {
        8 => quote!(u8),
        16 => quote!(u16),
        32 => quote!(u32),
        64 => quote!(u64),
        128 => quote!(u128),
        _ => panic!("Invalid data_width"),
    };
    quote!(casperfpga::yellow_blocks::snapshot::Snapshot::<T, #ty>)
}

fn kind_to_type(dev: &Device) -> Option<proc_macro2::TokenStream> {
    match dev.kind.as_str() {
        "xps:sw_reg" => Some(disambiguate_sw_reg(dev)),
        "xps:ten_gbe" => Some(quote!(casperfpga::yellow_blocks::ten_gbe::TenGbE::<T>)),
        "xps:snap_adc" => Some(quote!(casperfpga::yellow_blocks::snapadc::SnapAdc::<T>)),
        "casper:snapshot" => Some(disambiguate_snapshot(dev)),
        // Ignore the types that don't have mappings to yellow block implementations
        _ => None,
    }
}

fn dev_to_constructor(
    name: &str,
    devices: &HashMap<KString, Device>,
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
            "casper:snapshot" => from_fpg!(nsamples, offset),
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

pub(crate) fn generate_struct_fields(
    devices: &HashMap<KString, Device>,
) -> Vec<proc_macro2::TokenStream> {
    devices
        .iter()
        .filter_map(|(name, dev)| {
            // Construct the token stream
            kind_to_type(dev).map(|ty| {
                let ident = syn::parse_str::<Ident>(name.as_str()).unwrap_or_else(|_| {
                    panic!("FPGA register name `{name}` is not a valid rust identifier")
                });
                quote! {
                    pub #ident: #ty
                }
            })
        })
        .collect()
}

pub(crate) fn generate_field_names(devices: &HashMap<KString, Device>) -> Vec<Ident> {
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

pub(crate) fn generate_constructors(
    devices: &HashMap<KString, Device>,
) -> Vec<proc_macro2::TokenStream> {
    devices
        .iter()
        .filter_map(|(name, _)| dev_to_constructor(name, devices))
        .collect()
}
