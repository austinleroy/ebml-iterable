use proc_macro2::TokenStream;
use syn::spanned::Spanned;
use syn::{DeriveInput, Result, Data, Visibility};
use quote::{quote, quote_spanned};

use super::ast::Enum;

pub fn impl_ebml_specification_macro(ast: &DeriveInput) -> Result<TokenStream> {
    let input = Enum::from_syn(ast)?;

    let ebml_spec_trait = spanned_ebml_specification_trait(ast);
    let tag_data_type = spanned_tag_data_type(ast);
    let ty = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let get_tags = input.variants.iter().map(|var: &crate::ast::Variant| {
        let name = &var.ident;
        let id = &var.attributes.id;
        let data_type = &var.attributes.data_type;

        quote! {
            if id == #id {
                Some((#ty::#name, #data_type))
            }
        }
    });

    let get_tag_ids = input.variants.iter().map(|var: &crate::ast::Variant| {
        let name = &var.ident;
        let id = &var.attributes.id;

        quote! {
            #ty::#name => #id,
        }
    });

    Ok(quote! {        
        impl #impl_generics #ebml_spec_trait <#ty> for #ty #ty_generics #where_clause {
            fn get_tag(id: u64) -> Option<(#ty, #tag_data_type)> {
                #(#get_tags else)* {
                    None
                }
            }

            fn get_tag_id(item: &#ty) -> u64 {
                match item {
                    #(#get_tag_ids)*
                }
            }
        }
    })
}

fn spanned_ebml_specification_trait(input: &DeriveInput) -> TokenStream {
    let vis_span = match &input.vis {
        Visibility::Public(vis) => Some(vis.pub_token.span()),
        Visibility::Crate(vis) => Some(vis.crate_token.span()),
        Visibility::Restricted(vis) => Some(vis.pub_token.span()),
        Visibility::Inherited => None,
    };
    let data_span = match &input.data {
        Data::Struct(data) => data.struct_token.span(),
        Data::Enum(data) => data.enum_token.span(),
        Data::Union(data) => data.union_token.span(),
    };
    let first_span = vis_span.unwrap_or(data_span);
    let last_span = input.ident.span();
    let path = quote_spanned!(first_span=> ebml_iterable::specs::);
    let spec = quote_spanned!(last_span=> EbmlSpecification);
    quote!(#path #spec)
}

fn spanned_tag_data_type(input: &DeriveInput) -> TokenStream {
    let vis_span = match &input.vis {
        Visibility::Public(vis) => Some(vis.pub_token.span()),
        Visibility::Crate(vis) => Some(vis.crate_token.span()),
        Visibility::Restricted(vis) => Some(vis.pub_token.span()),
        Visibility::Inherited => None,
    };
    let data_span = match &input.data {
        Data::Struct(data) => data.struct_token.span(),
        Data::Enum(data) => data.enum_token.span(),
        Data::Union(data) => data.union_token.span(),
    };
    let first_span = vis_span.unwrap_or(data_span);
    let last_span = input.ident.span();
    let path = quote_spanned!(first_span=> ebml_iterable::specs::);
    let r#type = quote_spanned!(last_span=> TagDataType);
    quote!(#path #r#type)
}