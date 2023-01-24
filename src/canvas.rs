use std::io::{stdout, Stdout, Write};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use crossterm::{cursor, terminal, ExecutableCommand, QueueableCommand};

pub struct Canvas {
    stdout: Stdout,
    delay: Option<Duration>,
}

impl Canvas {
    pub fn new() -> Result<Self> {
        let mut stdout = stdout();
        stdout.execute(cursor::Hide)?;
        stdout.queue(terminal::Clear(terminal::ClearType::All))?;
        stdout.queue(cursor::MoveToRow(0))?;
        Ok(Self {
            stdout,
            delay: None,
        })
    }
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
    pub fn draw(&mut self, text: &str) -> Result<()> {
        self.stdout.queue(cursor::SavePosition)?;
        self.stdout.write_all(text.as_bytes())?;
        self.stdout.queue(cursor::RestorePosition)?;
        self.stdout.flush()?;
        if let Some(delay) = self.delay {
            thread::sleep(delay);
        }
        self.stdout.queue(cursor::RestorePosition)?;
        self.stdout
            .queue(terminal::Clear(terminal::ClearType::FromCursorDown))?;
        Ok(())
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        self.stdout.execute(cursor::Show).unwrap();
    }
}
