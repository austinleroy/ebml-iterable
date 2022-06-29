use proc_macro2::TokenStream;
use syn::{Attribute, AttrStyle, Ident, LitInt, parse::Parse, Token, Variant, Visibility};
use syn::parse::{ParseBuffer, ParseStream};
use syn::punctuated::Punctuated;
use syn::Result;
use syn::Error;
use quote::{quote};
use syn::spanned::Spanned;

pub struct EasyEBML {
    attrs: Vec<Attribute>,
    visibility: Visibility,
    ident: Ident,
    variants: Punctuated<EasyEBMLVariant, Token![,]>
}


impl Parse for EasyEBML {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let visibility: Visibility = input.parse()?;
        input.parse::<Token![enum]>()?;
        let ident = input.parse::<Ident>()?;
        let content: ParseBuffer;
        syn::braced!(content in input);
        let variants = content.parse_terminated(EasyEBMLVariant::parse)?;
        Ok(Self {
            attrs,
            visibility,
            ident,
            variants
        })
    }
}

impl EasyEBML {

    pub fn implement(self) -> Result<TokenStream> {
        let EasyEBML { attrs, visibility, ident, variants } = self;

        let variants: Vec<_> = variants.into_iter().map(EasyEBMLVariant::into_variant).collect::<Result<_>>()?;

        Ok(quote!(
            #[ebml_iterable::specs::ebml_specification]
            #(#attrs)*
            #visibility enum #ident {
                #(#variants),*
            }
        ))
    }
}

pub struct EasyEBMLVariant {
    path: Punctuated<Ident, Token![/]>,
    ty: Ident,
    id: LitInt
}

impl EasyEBMLVariant {
    pub fn into_variant(self) -> Result<Variant> {
        let EasyEBMLVariant { mut path, ty, id } = self;
        let ident = path.pop().ok_or_else(|| Error::new(path.span(), "easy_ebml enum variant must be at least: `Name: Type = id`"))?.into_value();
        let mut attrs = vec![];
        attrs.push(Attribute {
            pound_token: Default::default(),
            style: AttrStyle::Outer,
            bracket_token: Default::default(),
            path: Ident::new("id", proc_macro2::Span::call_site()).into(),
            tokens: quote!((#id))
        });
        attrs.push(Attribute {
            pound_token: Default::default(),
            style: AttrStyle::Outer,
            bracket_token: Default::default(),
            path: Ident::new("data_type", proc_macro2::Span::call_site()).into(),
            tokens: quote!((TagDataType::#ty))
        });

        if let Some(it) = path.pop() {
            let it = it.into_value();
            attrs.push(Attribute {
                pound_token: Default::default(),
                style: AttrStyle::Outer,
                bracket_token: Default::default(),
                path: Ident::new("parent", proc_macro2::Span::call_site()).into(),
                tokens: quote!((#it))
            });
        }

        Ok(Variant {
            attrs,
            ident,
            fields: syn::Fields::Unit,
            discriminant: None
        })
    }
}

impl Parse for EasyEBMLVariant {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = Punctuated::parse_separated_nonempty(input)?;
        input.parse::<Token![:]>()?;
        let ty: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let id: LitInt = input.parse()?;
        Ok(Self {
            path,
            ty,
            id
        })
    }
}
