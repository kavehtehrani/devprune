use std::sync::mpsc;
use std::time::Duration;

use crossterm::event::{self, Event};
use devprune_core::types::ScanEvent;

/// Unified event type fed to the main loop.
pub enum AppEvent {
    Input(Event),
    Scan(ScanEvent),
    Tick,
}

/// Poll for the next event, multiplexing keyboard input, scan channel messages,
/// and a 100 ms tick timer.
///
/// Returns `None` only when the scan receiver has hung up AND there is no
/// pending keyboard input. In practice the loop should keep running until the
/// user quits.
pub fn next_event(scan_rx: &mpsc::Receiver<ScanEvent>) -> Option<AppEvent> {
    // Check for keyboard input with a short timeout so we also service the
    // scan channel promptly.
    const POLL_TIMEOUT: Duration = Duration::from_millis(50);

    if event::poll(POLL_TIMEOUT).unwrap_or(false) {
        if let Ok(ev) = event::read() {
            return Some(AppEvent::Input(ev));
        }
    }

    // Drain one scan event if available.
    match scan_rx.try_recv() {
        Ok(ev) => return Some(AppEvent::Scan(ev)),
        Err(mpsc::TryRecvError::Empty) => {}
        Err(mpsc::TryRecvError::Disconnected) => {}
    }

    Some(AppEvent::Tick)
}
