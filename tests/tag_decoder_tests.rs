mod test_spec;

use std::io::Cursor;

use bytes::BytesMut;
use ebml_iterable::error::{CorruptedFileError, TagIteratorError};
use ebml_iterable::iterator::AllowableErrors;
use ebml_iterable::specs::Master;
use ebml_iterable::{PositionedTag, TagDecoder, TagIterator, TagWriter};

use test_spec::TestSpec;

#[test]
fn waits_for_a_complete_element() {
    let mut decoder = TagDecoder::<TestSpec>::new(&[]);
    let mut input = BytesMut::from(&[0x83][..]);

    assert_eq!(decoder.decode(&mut input).unwrap(), None);
    assert_eq!(decoder.position(), 0);

    input.extend_from_slice(&[0x82, 0x01]);
    assert_eq!(decoder.decode(&mut input).unwrap(), None);
    assert_eq!(decoder.position(), 0);

    input.extend_from_slice(&[0x02]);
    let tag = decoder.decode(&mut input).unwrap().unwrap();
    assert_eq!(tag.tag, TestSpec::TrackType(0x0102));
    assert_eq!(tag.offset, 0);
    assert_eq!(decoder.position(), 4);
    assert!(input.is_empty());
}

#[test]
fn decodes_a_nested_stream_one_byte_at_a_time() {
    let source = [
        0x18, 0x53, 0x80, 0x67, 0xff, 0x1f, 0x43, 0xb6, 0x75, 0xff, 0x41, 0x00, 0x40, 0x01, 0x2a,
    ];
    let mut decoder = TagDecoder::<TestSpec>::new(&[]);
    let mut input = BytesMut::new();
    let mut decoded = Vec::new();

    for byte in source {
        input.extend_from_slice(&[byte]);
        while let Some(tag) = decoder.decode(&mut input).unwrap() {
            decoded.push(tag);
        }
    }
    while !decoder.is_finished() {
        if let Some(tag) = decoder.decode_eof(&mut input).unwrap() {
            decoded.push(tag);
        }
    }

    assert_eq!(
        decoded,
        vec![
            PositionedTag {
                tag: TestSpec::Segment(Master::Start),
                offset: 0,
            },
            PositionedTag {
                tag: TestSpec::Cluster(Master::Start),
                offset: 5,
            },
            PositionedTag {
                tag: TestSpec::Count(42),
                offset: 10,
            },
            PositionedTag {
                tag: TestSpec::Cluster(Master::End),
                offset: 5,
            },
            PositionedTag {
                tag: TestSpec::Segment(Master::End),
                offset: 0,
            },
        ]
    );
}

#[test]
fn reports_partial_data_only_at_eof() {
    let mut decoder = TagDecoder::<TestSpec>::new(&[]);
    let mut input = BytesMut::from(&[0x83, 0x82, 0x01][..]);

    assert_eq!(decoder.decode(&mut input).unwrap(), None);
    assert!(matches!(
        decoder.decode_eof(&mut input),
        Err(TagIteratorError::UnexpectedEOF {
            tag_start: 0,
            tag_id: Some(0x83),
            tag_size: Some(2),
            partial_data: Some(data),
        }) if data == [0x01]
    ));
}

#[test]
fn unknown_first_tag_does_not_lock_the_document_path() {
    let mut decoder = TagDecoder::<TestSpec>::new(&[]);
    decoder.allow_errors(&[AllowableErrors::InvalidTagIds]);
    let mut input = BytesMut::from(&[0xf2, 0x81, 0x01, 0x83, 0x81, 0x01][..]);

    assert_eq!(
        decoder.decode(&mut input).unwrap().unwrap().tag,
        TestSpec::RawTag(0xf2, vec![0x01])
    );
    assert_eq!(
        decoder.decode(&mut input).unwrap().unwrap().tag,
        TestSpec::TrackType(1)
    );
}

#[test]
fn invalid_next_header_does_not_close_unknown_masters() {
    let mut decoder = TagDecoder::<TestSpec>::new(&[]);
    decoder.set_max_allowable_tag_size(Some(4));
    let mut input = BytesMut::from(
        &[
            0x18, 0x53, 0x80, 0x67, 0xff, 0x1f, 0x43, 0xb6, 0x75, 0xff,
        ][..],
    );

    assert!(matches!(
        decoder.decode(&mut input).unwrap().unwrap().tag,
        TestSpec::Segment(Master::Start)
    ));
    assert!(matches!(
        decoder.decode(&mut input).unwrap().unwrap().tag,
        TestSpec::Cluster(Master::Start)
    ));

    input.extend_from_slice(&[0x83, 0x85]);
    for _ in 0..2 {
        assert!(matches!(
            decoder.decode(&mut input),
            Err(TagIteratorError::CorruptedFileData(
                CorruptedFileError::InvalidTagSize {
                    position: 10,
                    tag_id: 0x83,
                    size: 5,
                }
            ))
        ));
        assert_eq!(decoder.position(), 10);
    }
}

#[test]
fn shared_parser_handles_empty_unsigned_integers() {
    let bytes = vec![0x83, 0x80];
    let mut iterator = TagIterator::<_, TestSpec>::new(Cursor::new(bytes.clone()), &[]);
    assert_eq!(iterator.next().unwrap().unwrap(), TestSpec::TrackType(0));

    let mut decoder = TagDecoder::<TestSpec>::new(&[]);
    let mut input = BytesMut::from(bytes.as_slice());
    assert_eq!(
        decoder.decode(&mut input).unwrap().unwrap().tag,
        TestSpec::TrackType(0)
    );
}

#[test]
fn shared_parser_handles_empty_signed_integers_and_floats() {
    let bytes = vec![0x84, 0x80, 0x85, 0x80];
    let mut iterator = TagIterator::<_, TestSpec>::new(Cursor::new(bytes.clone()), &[]);
    assert_eq!(iterator.next().unwrap().unwrap(), TestSpec::Signed(0));
    assert_eq!(iterator.next().unwrap().unwrap(), TestSpec::Float(0.0));

    let mut decoder = TagDecoder::<TestSpec>::new(&[]);
    let mut input = BytesMut::from(bytes.as_slice());
    assert_eq!(
        decoder.decode(&mut input).unwrap().unwrap().tag,
        TestSpec::Signed(0)
    );
    assert_eq!(
        decoder.decode(&mut input).unwrap().unwrap().tag,
        TestSpec::Float(0.0)
    );
}

#[test]
fn buffers_selected_master_tags() {
    let tags = [
        TestSpec::Root(Master::Start),
        TestSpec::Int(42),
        TestSpec::Root(Master::End),
    ];
    let mut destination = Cursor::new(Vec::new());
    let mut writer = TagWriter::new(&mut destination);
    for tag in &tags {
        writer.write(tag).unwrap();
    }
    drop(writer);

    let mut decoder = TagDecoder::new(&[TestSpec::Root(Master::Start)]);
    let mut input = BytesMut::from(destination.into_inner().as_slice());
    let decoded = decoder.decode(&mut input).unwrap().unwrap();
    assert_eq!(
        decoded.tag,
        TestSpec::Root(Master::Full(vec![TestSpec::Int(42)]))
    );
    assert_eq!(decoded.offset, 0);
}

#[test]
fn static_iterator_keeps_partial_header_id() {
    let mut iterator = TagIterator::<_, TestSpec>::new(Cursor::new(vec![0x83]), &[]);
    assert!(matches!(
        iterator.next(),
        Some(Err(TagIteratorError::UnexpectedEOF {
            tag_start: 0,
            tag_id: Some(0x83),
            tag_size: None,
            partial_data: None,
        }))
    ));
}
