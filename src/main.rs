extern crate pulldown_cmark;
extern crate termion;

// Any type that derives Fail can be cast into Error
use self::MarkdownError::*;
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, OPTION_ENABLE_FOOTNOTES,
                     OPTION_ENABLE_TABLES};
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fmt::Write;
use std::io::{self, Read};
use termion::color;
use termion::style;

mod escape;
use escape::{escape_href, escape_html};

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
    let mut ctx = Ctx::new(p, term_size);
    ctx.run();
    print!("{}", ctx.buf);
    Ok(())
}

enum TableState {
    Head,
    Body,
}

struct Ctx<I> {
    iter: I,
    header_lvl: i32,
    term_size: (u16, u16),
    buf: String,
    table_state: TableState,
    table_alignments: Vec<Alignment>,
    table_cell_index: usize,
}

impl<'a, 'b, I> Ctx<I>
where
    I: Iterator<Item = Event<'a>>,
{
    pub fn new(iter: I, term_size: (u16, u16)) -> Ctx<I> {
        Ctx {
            iter,
            buf: String::new(),
            table_state: TableState::Head,
            header_lvl: 0,
            term_size,
            table_alignments: Vec::new(),
            table_cell_index: 0,
        }
    }
    pub fn run(&mut self) {
        let mut numbers = HashMap::new();
        // process events
        while let Some(event) = self.iter.next() {
            match event {
                Event::Start(tag) => {
                    self.increment();
                    self.start_tag(tag, &mut numbers);
                }
                Event::End(tag) => {
                    self.decrement();
                    self.end_tag(tag);
                }
                Event::Text(text) => self.write_text(text),
                // Event::Html(html) | Event::InlineHtml(html) => write_text(html, tx),
                Event::SoftBreak => self.soft_break(),
                Event::HardBreak => self.hard_break(),
                Event::FootnoteReference(name) => self.footnote(name),
                _ => panic!("html and inline html converted to text, this is unreachable"),
            }
        }
    }
    fn increment(&mut self) {
        self.header_lvl += 1;
    }
    fn decrement(&mut self) {
        self.header_lvl -= 1;
    }
    fn fresh_line(&mut self) {
        self.buf.push('\n');
    }
    fn width(&self) -> usize {
        self.term_size.1 as usize
    }
    fn start_tag(&mut self, tag: Tag<'a>, numbers: &mut HashMap<Cow<'a, str>, usize>) {
        match tag {
            Tag::Paragraph => {
                self.fresh_line();
            }
            Tag::Rule => {
                let w = self.width();
                self.buf.push_str(&format!("{:-width$}", width = w));
            }
            Tag::Header(level) => {
                self.fresh_line();
                self.buf.push_str("<h");
                self.buf.push((b'0' + level as u8) as char);
                self.buf.push('>');
            }
            Tag::Table(alignments) => {
                //self.table_alignments = alignments;
                self.buf.push_str("<table>");
            }
            Tag::TableHead => {
                self.table_state = TableState::Head;
                self.buf.push_str("<thead><tr>");
            }
            Tag::TableRow => {
                //self.table_cell_index = 0;
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
                    self.buf.push_str("<pre><code>");
                } else {
                    self.buf.push_str("<pre><code class=\"language-");
                    escape_html(&mut self.buf, lang, false);
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
                escape_href(&mut self.buf, &dest);
                if !title.is_empty() {
                    self.buf.push_str("\" title=\"");
                    escape_html(&mut self.buf, &title, false);
                }
                self.buf.push_str("\">");
            }
            Tag::Image(dest, title) => {
                self.buf.push_str("<img src=\"");
                escape_href(&mut self.buf, &dest);
                self.buf.push_str("\" alt=\"");
                //self.raw_text(numbers);
                if !title.is_empty() {
                    self.buf.push_str("\" title=\"");
                    escape_html(&mut self.buf, &title, false);
                }
                self.buf.push_str("\" />")
            }
            Tag::FootnoteDefinition(name) => {
                self.fresh_line();
                let len = numbers.len() + 1;
                self.buf
                    .push_str("<div class=\"footnote-definition\" id=\"");
                escape_html(&mut self.buf, &*name, false);
                self.buf
                    .push_str("\"><sup class=\"footnote-definition-label\">");
                let number = numbers.entry(name).or_insert(len);
                self.buf.push_str(&*format!("{}", number));
                self.buf.push_str("</sup>");
            }
        }
    }
    fn end_tag(&mut self, tag: Tag<'a>) {
        match tag {
            Tag::Paragraph => self.buf.push_str("</p>\n"),
            Tag::Rule => (),
            Tag::Header(level) => {
                self.buf.push_str("</h");
                self.buf.push((b'0' + level as u8) as char);
                self.buf.push_str(">\n");
            }
            Tag::Table(_) => {
                self.buf.push_str("</tbody></table>\n");
            }
            Tag::TableHead => {
                self.buf.push_str("</tr></thead><tbody>\n");
                self.table_state = TableState::Body;
            }
            Tag::TableRow => {
                self.buf.push_str("</tr>\n");
            }
            Tag::TableCell => {
                match self.table_state {
                    TableState::Head => self.buf.push_str("</th>"),
                    TableState::Body => self.buf.push_str("</td>"),
                }
                self.table_cell_index += 1;
            }
            Tag::BlockQuote => self.buf.push_str("</blockquote>\n"),
            Tag::CodeBlock(_) => self.buf.push_str("</code></pre>\n"),
            Tag::List(Some(_)) => self.buf.push_str("</ol>\n"),
            Tag::List(None) => self.buf.push_str("</ul>\n"),
            Tag::Item => self.buf.push_str("</li>\n"),
            Tag::Emphasis => self.buf.push_str("</em>"),
            Tag::Strong => self.buf.push_str("</strong>"),
            Tag::Code => self.buf.push_str("</code>"),
            Tag::Link(_, _) => self.buf.push_str("</a>"),
            Tag::Image(_, _) => (), // shouldn't happen, handled in start
            Tag::FootnoteDefinition(_) => self.buf.push_str("</div>\n"),
        }
    }
    fn write_text(&mut self, text: Cow<'a, str>) {}
    fn soft_break(&mut self) {}
    fn hard_break(&mut self) {}
    fn footnote(&mut self, name: Cow<'a, str>) {}
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
