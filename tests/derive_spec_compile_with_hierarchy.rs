#[cfg(feature = "derive-spec")]
pub mod derive_spec_compile {
    use ebml_iterable::specs::{ebml_specification, TagDataType, Master, EbmlSpecification};
    
    #[ebml_specification]
    #[derive(Clone, Debug, PartialEq)]
    pub enum Trial {
        #[id(0x01)]
        #[data_type(TagDataType::Master)]
        Root,

        #[id(0x02)]
        #[data_type(TagDataType::Master)]
        #[parent(Root)]
        Parent,

        #[id(0x100)]
        #[data_type(TagDataType::UnsignedInt)]
        #[parent(Parent)]
        Count,

        #[id(0x200)]
        #[data_type(TagDataType::Binary)]
        #[parent(Parent)]
        Data,

        #[id(0x201)]
        #[data_type(TagDataType::Utf8)]
        #[parent(Parent)]
        Name,

        #[id(0x102)]
        #[data_type(TagDataType::Float)]
        #[parent(Parent)]
        Amount,

        #[id(0x101)]
        #[data_type(TagDataType::Integer)]
        #[parent(Parent)]
        Id,  
    }

    #[test]
    pub fn compile_worked() {
        let data_type = Trial::get_tag_data_type(0x01);
        assert_eq!(Some(TagDataType::Master), data_type);
        
        let tag = Trial::get_master_tag(0x01, Master::Start).unwrap();
        assert_eq!(Trial::Root(Master::Start), tag);
    }
}