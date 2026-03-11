use color_eyre::Result;
use crossterm::event::{Event, EventStream};
use futures::{FutureExt, StreamExt};

#[derive(Debug)]
pub struct EventHandler {
    event_stream: EventStream,
}

impl Default for EventHandler {
    fn default() -> Self {
        Self {
            event_stream: EventStream::new(),
        }
    }
}

impl EventHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn next_event(&mut self) -> Result<Option<Event>> {
        match self.event_stream.next().fuse().await {
            Some(Ok(event)) => Ok(Some(event)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}
