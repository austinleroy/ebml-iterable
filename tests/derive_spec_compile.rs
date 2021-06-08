#[cfg(feature = "derive-spec")]
pub mod derive_spec_compile {
    use ebml_iterable::specs::{EbmlSpecification, TagDataType};
    
    #[derive(EbmlSpecification, Debug, Eq, PartialEq)]
    pub enum Trial {
        #[id(0x01)]
        #[data_type(TagDataType::Master)]
        Root,
    
        #[id(0x02)]
        #[data_type(TagDataType::Master)]
        Parent,
    
        #[id(0x100)]
        #[data_type(TagDataType::UnsignedInt)]
        Count,
    
        #[id(0x200)]
        #[data_type(TagDataType::Binary)]
        Data,    
    }

    #[test]
    pub fn compile_worked() {
        let item = Trial::get_tag(0x01).unwrap();
        let tag = item.0;
        let data_type = item.1;
        assert_eq!(Trial::Root, tag);
        assert_eq!(TagDataType::Master, data_type);
    }
}