extern crate getopts;
extern crate image;
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
use std::io::{self, Read, Stdout};
use std::iter::Map;

mod escape;
pub mod table;
mod img;
pub mod terminal;
use terminal::{MDParser, TermAscii, TermUnicode};

fn main() {
    if let Err(e) = run() {
        println!("{:?}", e);
    }
}

fn run() -> MDResult<()> {
    // parse args
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = GetOpts::new();
    opts.optflag(
        "t",
        "truecolor",
        "print with truecolor (syntax highlighting)",
    );
    opts.optflag("a", "ascii", "print table using ascii characters");
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
    let p = Parser::new_ext(&buffer, opts);
    let term_size = termion::terminal_size()?;

    // dynamic dispatch
    // let mut terminal: Box<MDParser<Parser, Stdout>> = if
    // matches.opt_present("a") { Box::new(TermAscii::new(term_size,
    // truecolor)) } else {
    //     Box::new(TermUnicode::new(term_size, truecolor))
    // };
    if matches.opt_present("a") {
        let mut terminal = TermAscii::new(term_size, truecolor);
        terminal.parse(p, &mut io::stdout())?;
    } else {
        let mut terminal = TermUnicode::new(term_size, truecolor);
        terminal.parse(p, &mut io::stdout())?;
    };

    Ok(())
}

fn print_usage(program: &str, opts: GetOpts) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

// Error
#[derive(Debug)]
pub enum MarkdownError {
    Io(io::Error),
    Args(getopts::Fail),
    Img(image::ImageError),
}

pub type MDResult<T> = Result<T, MarkdownError>;

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
impl From<image::ImageError> for MarkdownError {
    fn from(e: image::ImageError) -> MarkdownError {
        Img(e)
    }
}

impl fmt::Display for MarkdownError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Io(ref e) => write!(f, "IO Error: {}", e),
            Args(ref e) => write!(f, "Arg Parse Error: {}", e),
            Img(ref e) => write!(f, "Image Error: {}", e),
        }
    }
}

impl Error for MarkdownError {
    fn description(&self) -> &str {
        match *self {
            Io(ref e) => e.description(),
            Args(ref e) => e.description(),
            Img(ref e) => e.description(),
        }
    }
    fn cause(&self) -> Option<&Error> {
        None
    }
}
