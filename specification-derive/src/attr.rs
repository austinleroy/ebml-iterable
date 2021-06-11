use proc_macro2::TokenStream;
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{AttributeArgs, Attribute, ItemEnum, Result, Data, Error, Visibility, Fields, FieldsUnnamed, Path, Ident};
use quote::{quote, quote_spanned};
use ebml_iterable_specification::TagDataType;

use super::ast::Enum;

pub fn impl_ebml_specification(args: &AttributeArgs, original: &mut ItemEnum) -> Result<TokenStream> {
    let input = Enum::from_syn(original)?;
    let ebml_specification_impl = get_impl(input)?;

    let modified_orig = modify_orig(original)?;

    Ok(quote!(
        #modified_orig

        #ebml_specification_impl
    ))
}

fn modify_orig(original: &mut ItemEnum) -> Result<TokenStream> {
    let spanned_master_enum = spanned_master_enum(original).clone();
    for var in original.variants.iter_mut() {
        let data_type_attribute: &Attribute = var
            .attrs
            .iter()
            .find(|a| a.path.is_ident("data_type"))
            .expect("#[data_type()] attribute required for variants under #[ebml_specification]");
            
        let data_type_path = data_type_attribute.parse_args::<syn::Path>()?;
        let data_type = get_last_path_ident(&data_type_path).ok_or(Error::new_spanned(data_type_attribute.clone(), format!("#[data_type()] requires `ebml_iterable::TagDataType`")))?;
        
        let data_type = if data_type == "Master" {
            let orig_ident = &original.ident;
            quote!( (#spanned_master_enum<#orig_ident>) )
        } else if data_type == "UnsignedInt" {
            quote!( (u64) )
        } else if data_type == "Integer" {
            quote!( (i64) )
        } else if data_type == "Utf8" {
            quote!( (String) )
        } else if data_type == "Binary" {
            quote!( (::std::vec::Vec<u8>) )
        } else if data_type == "Float" {
            quote!( (f64) )
        } else {
            return Err(Error::new_spanned(data_type_attribute.clone(), format!("unknown data_type \"{}\"", data_type)));
        };

        var.attrs.retain(|a| {
            if a.path.is_ident("id") || a.path.is_ident("data_type") {
                false
            } else {
                true
            }
        });
        var.fields = Fields::Unnamed(syn::parse2::<FieldsUnnamed>(data_type)?);
    }

    Ok(quote!(#original))
}

fn get_impl(input: Enum) -> Result<TokenStream> {
    let ty = &input.ident;
    let spanned_master_enum = spanned_master_enum(input.original);

    let get_tag_data_type = input.variants.iter().map(|var: &crate::ast::Variant| {
        let name = &var.ident;
        let id = &var.attributes.id;
        let data_type = &var.attributes.data_type;

        quote! {
            if id == #id {
                Some(#data_type)
            }
        }
    });

    let get_id = input.variants.iter().map(|var: &crate::ast::Variant| {
        let name = &var.ident;
        let id = &var.attributes.id;

        quote! {
            #ty::#name(_) => #id,
        }
    });

    let get_tag = |ret_val: String| {
        move |var: &crate::ast::Variant| {
            let name = &var.ident;
            let id = &var.attributes.id;
            let ret_val = TokenStream::from_str(&ret_val).expect("Misuse of get_tag function in ebml_iterable_specification_derive_attr");
    
            quote! {
                if id == #id {
                    Some(#ty::#name(#ret_val))
                }
            }
        }
    };

    let get_unsigned_int_tag = input.variants.iter()
        .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::UnsignedInt))
        .map(get_tag(String::from("data")));

    let get_signed_int_tag = input.variants.iter()
    .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Integer))
        .map(get_tag(String::from("data")));

    let get_utf8_tag = input.variants.iter()
    .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Utf8))
        .map(get_tag(String::from("data")));

    let get_binary_tag = input.variants.iter()
    .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Binary))
        .map(get_tag(String::from("data.to_vec()")));

    let get_float_tag = input.variants.iter()
    .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Float))
        .map(get_tag(String::from("data")));

    let get_master_tag_start = input.variants.iter()
        .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Master))
        .map(get_tag(format!("{}::Start", spanned_master_enum)));

    let get_master_tag_end = input.variants.iter()
        .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Master))
        .map(get_tag(format!("{}::End", spanned_master_enum)));

    let get_master_tag_full = input.variants.iter()
        .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Master))
        .map(get_tag(format!("{}::Full(children.to_vec())", spanned_master_enum)));

    let get_data = |var: &crate::ast::Variant| {
        let name = &var.ident;
        let id = &var.attributes.id;

        quote! {
            #ty::#name(val) => Some(val),
        }
    };

    let get_unsigned_int_data = input.variants.iter()
        .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::UnsignedInt))
        .map(get_data);

    let get_signed_int_data = input.variants.iter()
    .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Integer))
        .map(get_data);

    let get_utf8_data = input.variants.iter()
    .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Utf8))
        .map(get_data);

    let get_binary_data = input.variants.iter()
    .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Binary))
        .map(get_data);

    let get_float_data = input.variants.iter()
    .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Float))
        .map(get_data);

    let get_master_data = input.variants.iter()
        .filter(|v| matches!(&v.attributes.data_type_val, TagDataType::Master))
        .map(get_data);

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ebml_spec_trait = spanned_ebml_specification_trait(input.original);
    let tag_data_type = spanned_tag_data_type(input.original);

    Ok(quote! {        
        impl #impl_generics #ebml_spec_trait <#ty> for #ty #ty_generics #where_clause {
            fn get_tag_data_type(id: u64) -> Option<#tag_data_type> {
                #(#get_tag_data_type else)* {
                    None
                }
            }

            fn get_id(&self) -> u64 {
                match &self {
                    #(#get_id)*
                }
            }

            fn get_unsigned_int_tag(id: u64, data: u64) -> Option<#ty> {
                #(#get_unsigned_int_tag else)* {
                    None
                }
            }

            fn get_signed_int_tag(id: u64, data: i64) -> Option<#ty> {
                #(#get_signed_int_tag else)* {
                    None
                }
            }

            fn get_utf8_tag(id: u64, data: String) -> Option<#ty> {
                #(#get_utf8_tag else)* {
                    None
                }
            }

            fn get_binary_tag(id: u64, data: &[u8]) -> Option<#ty> {
                #(#get_binary_tag else)* {
                    None
                }
            }

            fn get_float_tag(id: u64, data: f64) -> Option<#ty> {
                #(#get_float_tag else)* {
                    None
                }
            }

            fn get_master_tag_start(id: u64) -> Option<#ty> {
                #(#get_master_tag_start else)* {
                    None
                }
            }

            fn get_master_tag_end(id: u64) -> Option<#ty> {
                #(#get_master_tag_end else)* {
                    None
                }
            }

            fn get_master_tag_full(id: u64, children: &[#ty]) -> Option<#ty> {
                #(#get_master_tag_full else)* {
                    None
                }
            }

            fn get_unsigned_int_data(&self) -> Option<&u64> {
                match &self {
                    #(#get_unsigned_int_data)*
                    _ => None,
                }
            }

            fn get_signed_int_data(&self) -> Option<&i64> {
                match &self {
                    #(#get_signed_int_data)*
                    _ => None,
                }
            }

            fn get_utf8_data(&self) -> Option<&str> {
                match &self {
                    #(#get_utf8_data)*
                    _ => None,
                }
            }

            fn get_binary_data(&self) -> Option<&[u8]> {
                match &self {
                    #(#get_binary_data)*
                    _ => None,
                }
            }

            fn get_float_data(&self) -> Option<&f64> {
                match &self {
                    #(#get_float_data)*
                    _ => None,
                }
            }

            fn get_master_data(&self) -> Option<&#spanned_master_enum<#ty>> {
                match &self {
                    #(#get_master_data)*
                    _ => None,
                }
            }
        }
    })
}

