//! In-engine developer console — command input + log output.

/// In-engine developer console.
///
/// Toggle with the `` ` `` key (DevConsole action). Commands are parsed as
/// text and dispatched to registered command handlers.
#[derive(Debug)]
pub struct DevConsole {
    /// Whether the console window is currently visible.
    pub visible: bool,
    /// Log lines shown in the console output area.
    pub log: Vec<String>,
    /// Current text in the input field.
    pub input: String,
    /// Command history (up/down arrow navigation).
    history: Vec<String>,
    history_idx: Option<usize>,
}

impl DevConsole {
    /// Create a new, hidden dev console.
    pub fn new() -> Self {
        Self {
            visible: false,
            log: Vec::new(),
            input: String::new(),
            history: Vec::new(),
            history_idx: None,
        }
    }

    /// Toggle visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Append a line to the console log.
    pub fn print(&mut self, line: impl Into<String>) {
        let line = line.into();
        log::debug!("[console] {}", line);
        self.log.push(line);
        // Keep last 500 lines
        if self.log.len() > 500 {
            self.log.drain(0..100);
        }
    }

    /// Submit the current input line as a command.
    pub fn submit(&mut self) -> Option<String> {
        let cmd = self.input.trim().to_string();
        if cmd.is_empty() { return None; }
        self.history.push(cmd.clone());
        self.history_idx = None;
        self.input.clear();
        Some(cmd)
    }
}

impl Default for DevConsole {
    fn default() -> Self { Self::new() }
}
