use log::{debug, trace};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

// List of virtual cursors guaranteeing at last one cursor
#[derive(Debug)]
pub struct Cursor(Vec<usize>);

impl Cursor {
    pub fn new() -> Self {
        Self(vec![0])
    }

    // Add a new active virtual layer
    pub fn add(&mut self) {
        let current = self.0.last().unwrap();
        self.0.push(*current);
    }

    // Delete the active layer and set the layer below
    // to it's position
    pub fn merge(&mut self) {
        if self.0.len() > 1 {
            let current = self.0.pop().unwrap();
            *self.0.last_mut().unwrap() = current;
        }
    }

    // Delete the active layer
    // requires the source for the scanner so it can get the positon as lines
    pub fn collapse(&mut self, s: &Vec<char>) -> ErrorPosition {
        debug!("Collapsing virtual cursor layer (before: {:?})", &self);
        if self.0.len() > 1 {
            let active = self.0.pop().unwrap();
            let base = *self.0.last().unwrap();
            debug!("(after: {:?})", &self);
            ErrorPosition {
                active,
                base,
                lines: s.as_lines(active),
            }
        } else {
            let active = *self.0.last().unwrap();
            debug!("(after: {:?})", &self);
            ErrorPosition {
                active,
                base: active,
                lines: s.as_lines(active),
            }
        }
    }

    // Get the position of the active layer
    pub fn at(&self) -> usize {
        trace!("Used {:?}", self);
        *self.0.last().unwrap()
    }

    // Increase the position of the active layer
    pub fn inc(&mut self) {
        let current = self.0.last_mut().unwrap();
        *current += 1;
    }
}

// The scanner uses layers of virtual cursors which
// advance in the input character by character but
// keep their respective starting positions so they
// can go back there to restart on failures. (This might change at some point)
// The current virtual cursor layer is collaped if
// a lexical error is encountered! This means the parser
// only needs to perform manual aborts if a syntax error
// is raised in the parser itself.
pub struct Scanner {
    cursor: Cursor,
    chars: Vec<char>,
}

impl Scanner {
    pub fn new(s: &str) -> Self {
        debug!("New scanner from: {}", &s);
        Self {
            cursor: Cursor::new(),
            chars: s.chars().collect(),
        }
    }

    pub fn at_end(&self) -> bool {
        if self.cursor.at() == self.chars.len() {
            debug!("Scanner has reached the end");
            true
        } else {
            debug!("Scanner has NOT reached the end");
            false
        }
    }

    pub fn begin(&mut self) {
        debug!("Adding virtual cursor layer");
        self.cursor.add();
    }

    pub fn abort(&mut self) {
        debug!("Removing virtual cursor layer");
        self.cursor.collapse(&self.chars);
    }

    pub fn commit(&mut self) {
        debug!("Comitting virtual cursor layer");
        self.cursor.merge();
    }

    pub fn current_char(&self) -> Option<char> {
        self.chars.get(self.cursor.at()).copied()
    }

    // Scan a single character
    pub fn take(&mut self, character: char) -> Result<(), ScanError> {
        if let Some(current) = self.current_char() {
            if character == current {
                self.cursor.inc();
                debug!("Took character '{}'  succesfully", character);
                Ok(())
            } else {
                let symbol = UnexpectedSymbol {
                    found: current,
                    expected: None,
                    position: self.cursor.collapse(&self.chars),
                };
                debug!("Failed to take character: {}", &symbol);
                Err(ScanError::UnexpectedSymbol(symbol))
            }
        } else {
            debug!("Failed to take character '{}': Hit end of input", character);
            Err(ScanError::UnexpectedEndOfInput(
                self.cursor.collapse(&self.chars),
            ))
        }
    }

    // Scan a constant sequence of characters (e.g. a keyword)
    // The successful result is not returned because it would match the input
    pub fn take_str(&mut self, s: &str) -> Result<(), ScanError> {
        for character in s.chars() {
            self.take(character)?;
        }

        Ok(())
    }

    // Scan character by character
    pub fn scan(&mut self, callback: impl Fn(char) -> Option<Action>) -> Result<String, ScanError> {
        self.scan_seq(|seq| {
            // Unwrap because `scan_seq` always pushes a char to the sequence
            // before evoking the callback
            callback(seq.chars().last().unwrap())
        })
    }

