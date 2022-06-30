use ebml_iterable_specification::{EbmlSpecification, EbmlTag, Master};
use std::convert::TryInto;
use crate::tag_iterator_util::EBMLSize::{Known, Unknown};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum EBMLSize {
    Known(usize),
    Unknown
}

impl From<u64> for EBMLSize {
    fn from(size: u64) -> Self {
        Self::new(size)
    }
}

impl EBMLSize {

    pub fn new(size: u64) -> Self {
        const UNKNOWN: u64 = u64::MAX >> 8;
        if size == UNKNOWN {
            return Unknown
        } else {
            match size.try_into() {
                Ok(value) => Known(value),
                Err(_) => Unknown
            }
        }
    }

}

pub struct ProcessingTag<TSpec>
    where TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{
    pub tag: TSpec,
    pub size: EBMLSize,
    pub start: usize,    
}

impl<TSpec> ProcessingTag<TSpec> where TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone {

    pub fn get_id(&self) -> u64 {
        self.tag.get_id()
    }

    pub fn into_inner(self) -> TSpec {
        self.tag
    }

    pub fn is_parent(&self, id: u64) -> bool {
        let mut parent_id_opt = self.tag.get_parent_id();
        while let Some(parent_id) = parent_id_opt {
            if parent_id == id {
                return true;
            }
            parent_id_opt = TSpec::get_master_tag(parent_id, Master::Start).unwrap_or_else(|| panic!("Bad specification implementation: parent id {} type was not master!!", parent_id)).get_parent_id();
        }
        false
    }

    pub fn is_sibling(&self, compare: &TSpec) -> bool {
        self.tag.get_parent_id() == compare.get_parent_id()
    }
}

pub const DEFAULT_BUFFER_LEN: usize = 1024 * 64;
