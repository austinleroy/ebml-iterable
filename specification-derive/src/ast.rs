use syn::{Data, ItemEnum, Error, Generics, Ident, Result, LitInt};

use ebml_iterable_specification::TagDataType;

pub struct Enum<'a> {
    pub original: &'a ItemEnum,
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
    pub data_type_val: TagDataType,
}

impl<'a> Enum<'a> {
    pub fn from_syn(node: &'a ItemEnum) -> Result<Self> {
        let variants = node
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
            return Err(Error::new_spanned(node, "#[id] attribute is required when using #[ebml_specification] attribute"));
        }
        let id = id.unwrap();

        if data_type.is_none() {
            return Err(Error::new_spanned(node, "#[data_type] attribute is required when using #[ebml_specification] attribute"));
        }
        let data_type = data_type.unwrap();

        let data_type_name = data_type.segments.iter().last();
        if data_type_name.is_none() {
            return Err(Error::new_spanned(node, "#[data_type] attribute value could not be resolved, expected `TagDataType` variant"));
        }
        let data_type_name = data_type_name.unwrap().ident.to_string();
        let data_type_val = if data_type_name == "UnsignedInt" {
            TagDataType::UnsignedInt
        } else if data_type_name == "Integer" {
            TagDataType::Integer
        } else if data_type_name == "Utf8" {
            TagDataType::Utf8
        } else if data_type_name == "Binary" {
            TagDataType::Binary
        } else if data_type_name == "Float" {
            TagDataType::Float
        } else if data_type_name == "Master" {
            TagDataType::Master
        } else {
            return Err(Error::new_spanned(node, format!("unrecognized #[data_type] value: {}", data_type_name)));
        };

        Ok(Variant {
            original: node,
            attributes: Attributes {
                id,
                data_type,
                data_type_val
            },
            ident: node.ident.clone(),
        })
    }
}