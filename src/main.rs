extern crate pulldown_cmark;

// Any type that derives Fail can be cast into Error
use self::MarkdownError::*;
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, OPTION_ENABLE_FOOTNOTES,
                     OPTION_ENABLE_TABLES};
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, Read};
use std::borrow::Cow;

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
    let mut p = Parser::new_ext(&buffer, opts);

    // process events
    while let Some(event) = p.next() {
        match event {
            Event::Start(tag) => start_tag(tag),
            Event::End(tag) => end_tag(tag),
            Event::Text(text) => write_text(text),
            Event::Html(html) | Event::InlineHtml(html) => write_text(html), // don't handle html now
            Event::SoftBreak => soft_break(),
            Event::HardBreak => hard_break(),
            Event::FootnoteReference(name) => footnote(name),
        }
    }
    Ok(())
}
fn start_tag<'a>(tag: Tag<'a>) {}

fn end_tag<'a>(tag: Tag<'a>) {}

fn write_text<'a>(text: Cow<'a, str>) {}

fn soft_break() {}
fn hard_break() {}
fn footnote<'a>(name: Cow<'a, str>) {}
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
