// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Terminal User Interface for watch mode.
//!
//! This module provides a rich TUI for monitoring lint results in real-time.
//! It uses ratatui for rendering and crossterm for terminal manipulation.
//!
//! ## Features
//!
//! - **File tree**: Shows watched files with status indicators
//! - **Issue list**: Scrollable list of lint issues
//! - **Status bar**: Current status, counts, and keyboard shortcuts
//! - **Help overlay**: Quick reference for keyboard shortcuts
//!
//! ## Keyboard Shortcuts
//!
//! - `q` / `Ctrl+C` - Quit
//! - `↑/↓` or `j/k` - Navigate issues
//! - `Enter` - Open file in editor
//! - `Tab` - Switch focus between panels
//! - `r` - Force re-run all
//! - `c` - Clear results
//! - `?` - Toggle help overlay

mod app;
pub mod event;
mod ui;
pub mod widgets;

pub use app::{App, AppState};
pub use event::{handle_key_event, Event, EventHandler};
pub use ui::draw;

use std::io::{self, Stdout};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

/// Type alias for the terminal backend
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> io::Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore the terminal to normal mode
pub fn restore_terminal(terminal: &mut Tui) -> io::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
