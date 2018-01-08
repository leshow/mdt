extern crate pulldown_cmark;
extern crate termion;

// Any type that derives Fail can be cast into Error
use self::MarkdownError::*;
use pulldown_cmark::{Event, Options, Parser, OPTION_ENABLE_FOOTNOTES, OPTION_ENABLE_TABLES};
use std::error::Error;
use std::fmt;
use std::io::{self, Read};

mod escape;
mod terminal;
use terminal::{MDParser, Terminal};

fn main() {
    if let Err(e) = run() {
        println!("{:?}", e);
    }
}

fn run() -> Result<(), MarkdownError> {
    let mut opts = Options::empty();
    opts.insert(OPTION_ENABLE_TABLES);
    opts.insert(OPTION_ENABLE_FOOTNOTES);

    // get input
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    // make parser
    let p = Parser::new_ext(&buffer, opts).map(|event| match event {
        Event::InlineHtml(html) | Event::Html(html) => Event::Text(html),
        _ => event,
    });
    let term_size = termion::terminal_size()?;

    let mut terminal = Terminal::new(term_size);
    let out = terminal.parse(p);
    print!("{}", out);
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
    fn cause(&self) -> Option<&Error> {
        None
    }
}
