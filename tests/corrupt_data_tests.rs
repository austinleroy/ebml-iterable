mod test_spec;

pub mod corrupt_data_tests {
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
        assert!(matches!(reader.next().unwrap(), Err(TagIteratorError::CorruptedFileData(CorruptedFileError::InvalidTagId{ tag_id: _, position: _ }))));
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

    fn get_data_with_oversized_child() -> Cursor<Vec<u8>> {
        let tags: Vec<TestSpec> = vec![
            TestSpec::Segment(Master::Start),
            TestSpec::Cluster(Master::Start),
            TestSpec::Block(vec![0x01, 0x02, 0x03, 0x04]),
            TestSpec::Cluster(Master::End),
            TestSpec::Segment(Master::End),
        ];

        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        for tag in tags.iter() {
            writer.write(tag).expect("Test shouldn't error");
        }

        // Extend size of block element without resizing parents
        dest.get_mut()[11] = 0x86;
        dest.get_mut().push(0x0a);
        dest.get_mut().push(0x0a);

        println!("dest {:x?}", dest);
        dest.set_position(0);
        dest
    }

    #[test]
    pub fn error_on_oversized_child() {
        let mut cursor = get_data_with_oversized_child();
        let mut reader: TagIterator<_, TestSpec> = TagIterator::new(&mut cursor, &[]);
        assert!(reader.next().unwrap().is_ok());
        assert!(reader.next().unwrap().is_ok());
        assert!(matches!(reader.next().unwrap(), Err(TagIteratorError::CorruptedFileData(CorruptedFileError::OversizedChildElement{position: _, tag_id: _, size: _}))));
    }

    #[test]
    pub fn allow_errors_oversized_child() {
        let mut cursor = get_data_with_oversized_child();
        let mut reader: TagIterator<_, TestSpec> = TagIterator::new(&mut cursor, &[]);
        reader.allow_errors(&[AllowableErrors::OversizedTags]);
        reader.for_each(|t| assert!(t.is_ok()));
    }

    #[test]
    pub fn recover_on_global_element() {
        let tags: Vec<TestSpec> = vec![
            TestSpec::Segment(Master::Start),
            TestSpec::Cluster(Master::Start),
            TestSpec::Crc32(vec![0x01]),
            TestSpec::Count(1),
            TestSpec::Cluster(Master::End),
            TestSpec::Segment(Master::End),
        ];

        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        for tag in tags.iter() {
            writer.write(tag).expect("Test shouldn't error");
        }

        // Inserting some junk data to skip
        dest.get_mut().insert(10, 0x0a);
        dest.get_mut().insert(10, 0x0a);
        dest.get_mut().insert(10, 0x0a);
        dest.set_position(0);
        
        println!("dest {:x?}", dest);
        
        let mut reader: TagIterator<_, TestSpec> = TagIterator::new(&mut dest, &[]);
        assert!(matches!(reader.next(), Some(t) if t.is_ok()));
        assert!(matches!(reader.next(), Some(t) if t.is_ok()));
        assert!(matches!(reader.next(), Some(t) if t.is_err()));
        assert!(reader.try_recover().is_ok());
        reader.for_each(|t| 
            if let Err(err) = t {
                println!("{err:?}");
                assert!(false);
            }
        );
    }
}