
use bytes::BytesMut;
use ebml_iterable_specification::{EbmlSpecification, EbmlTag};
use futures::{AsyncRead, AsyncReadExt, Stream};

use crate::error::TagIteratorError;
use crate::iterator::AllowableErrors;
use crate::TagDecoder;

///
/// This can be transformed into a [`Stream`] using [`into_stream`][TagIteratorAsync::into_stream], or consumed directly by calling [`.next().await`] in a loop.
///
/// The struct can be created with the [`new()`][TagIteratorAsync::new] function on any source that implements the [`futures::AsyncRead`] trait.
///
pub struct TagIteratorAsync<R: AsyncRead + Unpin, TSpec>
    where
        TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{
    source: R,
    input: BytesMut,
    decoder: TagDecoder<TSpec>,
    last_emitted_tag_offset: usize,
    buffer: Box<[u8]>,
}

impl<R: AsyncRead + Unpin, TSpec> TagIteratorAsync<R, TSpec>
    where
        TSpec: EbmlSpecification<TSpec> + EbmlTag<TSpec> + Clone
{

    pub fn new(source: R, tags_to_buffer: &[TSpec]) -> Self {
        let buffer = vec![0u8; 1024 * 64];
        Self {
            source,
            input: BytesMut::new(),
            decoder: TagDecoder::new(tags_to_buffer),
            last_emitted_tag_offset: 0,
            buffer: buffer.into_boxed_slice(),
        }
    }

    pub async fn next(&mut self) -> Option<Result<TSpec, TagIteratorError>> {
        loop {
            if let Some(tag) = self.decoder.decode(&mut self.input).ok().flatten() {
                self.last_emitted_tag_offset = tag.offset;
                return Some(Ok(tag.tag));
            }

            match self.source.read(&mut self.buffer).await {
                Ok(0) => {
                    match self.decoder.decode_eof(&mut self.input) {
                        Ok(Some(tag)) => {
                            self.last_emitted_tag_offset = tag.offset;
                            return Some(Ok(tag.tag));
                        }
                        Ok(None) => return None,
                        Err(err) => return Some(Err(err)),
                    }
                }
                Ok(len) => {
                    self.input.extend_from_slice(&self.buffer[..len]);
                }
                Err(err) => return Some(Err(TagIteratorError::ReadError { source: err })),
            }
        }
    }

    pub fn allow_errors(&mut self, errors: &[AllowableErrors]) {
        self.decoder.allow_errors(errors);
    }

    pub fn set_max_allowable_tag_size(&mut self, size: Option<usize>) {
        self.decoder.set_max_allowable_tag_size(size);
    }

    /// Attempts to recover from corrupted data on the underlying async source.
    ///
    /// This will read additional bytes from the underlying `AsyncRead` source
    /// until `TagDecoder::try_recover` succeeds or EOF/IO error occurs.
    pub async fn try_recover(&mut self) -> Result<(), TagIteratorError> {
        loop {
            if self.decoder.try_recover(&mut self.input) {
                return Ok(())
            } else {
                // Need more bytes from source; attempt to read.
                match self.source.read(&mut self.buffer).await {
                    Ok(0) => return Err(TagIteratorError::UnexpectedEOF { tag_start: self.decoder.position(), tag_id: None, tag_size: None, partial_data: None }),
                    Ok(len) => {
                        self.input.extend_from_slice(&self.buffer[..len]);
                    }
                    Err(e) => return Err(TagIteratorError::ReadError { source: e }),
                }
            }
        }
    }

    pub fn into_stream(self) -> impl Stream<Item=Result<TSpec, TagIteratorError>> {
        futures::stream::unfold(self, |mut read| async {
            let next = read.next().await;
            next.map(move |it| (it, read))
        })
    }

    pub fn last_emitted_tag_offset(&self) -> usize {
        self.last_emitted_tag_offset
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use ebml_iterable_specification::empty_spec::EmptySpec;
    use futures::{executor::block_on, stream::StreamExt};

    use super::*;
    use crate::TagWriter;

    struct ChunkedAsyncReader {
        chunks: VecDeque<Vec<u8>>,
    }

    impl AsyncRead for ChunkedAsyncReader {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let this = self.get_mut();
            let Some(chunk) = this.chunks.front_mut() else {
                return Poll::Ready(Ok(0));
            };

            let len = std::cmp::min(chunk.len(), buf.len());
            buf[..len].copy_from_slice(&chunk[..len]);
            chunk.drain(..len);
            if chunk.is_empty() {
                this.chunks.pop_front();
            }

            Poll::Ready(Ok(len))
        }
    }

    fn encode_tag(id: u64, payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut writer = TagWriter::new(&mut bytes);
        writer.write(&EmptySpec::with_data(id, payload)).unwrap();
        drop(writer);
        bytes
    }

    #[test]
    fn async_next_reads_across_multiple_chunks() {
        let bytes = encode_tag(0x81, b"hello");
        let chunks = bytes.chunks(3).map(|chunk| chunk.to_vec()).collect();
        let mut iterator = TagIteratorAsync::<_, EmptySpec>::new(ChunkedAsyncReader { chunks }, &[]);

        let tag = block_on(iterator.next()).unwrap().unwrap();
        assert_eq!(tag.get_id(), 0x81);
        assert_eq!(tag.as_binary(), Some(b"hello".as_slice()));
        assert!(block_on(iterator.next()).is_none());
    }

    #[test]
    fn async_stream_emits_values_in_order() {
        let bytes = [encode_tag(0x81, b"a"), encode_tag(0x82, b"b")].concat();
        let chunks = bytes.chunks(2).map(|chunk| chunk.to_vec()).collect();
        let stream = TagIteratorAsync::<_, EmptySpec>::new(ChunkedAsyncReader { chunks }, &[]).into_stream();

        let values: Vec<_> = block_on(stream.collect());
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].as_ref().unwrap().get_id(), 0x81);
        assert_eq!(values[1].as_ref().unwrap().get_id(), 0x82);
        assert_eq!(values[0].as_ref().unwrap().as_binary(), Some(b"a".as_slice()));
        assert_eq!(values[1].as_ref().unwrap().as_binary(), Some(b"b".as_slice()));
    }
}
