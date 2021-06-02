#[derive(PartialEq, Debug, Clone)]
pub enum EbmlTag {
    StartTag(u64),
    EndTag(u64),
    FullTag(DataTag),
}

impl EbmlTag {
    pub fn start_tag(&self) -> Option<u64> {
        match &self {
            EbmlTag::StartTag(id) => Some(*id),
            _ => None
        }
    }

    pub fn end_tag(&self) -> Option<u64> {
        match &self {
            EbmlTag::EndTag(id) => Some(*id),
            _ => None
        }
    }

    pub fn full_tag(self) -> Option<DataTag> {
        match self {
            EbmlTag::FullTag(data) => Some(data),
            _ => None
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct DataTag {
    pub id: u64,
    pub data_type: DataTagType,
}

#[derive(PartialEq, Debug, Clone)]
pub enum DataTagType {
    Master(Vec<DataTag>),
    UnsignedInt(u64),
    Integer(i64),
    Utf8(String),
    Binary(Vec<u8>),
    Float(f64),
}

impl DataTagType {
    pub fn master(self) -> Option<Vec<DataTag>> {
        match self {
            DataTagType::Master(children) => Some(children),
            _ => None
        }
    }

    pub fn unsigned_int(&self) -> Option<u64> {
        match &self {
            DataTagType::UnsignedInt(val) => Some(*val),
            _ => None
        }
    }

    pub fn integer(&self) -> Option<i64> {
        match &self {
            DataTagType::Integer(val) => Some(*val),
            _ => None
        }
    }

    pub fn utf8(self) -> Option<String> {
        match self {
            DataTagType::Utf8(val) => Some(val),
            _ => None
        }
    }

    pub fn binary(self) -> Option<Vec<u8>> {
        match self {
            DataTagType::Binary(val) => Some(val),
            _ => None
        }
    }

    pub fn float(&self) -> Option<f64> {
        match &self {
            DataTagType::Float(val) => Some(*val),
            _ => None
        }
    }
}