    // Scan dynamic sequences of characters
    pub fn scan_seq(
        &mut self,
        callback: impl Fn(&str) -> Option<Action>,
    ) -> Result<String, ScanError> {
        let mut sequence = String::new();
        let mut require = None;
        let mut request = false;

        loop {
            match self.current_char() {
                Some(target) => {
                    sequence.push(target);
                    match callback(&sequence) {
                        Some(action) => {
                            match action {
                                // Continue but return ok if next iteration fails
                                Action::Request => {
                                    self.cursor.inc();
                                    require = None;
                                    request = true;
                                    debug!("Requesting result after character '{}'", target);
                                }
                                // Return now
                                Action::Return => {
                                    self.cursor.inc();
                                    match require {
                                        Some(require) => {
                                            if target == require {
                                                debug!(
                                                    "Returning result after character '{}'",
                                                    target
                                                );
                                            } else {
                                                debug!(
                                                    "Failed to return result after character '{}' \
                                                    because previous requirement was not matched. \
                                                    Now returning sequence up to the require call",
                                                    target
                                                );
                                                sequence.pop(); // Remove the new character which did not match
                                            }
                                        }
                                        None => {
                                            debug!("Returning result after character '{}'", target)
                                        }
                                    }
                                    break Ok(sequence);
                                }
                                // Continue and return the current sequence if the next iteration
                                // fails or does not match the given symbol
                                Action::Require(symbol) => {
                                    self.cursor.inc();
                                    require = Some(symbol);
                                    debug!(
                                        "Requiring next character as '{}' after '{}'",
                                        target, symbol
                                    );
                                }
                            }
                        }
                        None => {
                            sequence.pop(); // The last character was invalid!

                            break match request {
                                true => {
                                    debug!("Returning result after failing to get new character on request");
                                    Ok(sequence)
                                }
                                false => {
                                    let symbol = UnexpectedSymbol {
                                        found: target,
                                        expected: None,
                                        position: self.cursor.collapse(&self.chars),
                                    };
                                    debug!("Failed to get new character while neither requiring nor requesting: {}", &symbol);
                                    Err(ScanError::UnexpectedSymbol(symbol))
                                }
                            };
                        }
                    }
                }
                None => {
                    break match request {
                        true => {
                            debug!("Returning result after hitting end of input on request");
                            Ok(sequence)
                        }
                        false => {
                            debug!("Hit end of input while neither requiring nor requesting");
                            Err(ScanError::UnexpectedEndOfInput(
                                self.cursor.collapse(&self.chars),
                            ))
                        }
                    }
                }
            }
        }
    }
}

trait PosAsLines {
    fn as_lines(&self, pos: usize) -> (usize, usize);
}

impl PosAsLines for Vec<char> {
    // Convert a position in the string into the
    // corresponding position as lines and columns.
    // Lines and columns both start with 1 as the lowest value.
    fn as_lines(&self, pos: usize) -> (usize, usize) {
        let mut line = 1;
        let mut column = 1;
        for character in self.iter().take(pos) {
            if *character == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        (line, column)
    }
}

#[derive(Debug)]
pub enum Action {
    Request,       // Try to get another character but still return a success if this failed
    Return,        // Return a success
    Require(char), // Like request but the next character has the be `char`
}

#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ScanError {
    #[error("{0}")]
    UnexpectedSymbol(UnexpectedSymbol),
    #[error("Unexpected end of input reached at position {0}")]
    UnexpectedEndOfInput(ErrorPosition),
}

impl ScanError {
    // Difference between active and base cursor position
    // when the error was raised
    pub const fn failed_after(&self) -> usize {
        let err_pos = match self {
            Self::UnexpectedSymbol(symbol) => symbol.position,
            Self::UnexpectedEndOfInput(position) => *position,
        };

        err_pos.active - err_pos.base
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct UnexpectedSymbol {
    found: char,
    expected: Option<char>,
    position: ErrorPosition,
}

impl std::fmt::Display for UnexpectedSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(expected) = self.expected {
            write!(
                f,
                "'{}' at {} (expected '{}')",
                self.found, self.position, expected
            )
        } else {
            write!(f, "'{}' at {}", self.found, self.position)
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ErrorPosition {
    active: usize,
    base: usize,
    lines: (usize, usize),
}

impl std::fmt::Display for ErrorPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.lines {
            (1, column) => write!(f, "column {column}"),
            (line, column) => write!(f, "line {line}, column {column}"),
        }
    }
}
