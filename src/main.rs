#![feature(nll, universal_impl_trait, conservative_impl_trait, associated_consts)]

extern crate getopts;
#[macro_use]
extern crate lazy_static;
extern crate pulldown_cmark;
extern crate syntect;
extern crate termion;

// Any type that derives Fail can be cast into Error
use self::MarkdownError::*;
use getopts::Options as GetOpts;
use pulldown_cmark::{Event, Options, Parser, OPTION_ENABLE_FOOTNOTES, OPTION_ENABLE_TABLES};
use std::env;
use std::error::Error;
use std::fmt;
use std::io::{self, Read, Write};

mod escape;
mod table;
mod terminal;
use terminal::{MDParser, Terminal};

fn main() {
    if let Err(e) = run() {
        println!("{:?}", e);
    }
}

fn run() -> Result<(), MarkdownError> {
    // parse args
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    // let mut term = io::stdout().into_raw_mode().unwrap();
    // println!("{:?}", term.available_colors().unwrap());

    let mut opts = GetOpts::new();
    opts.optflag(
        "t",
        "truecolor",
        "print with truecolor (syntax highlighting)",
    );
    opts.optflag("h", "help", "print this help menu");
    let matches = opts.parse(&args[1..])?;
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }
    let truecolor = matches.opt_present("t");

    // get input
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    // parser options
    let mut opts = Options::empty();
    opts.insert(OPTION_ENABLE_TABLES);
    opts.insert(OPTION_ENABLE_FOOTNOTES);

    // make parser
    let p = Parser::new_ext(&buffer, opts).map(|event| match event {
        Event::InlineHtml(html) | Event::Html(html) => Event::Text(html),
        _ => event,
    });
    let term_size = termion::terminal_size()?;

    let mut terminal = Terminal::new(term_size, truecolor);
    let out = terminal.parse(p);
    io::stdout().write_all(out.as_bytes())?;
    io::stdout().flush()?;

    Ok(())
}

fn print_usage(program: &str, opts: GetOpts) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

// Error
#[derive(Debug)]
pub(crate) enum MarkdownError {
    Io(io::Error),
    Args(getopts::Fail),
}

impl From<io::Error> for MarkdownError {
    fn from(e: io::Error) -> MarkdownError {
        Io(e)
    }
}
impl From<getopts::Fail> for MarkdownError {
    fn from(e: getopts::Fail) -> MarkdownError {
        Args(e)
    }
}

impl fmt::Display for MarkdownError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Io(ref e) => write!(f, "IO Error: {}", e),
            Args(ref e) => write!(f, "Arg Parse Error: {}", e),
        }
    }
}

impl Error for MarkdownError {
    fn description(&self) -> &str {
        match *self {
            Io(ref e) => e.description(),
            Args(ref e) => e.description(),
        }
    }
    fn cause(&self) -> Option<&Error> {
        None
    }
}
