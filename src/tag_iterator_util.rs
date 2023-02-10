use ebml_iterable_specification::{EbmlSpecification, EbmlTag, PathPart};
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

    fn is_parent(&self, id: &u64) -> bool {
        let path = <TSpec>::get_path_by_tag(&self.tag);
        path.iter().any(|p| matches!(p, PathPart::Id(p) if p == id))
    }

    fn is_sibling(&self, compare: &u64) -> bool {
        <TSpec>::get_path_by_tag(&self.tag) == <TSpec>::get_path_by_id(*compare)
    }

    pub fn is_ended_by(&self, id: &u64) -> bool {
        // Unknown sized tags can be ended if we reach an element that is:
        //  - A parent of the tag
        //  - A direct sibling of the tag
        //  - A Root element

        self.is_parent(id) || // parent
            self.is_sibling(id) || // sibling
            ( // Root element
                <TSpec>::get_tag_data_type(*id).is_some() && 
                <TSpec>::get_path_by_id(*id).is_empty()
            )
    }
}

pub const DEFAULT_BUFFER_LEN: usize = 1024 * 64;

///
/// Used to relax rules on how strictly a [`TagIterator`](ebml_iterable::TagIterator) should validate the read stream.
/// 
pub enum AllowableErrors {
    ///
    /// Causes the [`TagIterator`](ebml_iterable::TagIterator) to produce "RawTag" binary variants for any unknown tag ids rather than throwing an error.
    /// 
    InvalidTagIds,

    ///
    /// Causes the [`TagIterator`](ebml_iterable::TagIterator) to emit tags even if they appear outside of their defined parent element.
    /// 
    HierarchyProblems,
}