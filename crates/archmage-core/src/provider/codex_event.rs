use crate::error::{Error, Result};
use serde_json::Value;

/// Byte-buffered JSONL decoder; UTF-8 is decoded only after a complete line.
pub struct Decoder {
    pending: Vec<u8>,
    max: usize,
}
impl Decoder {
    pub fn new(max: usize) -> Self {
        Self {
            pending: Vec::new(),
            max,
        }
    }
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Value>> {
        let mut events = Vec::new();
        for byte in bytes {
            if *byte == b'\n' {
                if let Some(event) = self.line()? {
                    events.push(event);
                }
            } else {
                if self.pending.len() >= self.max {
                    return Err(Error::Limit("JSONL line limit exceeded".into()));
                }
                self.pending.push(*byte);
            }
        }
        Ok(events)
    }
    pub fn finish(&mut self) -> Result<Vec<Value>> {
        Ok(self.line()?.into_iter().collect())
    }
    fn line(&mut self) -> Result<Option<Value>> {
        let line = std::mem::take(&mut self.pending);
        if line.iter().all(u8::is_ascii_whitespace) {
            return Ok(None);
        }
        let event: Value = serde_json::from_slice(&line)
            .map_err(|_| Error::Preflight("invalid Codex JSONL".into()))?;
        if !event.is_object() || event["type"].as_str().is_none() {
            return Err(Error::Preflight(
                "Codex event requires object and type".into(),
            ));
        }
        Ok(Some(event))
    }
}