fn spanned_ebml_iterable_specs(input: &ItemEnum) -> TokenStream {
    let vis_span = match &input.vis {
        Visibility::Public(vis) => Some(vis.pub_token.span()),
        Visibility::Crate(vis) => Some(vis.crate_token.span()),
        Visibility::Restricted(vis) => Some(vis.pub_token.span()),
        Visibility::Inherited => None,
    };
    let data_span = input.enum_token.span();
    let first_span = vis_span.unwrap_or(data_span);
    quote_spanned!(first_span=> ebml_iterable::specs::)
}

fn spanned_master_enum(input: &ItemEnum) -> TokenStream {
    let path = spanned_ebml_iterable_specs(input);
    let last_span = input.ident.span();
    let r#enum = quote_spanned!(last_span=> Master);
    quote!(#path #r#enum)
}

fn spanned_ebml_specification_trait(input: &ItemEnum) -> TokenStream {
    let path = spanned_ebml_iterable_specs(input);
    let last_span = input.ident.span();
    let spec = quote_spanned!(last_span=> EbmlSpecification);
    quote!(#path #spec)
}

fn spanned_tag_data_type(input: &ItemEnum) -> TokenStream {
    let path = spanned_ebml_iterable_specs(input);
    let last_span = input.ident.span();
    let r#type = quote_spanned!(last_span=> TagDataType);
    quote!(#path #r#type)
}

fn get_last_path_ident(path: &Path) -> Option<&Ident> {
    let seg = path.segments.iter().last();
    if seg.is_none() {
        None
    } else {
        Some(&seg.unwrap().ident)
    }
}