extern crate html2text;
extern crate pulldown_cmark;
extern crate termion;


// Any type that derives Fail can be cast into Error
use self::MarkdownError::*;
use html2text::render::text_renderer::{RichAnnotation, TaggedLine};
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
    buf: String,
}


impl Default for Ctx {
    fn default() -> Self {
        Ctx {
            nest_lvl: 0,
            buf: String::new(),
        }
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
    let mut p = Parser::new_ext(&buffer, opts).map(|event| match event {
        Event::InlineHtml(html) | Event::Html(html) => Event::Text(html),
        _ => event,
    });

    let mut ctx = Ctx::default();

    // process events
    while let Some(event) = p.next() {
        match event {
            Event::Start(tag) => {
                ctx.increment();
                ctx.start_tag(tag);
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

    Ok(())
}

impl Ctx {
    pub fn increment(&mut self) {
        self.nest_lvl += 1;
    }
    pub fn decrement(&mut self) {
        self.nest_lvl -= 1;
    }
    fn fresh_line(&mut self) {
        self.buf.push_str("\n");
    }
    pub fn start_tag<'a>(&mut self, tag: Tag<'a>) {
        match tag {
            Tag::Paragraph => {
                self.fresh_line();
            }
            Tag::Rule => {
                self.buf.push_str(&format!("{}", termion::style::Underline));
            }
            Tag::Header(level) => {
                self.fresh_line();
                self.buf.push_str("<h");
                self.buf.push((b'0' + level as u8) as char);
                self.buf.push('>');
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments;
                self.buf.push_str("<table>");
            }
            Tag::TableHead => {
                self.table_state = TableState::Head;
                self.buf.push_str("<thead><tr>");
            }
            Tag::TableRow => {
                self.table_cell_index = 0;
                self.buf.push_str("<tr>");
            }
            Tag::TableCell => {
                match self.table_state {
                    TableState::Head => self.buf.push_str("<th"),
                    TableState::Body => self.buf.push_str("<td"),
                }
                match self.table_alignments.get(self.table_cell_index) {
                    Some(&Alignment::Left) => self.buf.push_str(" align=\"left\""),
                    Some(&Alignment::Center) => self.buf.push_str(" align=\"center\""),
                    Some(&Alignment::Right) => self.buf.push_str(" align=\"right\""),
                    _ => (),
                }
                self.buf.push_str(">");
            }
            Tag::BlockQuote => {
                self.fresh_line();
                self.buf.push_str("<blockquote>\n");
            }
            Tag::CodeBlock(info) => {
                self.fresh_line();
                let lang = info.split(' ').next().unwrap();
                if lang.is_empty() {
                    self.buf.push_str(&format)
                    self.buf.push_str("<pre><code>");
                } else {
                    self.buf.push_str("<pre><code class=\"language-");
                    escape_html(self.buf, lang, false);
                    self.buf.push_str("\">");
                }
            }
            Tag::List(Some(1)) => {
                self.fresh_line();
                self.buf.push_str("<ol>\n");
            }
            Tag::List(Some(start)) => {
                self.fresh_line();
                let _ = write!(self.buf, "<ol start=\"{}\">\n", start);
            }
            Tag::List(None) => {
                self.fresh_line();
                self.buf.push_str("<ul>\n");
            }
            Tag::Item => {
                self.fresh_line();
                self.buf.push_str("<li>");
            }
            Tag::Emphasis => self.buf.push_str("<em>"),
            Tag::Strong => self.buf.push_str("<strong>"),
            Tag::Code => self.buf.push_str("<code>"),
            Tag::Link(dest, title) => {
                self.buf.push_str("<a href=\"");
                escape_href(self.buf, &dest);
                if !title.is_empty() {
                    self.buf.push_str("\" title=\"");
                    escape_html(self.buf, &title, false);
                }
                self.buf.push_str("\">");
            }
            Tag::Image(dest, title) => {
                self.buf.push_str("<img src=\"");
                escape_href(self.buf, &dest);
                self.buf.push_str("\" alt=\"");
                self.raw_text(numbers);
                if !title.is_empty() {
                    self.buf.push_str("\" title=\"");
                    escape_html(self.buf, &title, false);
                }
                self.buf.push_str("\" />")
            }
            Tag::FootnoteDefinition(name) => {
                self.fresh_line();
                let len = numbers.len() + 1;
                self.buf
                    .push_str("<div class=\"footnote-definition\" id=\"");
                escape_html(self.buf, &*name, false);
                self.buf
                    .push_str("\"><sup class=\"footnote-definition-label\">");
                let number = numbers.entry(name).or_insert(len);
                self.buf.push_str(&*format!("{}", number));
                self.buf.push_str("</sup>");
            }
        }
    }
}
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
