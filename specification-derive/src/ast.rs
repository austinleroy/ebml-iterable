use syn::{Data, DeriveInput, Error, Generics, Ident, Result, LitInt};

pub struct Enum<'a> {
    pub original: &'a DeriveInput,
    pub ident: Ident,
    pub variants: Vec<Variant<'a>>,
    pub generics: &'a Generics,
}

pub struct Variant<'a> {
    pub original: &'a syn::Variant,
    pub attributes: Attributes,
    pub ident: Ident,
}

pub struct Attributes {
    pub id: u64,
    pub data_type: syn::Path,
}

impl<'a> Enum<'a> {
    pub fn from_syn(node: &'a DeriveInput) -> Result<Self> {
        let data = match &node.data {
            Data::Enum(data) => data,
            _ => { return Err(Error::new_spanned(node, "#[derive(EbmlSpecification)] only works on Enums")) }
        };

        let variants = data
            .variants
            .iter()
            .map(|node| Variant::from_syn(node))
            .collect::<Result<_>>()?;

        Ok(Enum {
            original: node,
            ident: node.ident.clone(),
            variants,
            generics: &node.generics,
        })
    }
}

impl<'a> Variant<'a> {
    fn from_syn(node: &'a syn::Variant) -> Result<Self> {
        let mut id: Option<u64> = None;
        let mut data_type: Option<syn::Path> = None;
    
        for attr in &node.attrs {
            if attr.path.is_ident("id") {
                if id.is_some() {
                    return Err(Error::new_spanned(node, "duplicate #[id] attribute"));
                }
                id = Some(attr.parse_args::<LitInt>()?.base10_parse::<u64>()?);
            } else if attr.path.is_ident("data_type") {
                if data_type.is_some() {
                    return Err(Error::new_spanned(node, "duplicate #[data_type] attribute"));
                }
                data_type = Some(attr.parse_args::<syn::Path>()?);
            } 
        }

        if id.is_none() {
            return Err(Error::new_spanned(node, "#[id] attribute is required when deriving EbmlSpecification"));
        }
        let id = id.unwrap();

        if data_type.is_none() {
            return Err(Error::new_spanned(node, "#[data_type] attribute is required when deriving EbmlSpecification"));
        }
        let data_type = data_type.unwrap();

        Ok(Variant {
            original: node,
            attributes: Attributes {
                id,
                data_type,
            },
            ident: node.ident.clone(),
        })
    }
}