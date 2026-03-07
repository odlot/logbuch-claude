use std::io::IsTerminal;

/// Returns true when stdout is connected to a terminal (colour is appropriate).
pub fn use_color() -> bool {
    std::io::stdout().is_terminal()
}

pub struct Out {
    pub color: bool,
}

impl Default for Out {
    fn default() -> Self {
        Self::new()
    }
}

impl Out {
    pub fn new() -> Self {
        Self { color: use_color() }
    }

    pub fn bold(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[1m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    pub fn dim(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[2m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    pub fn green(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[32m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    pub fn cyan(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[36m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    pub fn yellow(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[33m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }

    pub fn strikethrough(&self, s: &str) -> String {
        if self.color {
            format!("\x1b[9m{}\x1b[0m", s)
        } else {
            s.to_string()
        }
    }
}
