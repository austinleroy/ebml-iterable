mod test_spec;

pub mod spec_write_read {
    use ebml_iterable::specs::{Master, EbmlTag};
    use ebml_iterable::{TagIterator, TagWriter};
    use std::io::Cursor;

    use super::test_spec::TestSpec;

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

    #[test]
    pub fn write_unknown_size() {
        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        writer.write(&TestSpec::Root(Master::Start)).unwrap();
        writer.write_unknown_size(&TestSpec::Parent(Master::Start)).unwrap();
        writer.write(&TestSpec::Child(1)).unwrap();
        writer.write(&TestSpec::Child(2)).unwrap();
        writer.write(&TestSpec::Parent(Master::End)).unwrap();
        writer.write(&TestSpec::Root(Master::End)).unwrap();

        dest.set_position(0);
        
        let iter = TagIterator::<_, TestSpec>::new(dest, &[]);
        let tags: Vec<_> = iter.into_iter().collect();
        assert_eq!(tags.len(), 6, "Reading every tag that was written");
    }

    #[test]
    pub fn buffer_unknown_size() {
        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        writer.write(&TestSpec::Root(Master::Start)).unwrap();
        writer.write_unknown_size(&TestSpec::Parent(Master::Start)).unwrap();
        writer.write(&TestSpec::Child(1)).unwrap();
        writer.write(&TestSpec::Child(2)).unwrap();
        writer.write(&TestSpec::Parent(Master::End)).unwrap();
        writer.write(&TestSpec::Root(Master::End)).unwrap();

        dest.set_position(0);
        
        let iter = TagIterator::<_, TestSpec>::new(dest, &[TestSpec::Parent(Master::Start)]);
        let mut tags: Vec<_> = iter.into_iter().collect();
        assert_eq!(tags.len(), 3, "Buffering 'Parent' into full variant");
        
        tags.pop();
        let parent = tags.pop().unwrap().unwrap();
        assert!(matches!(parent.as_master(), Some(Master::Full(c)) if c.len() == 2), "Did not buffer tag as master with 2 children");
    }

    #[test]
    pub fn unknown_size_write_read() {
        let mut dest = Cursor::new(Vec::new());
        let mut writer = TagWriter::new(&mut dest);

        writer.write(&TestSpec::Root(Master::Start)).unwrap();
        writer.write_unknown_size(&TestSpec::Parent(Master::Start)).unwrap();
        writer.write(&TestSpec::Child(1)).unwrap();
        writer.write(&TestSpec::Child(2)).unwrap();
        writer.write(&TestSpec::Parent(Master::End)).unwrap();
        writer.write(&TestSpec::Int(2)).unwrap();
        writer.write(&TestSpec::Root(Master::End)).unwrap();

        dest.set_position(0);
        
        let mut iter = TagIterator::<_, TestSpec>::new(dest, &[]);
        assert!(matches!(iter.next(), Some(Ok(TestSpec::Root(Master::Start)))));
        assert!(matches!(iter.next(), Some(Ok(TestSpec::Parent(Master::Start)))));
        assert!(matches!(iter.next(), Some(Ok(TestSpec::Child(1)))));
        assert!(matches!(iter.next(), Some(Ok(TestSpec::Child(2)))));
        assert!(matches!(iter.next(), Some(Ok(TestSpec::Parent(Master::End)))));
        assert!(matches!(iter.next(), Some(Ok(TestSpec::Int(2)))));
        assert!(matches!(iter.next(), Some(Ok(TestSpec::Root(Master::End)))));
        assert!(matches!(iter.next(), None));
    }
}