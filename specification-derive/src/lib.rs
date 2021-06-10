extern crate proc_macro;

mod ast;
mod derive;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

///
/// Derives an implementation of EbmlSpecification for an enum.
/// 
/// This macro is intended to make implementing EbmlSpecification easier to manage.  Rather than requiring handwritten implementations for `EbmlSpecification` methods to properly return the tag id, data type, and name per method, this macro understands attributes assigned to enum members and generates an implementation accordingly.
/// 
/// When deriving `EbmlSpecification` for an enum, the following attributes are required for each variant:
///   * __#[id(`u64`)]__ - This attribute specifies the "id" of the tag. e.g. `0x1a45dfa3`
///   * __#[data_type(`TagDataType`)]__ - This attribute specifies the type of data contained in the tag. e.g. `TagDataType::UnsignedInt`
/// 
#[proc_macro_derive(EbmlSpecification, attributes(id, data_type))]
pub fn ebml_specification_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    derive::impl_ebml_specification_macro(&input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}