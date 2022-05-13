extern crate proc_macro;

mod ast;
mod attr;

use proc_macro::TokenStream;
use syn::{ItemEnum, Error};

///
/// Attribute that derives implementations of EbmlSpecification and EbmlTag for an enum.
/// 
/// This macro is intended to make implementing the traits in ebml-iterable-specification easier to manage.  Rather than requiring handwritten implementations for `EbmlSpecification` and `EbmlTag` methods, this macro understands attributes assigned to enum members and generates an implementation accordingly.
/// 
/// When deriving `EbmlSpecification` for an enum, the following attributes are required for each variant:
///   * __#[id(`u64`)]__ - This attribute specifies the "id" of the tag. e.g. `0x1a45dfa3`
///   * __#[data_type(`TagDataType`)]__ - This attribute specifies the type of data contained in the tag. e.g. `TagDataType::UnsignedInt`
/// 
/// # Note
///
/// This attribute modifies the variants in the enumeration by adding fields to them.  It also will add a `RawTag(u64, Vec<u8>)` variant to the enumeration.
///

#[proc_macro_attribute]
pub fn ebml_specification(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = match syn::parse::<ItemEnum>(input) {
        Ok(syntax_tree) => syntax_tree,
        Err(err) => {
            return TokenStream::from(Error::new(err.span(), "#[ebml_specification] attribute can only be applied to enums").to_compile_error())
        },
    };

    attr::impl_ebml_specification(&mut input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}