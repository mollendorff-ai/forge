//! Where a command reads and writes: its output, its messages, the answer to
//! a question it asks, and the width of the terminal it prints to.
//!
//! `forge` hands each command the process's own streams (`main.rs`); a test
//! hands it buffers, so a test of the command line runs in-process and reads
//! exactly what a user would see.
//!
//! A command writes with `outln!` and `errln!`, as it would with `println!`
//! and `eprintln!`. A write that fails is kept, the later writes to that
//! stream are skipped, and [`Console::finish`] returns the failure, which
//! `run_from` reports once the command is done: a command's code has no
//! error path for each line it prints.

use std::fmt;
use std::io::{self, BufRead, Write};

/// The streams and terminal a command writes to.
pub struct Console<'a> {
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    input: &'a mut dyn BufRead,
    out_failed: Option<io::Error>,
    err_failed: Option<io::Error>,
    /// The terminal's width, or `None` when the output is not a terminal (a
    /// file or a pipe), which is never wrapped.
    pub width: Option<usize>,
    /// Whether what clap writes to `out` (the help, the version) is in colour.
    pub color_out: bool,
    /// Whether what clap writes to `err` (a usage error) is in colour.
    pub color_err: bool,
}

impl<'a> Console<'a> {
    /// A console on these streams: what the command prints, its warnings and
    /// errors, and what the user types when it asks. Not a terminal, and
    /// without colour.
    pub fn new(out: &'a mut dyn Write, err: &'a mut dyn Write, input: &'a mut dyn BufRead) -> Self {
        Self {
            out,
            err,
            input,
            out_failed: None,
            err_failed: None,
            width: None,
            color_out: false,
            color_err: false,
        }
    }

    /// Write to the output: what `outln!` and `out!` call.
    pub fn write_out(&mut self, text: fmt::Arguments<'_>) {
        write_kept(&mut *self.out, &mut self.out_failed, text);
    }

    /// Write to the error stream: what `errln!` calls.
    pub fn write_err(&mut self, text: fmt::Arguments<'_>) {
        write_kept(&mut *self.err, &mut self.err_failed, text);
    }

    /// Read a line the user types, after showing what was written so far (a
    /// question asked without a newline).
    ///
    /// # Errors
    ///
    /// Returns an error if the output cannot be flushed or the input read.
    pub fn read_line(&mut self, line: &mut String) -> io::Result<usize> {
        self.out.flush()?;
        self.input.read_line(line)
    }

    /// Flush both streams and return the first write that failed, on the
    /// output first, then on the error stream.
    ///
    /// # Errors
    ///
    /// Returns the first write or flush that failed.
    pub fn finish(&mut self) -> io::Result<()> {
        if let Some(e) = self.out_failed.take().or_else(|| self.err_failed.take()) {
            return Err(e);
        }
        self.out.flush()?;
        self.err.flush()
    }
}

/// Write `text` to `stream` unless an earlier write failed; keep the first
/// failure in `failed`.
fn write_kept(stream: &mut dyn Write, failed: &mut Option<io::Error>, text: fmt::Arguments<'_>) {
    if failed.is_none() {
        if let Err(e) = stream.write_fmt(text) {
            *failed = Some(e);
        }
    }
}

/// Like `println!`, on a console's output: `outln!(c)`, `outln!(c, "{x}")`.
macro_rules! outln {
    ($c:expr) => {
        $c.write_out(format_args!("\n"))
    };
    ($c:expr, $($arg:tt)*) => {
        $c.write_out(format_args!("{}\n", format_args!($($arg)*)))
    };
}

/// Like `print!`, on a console's output.
macro_rules! out {
    ($c:expr, $($arg:tt)*) => {
        $c.write_out(format_args!($($arg)*))
    };
}

/// Like `eprintln!`, on a console's error stream.
macro_rules! errln {
    ($c:expr) => {
        $c.write_err(format_args!("\n"))
    };
    ($c:expr, $($arg:tt)*) => {
        $c.write_err(format_args!("{}\n", format_args!($($arg)*)))
    };
}

pub(crate) use {errln, out, outln};

/// What a command wrote, for a test to read.
#[cfg(test)]
#[derive(Debug)]
pub struct Captured {
    /// Everything written to the output.
    pub out: String,
    /// Everything written to the error stream.
    pub err: String,
}

