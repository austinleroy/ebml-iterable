#[cfg(feature = "derive-spec")]
pub mod spec_write_read {
    use ebml_iterable::specs::{ebml_specification, TagDataType, Master};
    use ebml_iterable::{TagIterator, TagWriter};
    use std::io::Cursor;
        
    #[ebml_specification]
    #[derive(Clone, Debug, PartialEq)]
    pub enum TestSpec {
        #[id(0x1a45dfa3)] 
        #[data_type(TagDataType::Master)]
        Ebml,

        #[id(0x18538067)]
        #[data_type(TagDataType::Master)]
        Segment,

        #[id(0x1F43B675)]
        #[data_type(TagDataType::Master)]
        Cluster,

        #[id(0x97)] 
        #[data_type(TagDataType::UnsignedInt)]
        CueRefCluster,

        #[id(0x4100)]
        #[data_type(TagDataType::UnsignedInt)]
        Count,

        #[id(0x83)] 
        #[data_type(TagDataType::UnsignedInt)]
        TrackType,

        #[id(0xa1)]
        #[data_type(TagDataType::Binary)]
        Block,

        #[id(0xa3)] 
        #[data_type(TagDataType::Binary)]
        SimpleBlock,
    }

    #[test]
    pub fn simple_read_write() {
        let tags: Vec<TestSpec> = vec![
            TestSpec::Ebml(Master::Start),
            TestSpec::Segment(Master::Start),
            TestSpec::TrackType(0x01),
            TestSpec::Segment(Master::End),
            TestSpec::Ebml(Master::End),
        ];

        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        for tag in tags.iter() {
            writer.write(tag).expect("Test shouldn't error");
        }

        println!("dest {:?}", dest);

        let mut src = Cursor::new(dest.get_ref().to_vec());
        let reader = TagIterator::new(&mut src, &[]);
        let read_tags: Vec<TestSpec> = reader.into_iter().map(|t| t.unwrap()).collect();

        println!("tags {:?}", read_tags);

        for i in 0..read_tags.len() {
            assert_eq!(tags[i], read_tags[i]);
        }
    }

    #[test]
    pub fn read_write_buffered_tag() {
        let tags: Vec<TestSpec> = vec![
            TestSpec::Ebml(Master::Start),
            TestSpec::Cluster(Master::Full(vec![TestSpec::CueRefCluster(0x02)])),
            TestSpec::Ebml(Master::End),
        ];

        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        for tag in tags.iter() {
            writer.write(tag).expect("Test shouldn't error");
        }

        println!("dest {:?}", dest);

        let mut src = Cursor::new(dest.get_ref().to_vec());
        let reader = TagIterator::new(&mut src, &[TestSpec::Cluster(Master::Start)]);
        let read_tags: Vec<TestSpec> = reader.into_iter().map(|t| t.unwrap()).collect();

        println!("tags {:?}", read_tags);

        for i in 0..read_tags.len() {
            assert_eq!(tags[i], read_tags[i]);
        }
    }

    #[test]
    pub fn oversized_tag() {
        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        writer.write(&TestSpec::Segment(Master::Start)).expect("Error writing tag");
        writer.write(&TestSpec::Cluster(Master::Start)).expect("Error writing tag");
        // Why 0x10001 specifically? This exceeds the default buffer length (0x10000)?!
        writer.write_raw(0xa1, &[0x00; 0x10001]).expect("Error writing tag");
        writer.write(&TestSpec::Count(0x00)).expect("Error writing tag");
        writer.write(&TestSpec::Cluster(Master::End)).expect("Error writing tag");
        writer.write(&TestSpec::Segment(Master::End)).expect("Error writing tag");
        drop(writer);

        dest.set_position(0);
        let iter = TagIterator::<_, TestSpec>::new(dest, &[]);

        let tags: Vec<_> = iter.into_iter().collect();
        assert_eq!(tags.len(), 4+2, "Reading every tag that was written");
    }
}