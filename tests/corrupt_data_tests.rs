mod test_spec;

pub mod spec_write_read {
    use ebml_iterable::error::{TagIteratorError, CorruptedFileError};
    use ebml_iterable::iterator::AllowableErrors;
    use ebml_iterable::specs::Master;
    use ebml_iterable::{TagIterator, TagWriter};
    use std::io::Cursor;

    use super::test_spec::TestSpec;

    fn get_data_with_invalid_ids() -> Cursor<Vec<u8>> {
        let tags: Vec<TestSpec> = vec![
            TestSpec::Segment(Master::Start),
            TestSpec::TrackType(0x01),
            TestSpec::RawTag(0xf2, vec![0x01]),
            TestSpec::Segment(Master::End),
        ];

        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        for tag in tags.iter() {
            writer.write(tag).expect("Test shouldn't error");
        }

        println!("dest {:x?}", dest);
        dest.set_position(0);
        dest
    }

    #[test]
    pub fn error_on_invalid_ids() {
        let mut cursor = get_data_with_invalid_ids();
        let mut reader: TagIterator<_, TestSpec> = TagIterator::new(&mut cursor, &[]);
        assert!(reader.next().unwrap().is_ok());
        assert!(reader.next().unwrap().is_ok());
        assert!(matches!(reader.next().unwrap(), Err(TagIteratorError::CorruptedFileData(CorruptedFileError::InvalidTagId(_)))));
    }

    #[test]
    pub fn allow_errors_invalid_ids() {
        let mut cursor = get_data_with_invalid_ids();
        let mut reader: TagIterator<_, TestSpec> = TagIterator::new(&mut cursor, &[]);
        reader.allow_errors(&[AllowableErrors::InvalidTagIds]);
        reader.for_each(|t| assert!(t.is_ok()));
    }

    fn get_data_with_hierarchy_problems() -> Cursor<Vec<u8>> {
        let tags: Vec<TestSpec> = vec![
            TestSpec::Segment(Master::Start),
            TestSpec::Count(1),
            TestSpec::Segment(Master::End),
        ];

        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        for tag in tags.iter() {
            writer.write(tag).expect("Test shouldn't error");
        }

        println!("dest {:x?}", dest);
        dest.set_position(0);
        dest
    }

    #[test]
    pub fn error_on_hierarchy_problems() {
        let mut cursor = get_data_with_hierarchy_problems();
        let mut reader: TagIterator<_, TestSpec> = TagIterator::new(&mut cursor, &[]);
        assert!(reader.next().unwrap().is_ok());
        assert!(matches!(reader.next().unwrap(), Err(TagIteratorError::CorruptedFileData(CorruptedFileError::HierarchyError{found_tag_id: _, current_parent_id: _}))));
    }

    #[test]
    pub fn allow_errors_hierarchy_problems() {
        let mut cursor = get_data_with_hierarchy_problems();
        let mut reader: TagIterator<_, TestSpec> = TagIterator::new(&mut cursor, &[]);
        reader.allow_errors(&[AllowableErrors::HierarchyProblems]);
        reader.for_each(|t| assert!(t.is_ok()));
    }
}