#[cfg(test)]
impl Captured {
    /// The output and the error stream together, for a failure message.
    #[must_use]
    pub fn both(&self) -> String {
        format!("{}{}", self.out, self.err)
    }
}

/// Run `f` on a console that writes to buffers and reads nothing.
///
/// Returns its result with what it wrote, colour codes taken out (whether
/// `colored` colours depends on the test's own terminal).
///
/// # Panics
///
/// Panics if what `f` wrote is not UTF-8.
#[cfg(test)]
pub fn capture<T>(f: impl FnOnce(&mut Console<'_>) -> T) -> (T, Captured) {
    let (mut out, mut err, mut input) = (Vec::new(), Vec::new(), io::empty());
    let result = f(&mut Console::new(&mut out, &mut err, &mut input));
    let text = |bytes: Vec<u8>| strip_colour(&String::from_utf8(bytes).expect("UTF-8 output"));
    (
        result,
        Captured {
            out: text(out),
            err: text(err),
        },
    )
}

/// `text` without its ANSI colour codes (`ESC [ ... m`).
#[cfg(test)]
fn strip_colour(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for code in chars.by_ref() {
                if code == 'm' {
                    break;
                }
            }
        } else {
            plain.push(c);
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stream whose writes, or only whose flushes, fail.
    struct Failing {
        writes: bool,
    }

    impl Write for Failing {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.writes {
                Err(io::Error::other("write refused"))
            } else {
                Ok(buf.len())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush refused"))
        }
    }

    #[test]
    fn capture_returns_the_result_and_both_streams() {
        let (result, captured) = capture(|c| {
            outln!(c, "on {}", "out");
            out!(c, "no newline ");
            outln!(c);
            errln!(c, "on err");
            errln!(c);
            let mut line = String::new();
            let read = c.read_line(&mut line).unwrap();
            (read, c.width, c.color_out, c.color_err, c.finish().is_ok())
        });
        assert_eq!(result, (0, None, false, false, true));
        assert_eq!(captured.out, "on out\nno newline \n");
        assert_eq!(captured.err, "on err\n\n");
        assert_eq!(captured.both(), "on out\nno newline \non err\n\n");
    }

    #[test]
    fn the_first_failed_write_is_kept_and_later_writes_are_skipped() {
        let (mut out, mut err, mut input) = (Failing { writes: true }, Vec::new(), io::empty());
        let mut c = Console::new(&mut out, &mut err, &mut input);
        outln!(c, "lost");
        outln!(c, "skipped");
        errln!(c, "still written");
        assert_eq!(c.finish().unwrap_err().to_string(), "write refused");
        // Reported once; the output's flush then fails as well
        assert_eq!(c.finish().unwrap_err().to_string(), "flush refused");
        drop(c);
        assert_eq!(err, b"still written\n");
    }

    #[test]
    fn a_failed_write_to_the_error_stream_is_kept_too() {
        let (mut out, mut err, mut input) = (Vec::new(), Failing { writes: true }, io::empty());
        let mut c = Console::new(&mut out, &mut err, &mut input);
        errln!(c, "lost");
        assert_eq!(c.finish().unwrap_err().to_string(), "write refused");
    }

    #[test]
    fn a_failed_flush_is_returned_by_finish_and_read_line() {
        let (mut out, mut err, mut input) = (Failing { writes: false }, Vec::new(), io::empty());
        let mut c = Console::new(&mut out, &mut err, &mut input);
        outln!(c, "written, never flushed");
        assert_eq!(c.finish().unwrap_err().to_string(), "flush refused");
        let mut line = String::new();
        assert_eq!(
            c.read_line(&mut line).unwrap_err().to_string(),
            "flush refused"
        );
        let (mut out, mut err, mut input) = (Vec::new(), Failing { writes: false }, io::empty());
        let mut c = Console::new(&mut out, &mut err, &mut input);
        assert_eq!(c.finish().unwrap_err().to_string(), "flush refused");
    }

    #[test]
    fn colour_codes_are_taken_out() {
        assert_eq!(strip_colour("\x1b[1;32mgreen\x1b[0m plain"), "green plain");
        assert_eq!(strip_colour("no codes"), "no codes");
    }
}
