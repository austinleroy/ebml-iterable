use ebml_iterable_specification::{EbmlSpecification, EbmlTag};
use std::convert::TryInto;
use crate::tag_iterator_util::EBMLSize::{Known, Unknown};
use crate::tag_iterator_util::ProcessingTag::{EndTag, NextTag};

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

pub enum ProcessingTag<TSpec>
    where TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{
    EndTag {
        tag: TSpec,
        size: EBMLSize,
        start: usize,
    },
    NextTag {
        tag: TSpec,
    }
}

impl<TSpec> ProcessingTag<TSpec> where TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone {

    pub fn get_id(&self) -> u64 {
        match self {
            EndTag { tag,.. } => tag.get_id(),
            NextTag { tag } => tag.get_id()
        }
    }

    pub fn into_inner(self) -> TSpec {
        match self {
            EndTag { tag,.. } => tag,
            NextTag { tag } => tag
        }
    }

    pub fn inner(&self) -> &TSpec {
        match self {
            EndTag { tag,.. } => tag,
            NextTag { tag } => tag
        }
    }
}

pub const DEFAULT_BUFFER_LEN: usize = 1024 * 64;
