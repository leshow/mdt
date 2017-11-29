extern crate pulldown_cmark;

// Any type that derives Fail can be cast into Error
use self::MarkdownError::*;
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, OPTION_ENABLE_FOOTNOTES,
                     OPTION_ENABLE_TABLES};
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, Read};

fn main() {
    if let Err(e) = run() {
        println!("{:?}", e);
    }
}

fn run() -> Result<(), MarkdownError> {
    let mut opts = Options::empty();
    opts.insert(OPTION_ENABLE_TABLES);
    opts.insert(OPTION_ENABLE_FOOTNOTES);

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    let p = Parser::new_ext(&buffer, opts);
    Ok(())
}

// Error
#[derive(Debug)]
pub(crate) enum MarkdownError {
    Io(io::Error),
}

impl From<io::Error> for MarkdownError {
    fn from(e: io::Error) -> MarkdownError {
        Io(e)
    }
}

impl fmt::Display for MarkdownError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Io(ref e) => write!(f, "IO Error: {}", e),
        }
    }
}

impl Error for MarkdownError {
    fn description(&self) -> &str {
        match *self {
            Io(ref e) => e.description(),
        }
    }
}
