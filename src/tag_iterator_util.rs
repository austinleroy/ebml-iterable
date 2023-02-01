use ebml_iterable_specification::{EbmlSpecification, EbmlTag, Master};
use std::convert::TryInto;
use crate::tag_iterator_util::EBMLSize::{Known, Unknown};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum EBMLSize {
    Known(usize),
    Unknown
}

impl EBMLSize {
    pub fn new(size: u64, vint_length: usize) -> Self {
        match vint_length {
            1 => if size == ((1 << (7))     - 1) { return Unknown; },
            2 => if size == ((1 << (7 * 2)) - 1) { return Unknown; },
            3 => if size == ((1 << (7 * 3)) - 1) { return Unknown; },
            4 => if size == ((1 << (7 * 4)) - 1) { return Unknown; },
            5 => if size == ((1 << (7 * 5)) - 1) { return Unknown; },
            6 => if size == ((1 << (7 * 6)) - 1) { return Unknown; },
            7 => if size == ((1 << (7 * 7)) - 1) { return Unknown; },
            8 => if size == ((1 << (7 * 8)) - 1) { return Unknown; },
            _ => {},
        }

        match size.try_into() {
            Ok(value) => Known(value),
            Err(_) => Unknown
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ProcessingTag<TSpec>
    where TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{
    pub tag: TSpec,
    pub size: EBMLSize,
    pub tag_start: usize,
    pub data_start: usize,
}

impl<TSpec> ProcessingTag<TSpec> where TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone {
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
