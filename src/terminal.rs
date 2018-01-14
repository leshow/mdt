use MarkdownError::*;
use escape::{escape_href, escape_html};
use pulldown_cmark::{Alignment, Event, Tag};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Formatter, Write};
use std::io::{self, Read};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, Theme, ThemeSet};
use syntect::parsing::{ScopeStack, SyntaxDefinition, SyntaxSet};
// use syntect::util::as_24_bit_terminal_escaped;
use termion::color::{self, Color, Rgb};
use termion::style;

lazy_static! {
    static ref RESET_COLOR: String = format!("{}", color::Fg(color::Reset));
    static ref RESET_STYLE: String = format!("{}", style::Reset);
}

pub trait MDParser<'a, I>
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

struct Truecolor(bool);

impl Default for Truecolor {
    fn default() -> Self {
        Truecolor(false)
    }
}

pub struct Terminal<'a> {
    indent_lvl: usize,
    term_size: (u16, u16),
    table_state: TableState,
    table_alignments: Vec<Alignment>,
    table_cell_index: usize,
    links: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    ordered: bool,
    items: usize,
    truecolor: bool,
    in_code: bool,
    code: String,
    lang: Option<String>,
    dontskip: bool,
}

impl<'a, I> MDParser<'a, I> for Terminal<'a>
where
    I: Iterator<Item = Event<'a>>,
{
    type Output = String;
    fn parse(&mut self, iter: I) -> Self::Output {
        let mut buf = String::new();
        let mut numbers = HashMap::new();

        {
            let mbuf = &mut buf;
            for event in iter {
                match event {
                    Event::Start(tag) => {
                        self.increment();
                        self.start_tag(tag, mbuf, &mut numbers);
                    }
                    Event::End(tag) => {
                        self.decrement();
                        self.end_tag(tag, mbuf);
                    }
                    Event::Text(text) => self.write_buf(mbuf, text),
                    Event::SoftBreak => self.soft_break(),
                    Event::HardBreak => self.hard_break(),
                    Event::FootnoteReference(name) => self.write_buf(mbuf, name),
                    _ => panic!("html and inline html converted to text, this is unreachable"),
                }
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
    pub fn new(term_size: (u16, u16), truecolor: bool) -> Terminal<'a> {
        Terminal {
            table_state: TableState::Head,
            indent_lvl: 0,
            term_size,
            table_alignments: Vec::new(),
            table_cell_index: 0,
            links: Vec::new(),
            ordered: false,
            items: 0,
            truecolor,
            in_code: false,
            code: String::new(),
            lang: None,
            dontskip: false,
        }
    }

    fn increment(&mut self) {
        self.indent_lvl += 1;
    }

    fn decrement(&mut self) {
        self.indent_lvl -= 1;
    }

    fn width(&self) -> usize {
        self.term_size.0 as usize
    }

    fn inc_li(&mut self) {
        self.items = self.items + 1;
    }

    fn start_tag(
        &mut self,
        tag: Tag<'a>,
        buf: &mut String,
        numbers: &mut HashMap<Cow<'a, str>, usize>,
    ) {
        match tag {
            Tag::Paragraph => {
                if !self.dontskip {
                    fresh_line(buf);
                }
                self.dontskip = false;
            }
            Tag::Rule => {
                fresh_line(buf);
                buf.push_str(&"-".repeat(self.width()));
            }
            Tag::Header(level) => {
                fresh_line(buf);
                let steeze = format!("{}", color::Fg(color::Red));
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
                buf.push_str(&format!(
                    "{}{}",
                    color::Fg(color::Green),
                    "   ".repeat(self.indent_lvl) + "> "
                ));
                self.dontskip = true;
            }
            Tag::CodeBlock(info) => {
                fresh_line(buf);
                // let lang = info.split(' ').next().unwrap();
                self.lang = info.split(' ').next().map(String::from);
                self.in_code = true;
                // if lang.is_empty() {
                //     // buf.push_str("<pre><code>");

                // } else {
                //     // buf.push_str("<pre><code class=\"language-");
                //     // escape_html(buf, lang, false);
                //     // buf.push_str("\">");

                // }
            }
            Tag::List(Some(1)) => {
                fresh_line(buf);
                // <ol>
                self.ordered = true;
                self.items = 0;
            }
            Tag::List(Some(start)) => {
                fresh_line(buf);
                // <ol start=start>
                self.ordered = true;
                self.items = start;
                // write!(buf, "<ol start=\"{}\">\n", start);
            }
            Tag::List(None) => {
                // UL
                fresh_line(buf);
                self.ordered = false;
            }
            Tag::Item => {
                fresh_line(buf);
                if self.ordered {
                    self.inc_li();
                    buf.push_str(&(self.items.to_string() + ". "));
                } else {
                    buf.push_str("* ");
                }
            }
            Tag::Emphasis => {
                buf.push_str(&format!("{}", style::Italic));
            }
            Tag::Strong => buf.push_str(&format!("{}", style::Bold)),
            Tag::Code => buf.push_str(&format!("`{}", style::Italic)),
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

                // buf.push_str("<div class=\"footnote-definition\" id=\"");
                // escape_html(buf, &*name, false);
                // buf.push_str("\"><sup class=\"footnote-definition-label\">");

                let number = numbers.entry(name).or_insert(len);
                // buf.push_str(&*format!("{}", number));

                buf.push_str(&format!("[^{}] ", number.to_string()));
                self.dontskip = true;
            }
        }
    }

    fn end_tag(&mut self, tag: Tag<'a>, buf: &mut String) {
        match tag {
            Tag::Paragraph => fresh_line(buf),
            Tag::Rule => (),
            Tag::Header(_) => {
                fresh_line(buf);
                buf.push_str(&RESET_COLOR);
            }
            Tag::Table(_) => {
                buf.push_str("</tbody></table>\n");
            }
            Tag::TableHead => {
                buf.push_str("</tr></thead><tbody>\n");
                self.table_state = TableState::Body;
            }
            Tag::TableRow => {
                buf.push_str("</tr>\n");
            }
            Tag::TableCell => {
                match self.table_state {
                    TableState::Head => buf.push_str("</th>"),
                    TableState::Body => buf.push_str("</td>"),
                }
                self.table_cell_index += 1;
            }
            Tag::BlockQuote => buf.push_str(&RESET_COLOR),
            Tag::CodeBlock(_) => {
                self.in_code = false;
                self.write_code(buf);
                buf.push_str(&RESET_COLOR);
                fresh_line(buf);
            }
            Tag::List(Some(_)) => fresh_line(buf), // ol
            Tag::List(None) => fresh_line(buf),
            Tag::Item => (),
            Tag::Emphasis => buf.push_str(&RESET_STYLE),
            Tag::Strong => buf.push_str(&RESET_STYLE),
            Tag::Code => {
                buf.push_str("`");
                buf.push_str(&RESET_STYLE);
            }
            Tag::Link(_, _) => {
                buf.push_str(&RESET_STYLE);
                let num = self.links.len().to_string();
                let l = String::from("[") + &num + "]";
                buf.push_str(&l);
            }
            Tag::Image(_, _) => (), // shouldn't happen, handled in start
            Tag::FootnoteDefinition(_) => fresh_line(buf),
        }
    }

    fn soft_break(&mut self) {}

    fn hard_break(&mut self) {}

    fn write_code(&mut self, buf: &mut String) {
        let ts = ThemeSet::load_defaults();
        let ps = SyntaxSet::load_defaults_newlines();

        let syntax = if let Some(ref lang) = self.lang {
            ps.find_syntax_by_extension(lang)
        } else {
            ps.find_syntax_by_first_line(&self.code)
        }.unwrap_or_else(|| ps.find_syntax_plain_text());
        let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
        for line in self.code.lines() {
            let regions: Vec<(Style, &str)> = h.highlight(&line);
            let highlighted = format!("  {}", as_24_bit_terminal_escaped(&regions[..], false));
            buf.push_str(&highlighted);
            buf.push_str("\n");
        }
        // Clear the formatting
        buf.push_str("\x1b[0m");
        self.code = String::new();
    }

    fn write_buf(&mut self, buf: &mut String, text: Cow<'a, str>) {
        if self.in_code {
            if self.truecolor {
                self.code.push_str(&text);
            } else {
                buf.push_str(&format!("  {}", text));
            }
        } else {
            buf.push_str(&text);
        }
    }
}

fn fresh_line(buf: &mut String) {
    buf.push('\n');
}

fn color_wheel(level: i32, m: i32) -> String {
    match level % m {
        1 => format!("{}", color::Fg(color::White)),
        2 => format!("{}", color::Fg(color::Magenta)),
        3 => format!("{}", color::Fg(color::Cyan)),
        4 => format!("{}", color::Fg(color::Red)),
        5 => format!("{}", color::Fg(color::Green)),
        _ => format!("{}", color::Fg(color::Blue)),
    }
}
fn as_24_bit_terminal_escaped(v: &[(Style, &str)], bg: bool) -> String {
    let mut res: String = String::new();

    for &(ref style, text) in v.iter() {
        // let Fg = Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
        // let Bg = Rgb(style.background.r, style.background.g, style.foreground.b);
        // Fg.write_fg(&mut res);
        if bg {
            write!(
                res,
                "\x1b[48;2;{};{};{}m",
                style.background.r, style.background.g, style.background.b
            ).unwrap();
        }
        write!(
            res,
            "\x1b[38;2;{};{};{}m{}",
            style.foreground.r, style.foreground.g, style.foreground.b, text
        ).unwrap();
        // write!(
        //     res,
        //     "\x1b[38;5;{}m{}",
        //     fromrgb(style.foreground.r, style.foreground.g, style.foreground.b),
        //     text
        // ).unwrap();
    }
    res.push_str("\x1b[0m");

    res
}

fn fromrgb(r: u8, g: u8, b: u8) -> u16 {
    return 16 + 36 * (r as u16) + 6 * (g as u16) + (b as u16);
}

macro_rules! csi {
    ($( $l:expr ),*) => { concat!("\x1B[", $( $l ),* ) };
}
