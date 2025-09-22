use futures::{Stream, StreamExt};
use reqwest::Response;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{debug, error, warn};

use crate::error::{ProviderError, Result};
use crate::models::ChatCompletionChunk;

/// A stream of Server-Sent Events from OpenAI's streaming API
pub struct SseStream {
    inner: Pin<Box<dyn Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>,
    buffer: String,
}

impl SseStream {
    pub fn new(response: Response) -> Self {
        let stream = response.bytes_stream();

        Self {
            inner: Box::pin(stream),
            buffer: String::new(),
        }
    }

    /// Parse SSE events and extract OpenAI chat completion chunks
    pub fn parse_events(self) -> impl Stream<Item = Result<ChatCompletionChunk>> {
        self.filter_map(|line_result| async move {
            match line_result {
                Ok(line) => {
                    if line.starts_with("data: ") {
                        let data = &line[6..]; // Remove "data: " prefix

                        if data == "[DONE]" {
                            debug!("Received stream completion marker");
                            return None;
                        }

                        match serde_json::from_str::<ChatCompletionChunk>(data) {
                            Ok(chunk) => {
                                debug!("Parsed SSE chunk: {:?}", chunk.id);
                                Some(Ok(chunk))
                            }
                            Err(e) => {
                                warn!("Failed to parse SSE chunk: {}, data: {}", e, data);
                                Some(Err(ProviderError::StreamError {
                                    operation: "parse_chunk".to_string(),
                                    reason: format!("Failed to parse chunk: {}", e),
                                }))
                            }
                        }
                    } else if line.starts_with("event: ") || line.is_empty() {
                        // Skip event lines and empty lines
                        None
                    } else {
                        warn!("Unexpected SSE line format: {}", line);
                        None
                    }
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    Some(Err(e))
                }
            }
        })
    }

    /// Extract content deltas from the stream
    pub fn content_stream(self) -> impl Stream<Item = Result<String>> {
        self.parse_events().filter_map(|chunk_result| async move {
            match chunk_result {
                Ok(chunk) => {
                    // Extract content from the first choice's delta
                    if let Some(choice) = chunk.choices.first() {
                        if let Some(content) = &choice.delta.content {
                            Some(Ok(content.clone()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            }
        })
    }
}

impl Stream for SseStream {
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    // Convert bytes to string and add to buffer
                    match String::from_utf8(bytes.to_vec()) {
                        Ok(text) => {
                            self.buffer.push_str(&text);

                            // Process complete lines
                            if let Some(newline_pos) = self.buffer.find('\n') {
                                let line = self.buffer[..newline_pos].trim().to_string();
                                self.buffer = self.buffer[newline_pos + 1..].to_string();

                                if !line.is_empty() {
                                    return Poll::Ready(Some(Ok(line)));
                                }
                                // Continue to next iteration for more lines
                            } else {
                                // No complete line yet, continue polling
                                continue;
                            }
                        }
                        Err(e) => {
                            return Poll::Ready(Some(Err(ProviderError::StreamError {
                                operation: "decode_utf8".to_string(),
                                reason: format!("Invalid UTF-8 in stream: {}", e),
                            })));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(ProviderError::Http {
                        operation: "stream_chunk_read".to_string(),
                        source: e,
                    })));
                }
                Poll::Ready(None) => {
                    // Check if there's remaining data in buffer
                    if !self.buffer.is_empty() {
                        let line = self.buffer.trim().to_string();
                        self.buffer.clear();
                        if !line.is_empty() {
                            return Poll::Ready(Some(Ok(line)));
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_sse_parsing() {
        // This would test SSE parsing with mock data
        // Implementation details would depend on your testing strategy
    }
}
