pub enum SpecTagType {
    Master,
    UnsignedInt,
    Integer,
    Utf8,
    Binary,
    Float,
}

pub trait TagSpec {
    type SpecType: Copy;

    fn get_tag(&self, id: u64) -> Self::SpecType;
    fn get_tag_type(&self, tag: &Self::SpecType) -> SpecTagType;
}