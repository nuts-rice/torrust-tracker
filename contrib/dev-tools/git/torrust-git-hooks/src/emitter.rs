// NDJSON writer
//
// Every event is serialized to single line

use std::io::{self, Write};

use crate::event::{Event, Hook, Record};

pub struct Emitter<W: Write> {
    hook: Option<Hook>,
    writer: W,
}

impl<W: Write> Emitter<W> {
    pub const fn new(hook: Hook, writer: W) -> Self {
        Self {
            hook: Some(hook),
            writer,
        }
    }

    pub const fn global(writer: W) -> Self {
        Self { hook: None, writer }
    }

    /// Serialise `event` as one line and flush it.
    ///
    /// # Errors
    ///
    /// Returns an error if serialisation fails, or if the underlying writer rejects the
    /// write or the flush.
    pub fn emit(&mut self, event: &Event) -> io::Result<()> {
        let line = self
            .hook
            .map_or_else(
                || serde_json::to_string(&Record::global(event)),
                |hook| serde_json::to_string(&Record::new(hook, event)),
            )
            .map_err(io::Error::other)?;

        writeln!(self.writer, "{line}")?;
        self.writer.flush()
    }
}
