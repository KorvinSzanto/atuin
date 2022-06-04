use std::{thread, time::Duration};

use crossbeam_channel::{bounded, TrySendError};
use termion::{event::Event as TermEvent, event::Key, input::TermRead};

pub enum Event<I> {
    Input(I),
    Tick,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: crossbeam_channel::Receiver<Event<TermEvent>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: Key,
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: Key::Char('q'),
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Events {
    pub fn new() -> Events {
        Events::with_config(Config::default())
    }

    pub fn with_config(config: Config) -> Events {
        // Keep channel small so scroll events don't stack for ages.
        let (tx, rx) = bounded(1);

        {
            let tx = tx.clone();
            thread::spawn(move || {
                let tty = termion::get_tty().expect("Could not find tty");
                for event in tty.events().flatten() {
                    if let Err(err) = tx.try_send(Event::Input(event)) {
                        if let TrySendError::Full(_) = err {
                            // Silently ignore send fails when buffer is full.
                            // This will most likely be scroll wheel spam and we can drop some events.
                        } else {
                            eprintln!("{}", err);
                            return;
                        }
                    }
                }
            })
        };

        thread::spawn(move || loop {
            if tx.send(Event::Tick).is_err() {
                break;
            }
            thread::sleep(config.tick_rate);
        });

        Events { rx }
    }

    pub fn next(&self) -> Result<Event<TermEvent>, crossbeam_channel::RecvError> {
        self.rx.recv()
    }
}
