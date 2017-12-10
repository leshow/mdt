extern crate pulldown_cmark;

// Any type that derives Fail can be cast into Error
use self::MarkdownError::*;
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, OPTION_ENABLE_FOOTNOTES,
                     OPTION_ENABLE_TABLES};
use std::borrow::Cow;
use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, Read, Write};
use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::thread;

fn main() {
    if let Err(e) = run() {
        println!("{:?}", e);
    }
}

struct Ctx {
    nest_lvl: i32,
}

impl Ctx {
    pub fn increment(&mut self) {
        self.nest_lvl += 1;
    }
    pub fn decrement(&mut self) {
        self.nest_lvl -= 1;
    }
}

impl Default for Ctx {
    fn default() -> Self {
        Ctx { nest_lvl: 0 }
    }
}

fn run() -> Result<(), MarkdownError> {
    let done = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();

    let mut opts = Options::empty();
    opts.insert(OPTION_ENABLE_TABLES);
    opts.insert(OPTION_ENABLE_FOOTNOTES);

    // get input
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;

    // make parser
    let mut p = Parser::new_ext(&buffer, opts).map(|event| match event {
        Event::InlineHtml(html) | Event::Html(html) => Event::Text(html),
        _ => event,
    });

    let mut ctx = Ctx::default();
    // let final = done.clone();
    let done = done.clone();
    thread::spawn(move || {
        while done.load(Ordering::Relaxed) {
            let output = match rx.recv() {
                Ok(text) => text,
                Err(_) => panic!("Received error"),
            };
            io::stdout().write(output);
        }
    });

    // process events
    while let Some(event) = p.next() {
        match event {
            Event::Start(tag) => {
                ctx.increment();
                start_tag(tag, tx.clone());
            }
            Event::End(tag) => {
                ctx.decrement();
                end_tag(tag, tx.clone());
            }
            Event::Text(text) => write_text(text, tx.clone()),
            // Event::Html(html) | Event::InlineHtml(html) => write_text(html, tx),
            Event::SoftBreak => soft_break(),
            Event::HardBreak => hard_break(),
            Event::FootnoteReference(name) => footnote(name),
            _ => panic!("html and inline html converted to text, this is unreachable"),
        }
    }
    // final.store(false, Ordering::Relaxed);

    Ok(())
}

fn start_tag<'a, T>(tag: Tag<'a>, sender: Sender<T>) {}

fn end_tag<'a, T>(tag: Tag<'a>, sender: Sender<T>) {}

fn write_text<'a, T>(text: Cow<'a, str>, sender: Sender<T>) {}

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
    fn cause(&self) -> Option<&Error> {
        None
    }
}
