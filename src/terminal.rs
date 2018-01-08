use {color_wheel, reset, style_reset};
use MarkdownError::*;
use escape::{escape_href, escape_html};
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, OPTION_ENABLE_FOOTNOTES,
                     OPTION_ENABLE_TABLES};
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Write};
use std::io::{self, Read};
use termion::color;
use termion::style;

trait MDParser<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    type Output;
    fn parse(&mut self, iter: I) -> Self::Output;
}

enum TableState {
    Head,
    Body,
}

struct Terminal<'a> {
    header_lvl: i32,
    term_size: (u16, u16),
    table_state: TableState,
    table_alignments: Vec<Alignment>,
    table_cell_index: usize,
    links: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    ordered: bool,
    items: usize,
}

impl<'a, I> MDParser<'a, I> for Terminal<'a>
where
    I: Iterator<Item = Event<'a>>,
{
    type Output = String;
    fn parse(&mut self, iter: I) -> Self::Output {
        let mut buf = String::new();
        let mut numbers = HashMap::new();

        for event in iter {
            match event {
                Event::Start(tag) => {
                    self.increment();
                    self.start_tag(tag, &mut buf, &mut numbers);
                }
                Event::End(tag) => {
                    self.decrement();
                    self.end_tag(tag, &mut buf);
                }
                Event::Text(text) => self.write_text(text),
                Event::SoftBreak => self.soft_break(),
                Event::HardBreak => self.hard_break(),
                Event::FootnoteReference(name) => self.footnote(name),
                _ => panic!("html and inline html converted to text, this is unreachable"),
            }
        }
        // write links as footnotes
        let mut links = String::new();
        for (i, &(ref dest, ref title)) in self.links.iter().enumerate() {
            let i = i + 1;
            if !title.is_empty() {
                links.push_str(&format!("[{}] {}: {}\n", i, title, dest));
            } else {
                links.push_str(&format!("[{}] {}\n", i, dest));
            }
        }
        buf.push_str(&links);
        buf
    }
}

impl<'a> Terminal<'a> {
    pub fn new(term_size: (u16, u16)) -> Terminal<'a> {
        Terminal {
            table_state: TableState::Head,
            header_lvl: 0,
            term_size,
            table_alignments: Vec::new(),
            table_cell_index: 0,
            links: Vec::new(),
            ordered: false,
            items: 0,
        }
    }

    fn increment(&mut self) {
        self.header_lvl += 1;
    }

    fn decrement(&mut self) {
        self.header_lvl -= 1;
    }

    fn width(&self) -> usize {
        self.term_size.0 as usize
    }

    fn inc_li(&mut self) {
        self.items = self.items + 1;
    }
    pub fn start_tag(
        &mut self,
        tag: Tag<'a>,
        buf: &mut String,
        numbers: &mut HashMap<Cow<'a, str>, usize>,
    ) {
        match tag {
            Tag::Paragraph => {
                fresh_line(buf);
            }
            Tag::Rule => {
                fresh_line(buf);
                let w = self.width();
                let r = "-".repeat(w);
                buf.push_str(&r);
            }
            Tag::Header(level) => {
                fresh_line(buf);
                let steeze = color_wheel(level, 6);
                let r = steeze + &"#".repeat(level as usize) + " ";
                buf.push_str(&r);
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments;
                buf.push_str("<table>");
            }
            Tag::TableHead => {
                self.table_state = TableState::Head;
                buf.push_str("<thead><tr>");
            }
            Tag::TableRow => {
                self.table_cell_index = 0;
                buf.push_str("<tr>");
            }
            Tag::TableCell => {
                match self.table_state {
                    TableState::Head => buf.push_str("<th"),
                    TableState::Body => buf.push_str("<td"),
                }
                match self.table_alignments.get(self.table_cell_index) {
                    Some(&Alignment::Left) => buf.push_str(" align=\"left\""),
                    Some(&Alignment::Center) => buf.push_str(" align=\"center\""),
                    Some(&Alignment::Right) => buf.push_str(" align=\"right\""),
                    _ => (),
                }
                buf.push_str(">");
            }
            Tag::BlockQuote => {
                fresh_line(buf);
                buf.push_str("<blockquote>");
            }
            Tag::CodeBlock(info) => {
                fresh_line(buf);
                let lang = info.split(' ').next().unwrap();
                if lang.is_empty() {
                    buf.push_str("<pre><code>");
                } else {
                    buf.push_str("<pre><code class=\"language-");
                    escape_html(buf, lang, false);
                    buf.push_str("\">");
                }
            }
            Tag::List(Some(1)) => {
                fresh_line(buf);
                self.ordered = true;
                self.items = 0;
                // buf.push_str("<ol>\n");
            }
            Tag::List(Some(start)) => {
                fresh_line(buf);
                self.ordered = true;
                self.items = start;
                // write!(buf, "<ol start=\"{}\">\n", start);
            }
            Tag::List(None) => {
                fresh_line(buf);
                self.ordered = false;
                // buf.push_str("<ul>\n");
            }
            Tag::Item => {
                fresh_line(buf);
                if self.ordered {
                    self.inc_li();
                    buf.push_str(&(self.items.to_string() + " "));
                } else {
                    buf.push_str("* ");
                }
            }
            Tag::Emphasis => {
                buf.push_str(&format!("{}", style::Italic));
            }
            Tag::Strong => buf.push_str(&format!("{}", style::Bold)),
            Tag::Code => buf.push_str("<code>"),
            Tag::Link(dest, title) => {
                buf.push_str(&format!("{}", style::Underline));
                self.links.push((dest, title));
            }
            Tag::Image(dest, title) => {
                buf.push_str("<img src=\"");
                escape_href(buf, &dest);
                buf.push_str("\" alt=\"");
                //self.raw_text(numbers);
                if !title.is_empty() {
                    buf.push_str("\" title=\"");
                    escape_html(buf, &title, false);
                }
                buf.push_str("\" />")
            }
            Tag::FootnoteDefinition(name) => {
                fresh_line(buf);
                let len = numbers.len() + 1;
                //
                // buf
                //     .push_str("<div class=\"footnote-definition\" id=\"");
                // escape_html(&mut buf, &*name, false);
                // buf
                //     .push_str("\"><sup class=\"footnote-definition-label\">");
                let number = numbers.entry(name).or_insert(len);
                // buf.push_str(&*format!("{}", number));
                buf.push_str(&format!("[^{}] ", number.to_string()));
                //buf.push_str("</sup>");
            }
        }
    }
    pub fn end_tag(&mut self, tag: Tag<'a>, buf: &mut String) {}
    pub fn write_text(&mut self, text: Cow<'a, str>) {}
    pub fn soft_break(&mut self) {}
    pub fn hard_break(&mut self) {}
    pub fn footnote(&mut self, name: Cow<'a, str>) {}
}

fn fresh_line(buf: &mut String) {
    buf.push('\n');
}
