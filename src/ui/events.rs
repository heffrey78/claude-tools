use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

/// Terminal events that can occur
#[derive(Clone, Debug)]
pub enum Event {
    /// Key press event
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Terminal resize event
    Resize(u16, u16),
    /// Tick event (for animations, periodic updates)
    Tick,
    /// File system change event
    FileChanged(PathBuf),
    /// Configuration file change event
    ConfigChanged,
}

/// Event handler for terminal events
pub struct EventHandler {
    /// Event sender (kept for future use)
    _sender: mpsc::Sender<Event>,
    /// Event receiver
    receiver: mpsc::Receiver<Event>,
    /// Event thread handle
    _handler: thread::JoinHandle<()>,
}

impl EventHandler {
    /// Creates a new event handler with default tick rate
    pub fn new(tick_rate: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        let event_sender = sender.clone();

        let handler = thread::spawn(move || {
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if event::poll(timeout).expect("Unable to poll for event") {
                    match event::read().expect("Unable to read event") {
                        CrosstermEvent::Key(key) => {
                            if let Err(_) = event_sender.send(Event::Key(key)) {
                                break;
                            }
                        }
                        CrosstermEvent::Mouse(mouse) => {
                            if let Err(_) = event_sender.send(Event::Mouse(mouse)) {
                                break;
                            }
                        }
                        CrosstermEvent::Resize(width, height) => {
                            if let Err(_) = event_sender.send(Event::Resize(width, height)) {
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if let Err(_) = event_sender.send(Event::Tick) {
                        break;
                    }
                    last_tick = Instant::now();
                }
            }
        });

        Self {
            _sender: sender,
            receiver,
            _handler: handler,
        }
    }

    /// Receive the next event
    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.receiver.recv()
    }
}
