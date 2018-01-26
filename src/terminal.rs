use MarkdownError::*;
use escape::{escape_href, escape_html};
use pulldown_cmark::{Alignment, Event, Tag};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::io::Write;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use table::{AsciiTable, Table, TableState};
// use syntect::util::as_24_bit_terminal_escaped;
use termion::color;
use termion::style;

lazy_static! {
    static ref RESET_COLOR: String = format!("{}", color::Fg(color::Reset));
    static ref RESET_STYLE: String = format!("{}", style::Reset);
}

pub type TermAscii<'a> = Terminal<'a, AsciiTable>;

pub trait MDParser<'a, I, W>
where
    I: Iterator<Item = Event<'a>>,
    W: Write,
{
    fn parse(&mut self, iter: I, w: &mut W);
}

pub struct Terminal<'a, T> {
    indent_lvl: usize,
    term_size: (u16, u16),
    in_table: bool,
    table_alignments: Vec<Alignment>,
    table: T,
    links: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    ordered: bool,
    items: usize,
    truecolor: bool,
    in_code: bool,
    code: String,
    lang: Option<String>,
    dontskip: bool,
}

impl<'a, T> Default for Terminal<'a, T>
where
    T: Table,
{
    fn default() -> Self {
        Terminal {
            table: T::new(),
            in_table: false,
            table_alignments: Vec::new(),
            in_code: false,
            lang: None,
            dontskip: false,
            ordered: false,
            items: 0,
            code: String::new(),
            indent_lvl: 0,
            term_size: (100, 100),
            links: Vec::new(),
            truecolor: false,
        }
    }
}

