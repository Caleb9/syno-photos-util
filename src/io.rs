//! Isolates IO and PasswordReader for testing

use anyhow::Result;
use std::io::{BufRead, IsTerminal, Stderr, StdinLock, Stdout, Write};
use yapp::{PasswordReader, Yapp};

pub trait Io {
    type StdIn: BufRead + IsTerminal;
    type StdOut: Write;
    type StdErr: Write;
    type PasswordReader: PasswordReader;

    fn stdin(&mut self) -> &mut Self::StdIn;
    fn stdout(&mut self) -> &mut Self::StdOut;
    fn stderr(&mut self) -> &mut Self::StdErr;
    fn password_reader(&mut self) -> &mut Self::PasswordReader;
}

pub(crate) fn read_input<I: Io>(prompt: &str, io: &mut I) -> Result<String> {
    let mut input = String::new();
    write!(io.stdout(), "{prompt}: ")?;
    io.stdout().flush()?;
    io.stdin().read_line(&mut input)?;
    input.truncate(input.trim_end().len());
    Ok(input)
}

pub struct IoImpl {
    stdin: StdinLock<'static>,
    stdout: Stdout,
    stderr: Stderr,
    password_reader: Yapp,
}

impl Io for IoImpl {
    type StdIn = StdinLock<'static>;
    type StdOut = Stdout;
    type StdErr = Stderr;
    type PasswordReader = Yapp;

    fn stdin(&mut self) -> &mut Self::StdIn {
        &mut self.stdin
    }

    fn stdout(&mut self) -> &mut Self::StdOut {
        &mut self.stdout
    }

    fn stderr(&mut self) -> &mut Self::StdErr {
        &mut self.stderr
    }

    fn password_reader(&mut self) -> &mut Self::PasswordReader {
        &mut self.password_reader
    }
}

impl IoImpl {
    pub fn new() -> Self {
        IoImpl {
            stdin: std::io::stdin().lock(),
            stdout: std::io::stdout(),
            stderr: std::io::stderr(),
            password_reader: Yapp::default().with_echo_symbol('*'),
        }
    }
}

impl Default for IoImpl {
    fn default() -> Self {
        Self::new()
    }
}