impl<'a, I, T, W> MDParser<'a, I, W> for Terminal<'a, T>
where
    I: Iterator<Item = Event<'a>>,
    T: Table + Debug,
    W: Write, // O: From<Cow<'a, str>> + fmt::Write
{
    // type Output = String;
    fn parse(&mut self, iter: I, w: &mut W) {
        let mut buf = String::new();
        let mut numbers = HashMap::new();

        {
            // let mbuf = &mut buf;
            for event in iter {
                match event {
                    Event::Start(tag) => {
                        self.increment();
                        self.start_tag(tag, w, &mut numbers);
                    }
                    Event::End(tag) => {
                        self.decrement();
                        self.end_tag(tag, w);
                    }
                    Event::Text(text) => self.write_buf(w, text),
                    Event::SoftBreak => self.soft_break(),
                    Event::HardBreak => self.hard_break(),
                    Event::FootnoteReference(name) => self.write_buf(w, name),
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
        //buf.push_str(&links);
        write!(buf, "{}", &links);
        // buf
    }
}

impl<'a, T> Terminal<'a, T>
where
    T: Table + Debug,
    // O: From<Cow<'a, str>> + fmt::Write,
{
    pub fn new(term_size: (u16, u16), truecolor: bool) -> Terminal<'a, T> {
        Terminal {
            term_size,
            truecolor,
            ..Terminal::default()
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

    fn start_tag<W: Write>(
        &mut self,
        tag: Tag<'a>,
        buf: &mut W,
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
                // buf.push_str(&"-".repeat(self.width()));
                write!(buf, "{}", &"-".repeat(self.width()));
            }
            Tag::Header(level) => {
                fresh_line(buf);
                // buf.push_str(&format!(
                //     "{}{} {} ",
                //     color::Fg(color::Yellow),
                //     "#".repeat(level as usize),
                //     color::Fg(color::Red)
                // ));
                write!(
                    buf,
                    "{}{} {} ",
                    color::Fg(color::Yellow),
                    "#".repeat(level as usize),
                    color::Fg(color::Red)
                );
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments;
                self.in_table = true;
                // buf.push_str("<table>");
                fresh_line(buf);
            }
            Tag::TableHead => {
                // self.table_state = TableState::Head;
                self.table.set_table_state(TableState::Head);
                // buf.push_str("<thead><tr>");
                write!(buf, "<thead><tr>");
            }
            Tag::TableRow => {
                self.table.set_index(0);
                // buf.push_str("<tr>");
                write!(buf, "<tr>");
            }
            Tag::TableCell => {
                match self.table.table_state() {
                    TableState::Head => write!(buf, "<tr"),
                    TableState::Body => write!(buf, "<td"),
                };

                write!(
                    buf,
                    "{}",
                    match self.table_alignments.get(self.table.index()) {
                        Some(&Alignment::Left) => " align=\"left\"",
                        Some(&Alignment::Center) => " align=\"center\"",
                        Some(&Alignment::Right) => " align=\"right\"",
                        _ => "",
                    }
                );
                write!(buf, ">");
            }
            Tag::BlockQuote => {
                fresh_line(buf);
                write!(
                    buf,
                    "{}{}",
                    color::Fg(color::Green),
                    "   ".repeat(self.indent_lvl) + "> "
                );
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
                //     // escape_h                let align = ;tml(buf, lang, false);
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
                    write!(buf, " {}", &(self.items.to_string() + ". "));
                } else {
                    write!(buf, "{} * ", color::Fg(color::Red));
                    write!(buf, "{}", *RESET_COLOR);
                }
            }
            Tag::Emphasis => {
                write!(buf, "{}", style::Italic);
            }
            Tag::Strong => {
                write!(buf, "{}", style::Bold);
            }
            Tag::Code => {
                write!(buf, "`{}", style::Italic);
            }
            Tag::Link(dest, title) => {
                write!(buf, "{}", style::Underline);
                self.links.push((dest, title));
            }
            Tag::Image(dest, title) => {
                write!(buf, "<img src=\"");
                escape_href(buf, &dest);
                write!(buf, "\" alt=\"");
                //self.raw_text(numbers);
                if !title.is_empty() {
                    write!(buf, "\" title=\"");
                    escape_html(buf, &title, false);
                }
                write!(buf, "\" />");
            }
            Tag::FootnoteDefinition(name) => {
                fresh_line(buf);
                let len = numbers.len() + 1;

                // buf.push_str("<div class=\"footnote-definition\" id=\"");
                // escape_html(buf, &*name, false);
                // buf.push_str("\"><sup class=\"footnote-definition-label\">");

                let number = numbers.entry(name).or_insert(len);
                // buf.push_str(&*format!("{}", number));

                write!(buf, "[^{}] ", number.to_string());
                self.dontskip = true;
            }
        }
    }

    fn end_tag<W: Write>(&mut self, tag: Tag<'a>, buf: &mut W) {
        match tag {
            Tag::Paragraph => fresh_line(buf),
            Tag::Rule => (),
            Tag::Header(_) => {
                fresh_line(buf);
                write!(buf, "{}", *RESET_COLOR);
            }
            Tag::Table(_) => {
                self.in_table = false;
                write!(buf, "</tbody></table>\n");
            }
            Tag::TableHead => {
                write!(buf, "</tr></thead><tbody>\n");
                self.table.set_table_state(TableState::Body);
            }
            Tag::TableRow => {
                write!(buf, "</tr>\n");
            }
            Tag::TableCell => {
                match self.table.table_state() {
                    TableState::Head => {
                        write!(buf, "</th>");
                    }
                    TableState::Body => {
                        write!(buf, "</td>");
                    }
                }
                self.table.inc_index();
                // self.table_cell_index += 1;
            }
            Tag::BlockQuote => {
                write!(buf, "{}", *RESET_COLOR);
            }
            Tag::CodeBlock(_) => {
                self.in_code = false;
                self.write_code(buf);
                write!(buf, "{}", *RESET_COLOR);
                fresh_line(buf);
            }
            Tag::List(Some(_)) => fresh_line(buf), // ol
            Tag::List(None) => fresh_line(buf),
            Tag::Item => (),
            Tag::Emphasis => {
                write!(buf, "{}", *RESET_STYLE);
            }
            Tag::Strong => {
                write!(buf, "{}", *RESET_STYLE);
            }
            Tag::Code => {
                write!(buf, "`");
                write!(buf, "{}", *RESET_STYLE);
            }
            Tag::Link(_, _) => {
                write!(buf, "{}", *RESET_STYLE);
                let num = self.links.len().to_string();
                let l = String::from("[") + &num + "]";
                write!(buf, "{}", &l);
            }
            Tag::Image(_, _) => (), // shouldn't happen, handled in start
            Tag::FootnoteDefinition(_) => {
                fresh_line(buf);
                println!("{:?}", self.table);
            }
        }
    }

    fn soft_break(&mut self) {}

    fn hard_break(&mut self) {}

    fn write_code(&mut self, buf: &mut String) {
        let ts = ThemeSet::load_defaults();
        let ps = SyntaxSet::load_defaults_newlines();
        let syntax = if let Some(ref lang) = self.lang {
            ps.find_syntax_by_token(lang)
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
        // self.code = String::new();
    }

    fn write_buf<W: Write>(&mut self, buf: &mut W, text: Cow<'a, str>) {
        if self.in_code {
            if self.truecolor {
                self.code.push_str(&text);
            } else {
                // buf.push_str(&format!("  {}", text));
                write!(buf, "   {}", text);
            }
        } else if self.in_table {
            //println!("{:?} {:?}", self.table.index(), text);
            self.table.push(&text);
        } else {
            // buf.push_str(&text);
            write!(buf, "{}", text);
        }
    }
}

fn fresh_line(buf: &mut impl Write) {
    // buf.push('\n');
    write!(buf, "\n");
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
