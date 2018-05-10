use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::{Result, Write};

use pulldown_cmark::{Alignment, Event, Tag};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use table::{AsciiTable, Table, TableState, UnicodeTable};
use termion::color;
use termion::style;

lazy_static! {
    static ref RESET_COLOR: String = format!("{}", color::Fg(color::Reset));
    static ref RESET_STYLE: String = format!("{}", style::Reset);
}

// to simplify creating variants
pub type TermAscii<'a> = Terminal<'a, AsciiTable<'a>>;
pub type TermUnicode<'a> = Terminal<'a, UnicodeTable<'a>>;

pub enum TermStyle<'a> {
    Ascii(TermAscii<'a>),
    Unicode(TermUnicode<'a>),
}

impl<'a> TermStyle<'a> {
    pub fn ascii(term_size: (u16, u16), truecolor: bool) -> Self {
        TermStyle::Ascii(TermAscii::new(term_size, truecolor))
    }
    pub fn unicode(term_size: (u16, u16), truecolor: bool) -> Self {
        TermStyle::Unicode(TermUnicode::new(term_size, truecolor))
    }
}

impl<'a, I, W> MDParser<'a, I, W> for TermStyle<'a>
where
    I: Iterator<Item = Event<'a>>,
    W: Write,
{
    fn parse(&mut self, iter: I, w: &mut W) -> Result<()> {
        match self {
            &mut TermStyle::Ascii(ref mut t) => t.parse(iter, w),
            &mut TermStyle::Unicode(ref mut t) => t.parse(iter, w),
        }
    }
}

// main trait impl
pub trait MDParser<'a, I, W>
where
    I: Iterator<Item = Event<'a>>,
    W: Write,
{
    fn parse(&mut self, iter: I, w: &mut W) -> Result<()>;
}

pub struct Terminal<'a, T> {
    indent_lvl: usize,
    term_size: (u16, u16),
    links: Vec<(Cow<'a, str>, Cow<'a, str>)>,
    truecolor: bool,
    dontskip: bool,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    state: State<T>,
}

enum State<T> {
    Code {
        code: String,
        lang: Option<String>,
    },
    Table {
        table_alignments: Vec<Alignment>,
        table: T,
    },
    Li,
    Ol {
        items: usize,
    },
    Nil,
}

impl<T> Default for State<T> {
    fn default() -> Self {
        State::Nil
    }
}

impl<'a, T> State<T>
where
    T: Table<'a>,
{
    fn table(table_alignments: Vec<Alignment>, width: usize) -> State<T> {
        State::Table {
            table: T::new(width),
            table_alignments,
        }
    }
    fn set_table_state(&mut self, table_state: TableState) {
        match *self {
            State::Table { ref mut table, .. } => table.set_table_state(table_state),
            _ => {}
        }
    }
    fn table_draw<W: Write>(&mut self, buf: &mut W) -> Result<()> {
        match *self {
            State::Table { ref mut table, .. } => table.draw(buf)?,
            _ => {}
        }
        Ok(())
    }
    fn table_inc_index(&mut self) {
        match *self {
            State::Table { ref mut table, .. } => table.inc_index(),
            _ => {}
        }
    }
    fn set_table_index(&mut self, idx: usize) {
        match *self {
            State::Table { ref mut table, .. } => table.set_index(idx),
            _ => {}
        }
    }
    fn code(lang: Option<String>) -> State<T> {
        State::Code {
            code: String::new(),
            lang,
        }
    }
    fn write_buf<W: Write>(&mut self, buf: &mut W, text: Cow<'a, str>) -> Result<()> {
        match *self {
            State::Code { ref mut code, .. } => code.push_str(&text),
            State::Table { ref mut table, .. } => table.push(text),
            _ => write!(buf, "{}", text)?,
        }
        Ok(())
    }

    fn li() -> State<T> {
        State::Li
    }
    fn ol(start: usize) -> State<T> {
        State::Ol { items: start }
    }
    fn inc_li<W: Write>(&mut self, buf: &mut W) -> Result<()> {
        match *self {
            State::Ol { ref mut items } => {
                *items = *items + 1;
                write!(buf, " {}", &(items.to_string() + ". "))?;
            }
            _ => {
                write!(buf, "{} * ", color::Fg(color::Red))?;
                write!(buf, "{}", *RESET_COLOR)?;
            }
        };
        Ok(())
    }
}

impl<'a, T> Default for Terminal<'a, T>
where
    T: Table<'a>,
{
    fn default() -> Self {
        Terminal {
            dontskip: false,
            indent_lvl: 0,
            term_size: (100, 100),
            links: Vec::new(),
            truecolor: false,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            state: State::Nil,
        }
    }
}

impl<'a, I, T, W> MDParser<'a, I, W> for Terminal<'a, T>
where
    I: Iterator<Item = Event<'a>>,
    T: Table<'a> + Debug,
    W: Write,
{
    fn parse(&mut self, iter: I, w: &mut W) -> Result<()> {
        let mut numbers = HashMap::new();

        for event in iter {
            match event {
                Event::Start(tag) => {
                    self.increment();
                    self.start_tag(tag, w, &mut numbers)?;
                }
                Event::End(tag) => {
                    self.decrement();
                    self.end_tag(tag, w)?;
                }
                Event::InlineHtml(html) | Event::Html(html) => self.state.write_buf(w, html)?,
                Event::Text(text) => self.state.write_buf(w, text)?,
                Event::SoftBreak => self.soft_break(),
                Event::HardBreak => self.hard_break(),
                Event::FootnoteReference(name) => self.state.write_buf(w, name)?,
            }
        }

        // write links as footnotes
        for (i, &(ref dest, ref title)) in self.links.iter().enumerate() {
            let i = i + 1;
            if !title.is_empty() {
                write!(w, "[{}] {}: {}\n", i, title, dest).unwrap();
            } else {
                write!(w, "[{}] {}\n", i, dest).unwrap();
            }
        }
        Ok(())
    }
}

impl<'a, T> Terminal<'a, T>
where
    T: Table<'a> + Debug,
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

    fn start_tag<W: Write>(
        &mut self,
        tag: Tag<'a>,
        buf: &mut W,
        numbers: &mut HashMap<Cow<'a, str>, usize>,
    ) -> Result<()> {
        match tag {
            Tag::Paragraph => {
                if !self.dontskip {
                    fresh_line(buf)?;
                }
                self.dontskip = false;
            }
            Tag::Rule => {
                fresh_line(buf)?;
                write!(buf, "{}", &"-".repeat(self.width()))?;
            }
            Tag::Header(level) => {
                fresh_line(buf)?;
                write!(
                    buf,
                    "{}{} {} ",
                    color::Fg(color::Yellow),
                    "#".repeat(level as usize),
                    color::Fg(color::Red)
                )?;
            }
            Tag::Table(alignments) => {
                fresh_line(buf)?;
                self.state = State::table(alignments, self.width());
            }
            Tag::TableHead => {
                self.state.set_table_state(TableState::Head);
            }
            Tag::TableRow => {
                self.state.set_table_index(0);
            }
            Tag::TableCell => {
                // write!(
                //     buf,
                //     "{}",
                //     match self.table_alignments.get(self.table.index()) {
                //         Some(&Alignment::Left) => " align=\"left\"",
                //         Some(&Alignment::Center) => " align=\"center\"",
                //         Some(&Alignment::Right) => " align=\"right\"",
                //         _ => "",
                //     }
                // )?;
            }
            Tag::BlockQuote => {
                fresh_line(buf)?;
                write!(
                    buf,
                    "{}{}> ",
                    color::Fg(color::Green),
                    "   ".repeat(self.indent_lvl)
                )?;
                self.dontskip = true;
            }
            Tag::CodeBlock(info) => {
                fresh_line(buf)?;
                self.state = State::code(info.split(' ').next().map(String::from));
            }
            Tag::List(Some(1)) => {
                fresh_line(buf)?;
                // <ol>
                self.state = State::ol(0);
            }
            Tag::List(Some(start)) => {
                fresh_line(buf)?;
                // <ol start=start>
                self.state = State::ol(start);
            }
            Tag::List(None) => {
                // UL
                fresh_line(buf)?;
                self.state = State::li();
            }
            Tag::Item => {
                fresh_line(buf)?;
                self.state.inc_li(buf)?;
            }
            Tag::Emphasis => {
                write!(buf, "{}", style::Italic)?;
            }
            Tag::Strong => {
                write!(buf, "{}", style::Bold)?;
            }
            Tag::Code => {
                write!(buf, "`{}", style::Italic)?;
            }
            Tag::Link(dest, title) => {
                write!(buf, "{}", style::Underline)?;
                self.links.push((dest, title));
            }
            Tag::Image(dest, title) => {
                write!(buf, "<img src=\"{}", &dest)?;
                write!(buf, "\" alt=\"")?;
                //self.raw_text(numbers);
                if !title.is_empty() {
                    write!(buf, "\" title=\"{}", &title)?;
                }
                write!(buf, "\" />")?;
            }
            Tag::FootnoteDefinition(name) => {
                fresh_line(buf)?;
                let len = numbers.len() + 1;

                // buf.push_str("<div class=\"footnote-definition\" id=\"");
                // escape_html(buf, &*name, false);
                // buf.push_str("\"><sup class=\"footnote-definition-label\">");

                let number = numbers.entry(name).or_insert(len);

                write!(buf, "[^{}] ", number.to_string())?;
                self.dontskip = true;
            }
        }
        Ok(())
    }

    fn end_tag<W: Write>(&mut self, tag: Tag<'a>, buf: &mut W) -> Result<()> {
        match tag {
            Tag::Paragraph => fresh_line(buf)?,
            Tag::Rule => (),
            Tag::Header(_) => {
                write!(buf, "{}\n", *RESET_COLOR)?;
            }
            Tag::Table(_) => {
                // self.in_table = false;
                self.state.table_draw(buf)?;
                self.state = State::default();
            }
            Tag::TableHead => {
                self.state.set_table_state(TableState::Body);
            }
            Tag::TableRow => {}
            Tag::TableCell => {
                self.state.table_inc_index();
            }
            Tag::BlockQuote => {
                write!(buf, "{}", *RESET_COLOR)?;
            }
            Tag::CodeBlock(_) => {
                self.write_code(buf)?;
                self.state = State::default();
                write!(buf, "{}\n", *RESET_COLOR)?;
            }
            Tag::List(Some(_)) => fresh_line(buf)?, // ol
            Tag::List(None) => fresh_line(buf)?,
            Tag::Item => (),
            Tag::Emphasis => {
                write!(buf, "{}", *RESET_STYLE)?;
            }
            Tag::Strong => {
                write!(buf, "{}", *RESET_STYLE)?;
            }
            Tag::Code => {
                write!(buf, "`")?;
                write!(buf, "{}", *RESET_STYLE)?;
            }
            Tag::Link(_, _) => {
                write!(buf, "{}", *RESET_STYLE)?;
                let num = self.links.len().to_string();
                let l = "[".to_string() + &num + "]";
                write!(buf, "{}", &l)?;
            }
            Tag::Image(_, _) => (), // shouldn't happen, handled in start
            Tag::FootnoteDefinition(_) => {
                fresh_line(buf)?;
            }
        }
        Ok(())
    }

    fn soft_break(&mut self) {}

    fn hard_break(&mut self) {}

    fn write_code<W: Write>(&mut self, buf: &mut W) -> Result<()> {
        match self.state {
            State::Code {
                ref mut code,
                ref mut lang,
            } => {
                let ts = &self.theme_set.themes["Solarized (dark)"];
                let ps = &self.syntax_set;

                let syntax = if let Some(ref lang) = *lang {
                    ps.find_syntax_by_token(lang)
                } else {
                    ps.find_syntax_by_first_line(&code)
                }.unwrap_or_else(|| ps.find_syntax_plain_text());

                let mut h = HighlightLines::new(syntax, ts);
                for line in code.lines() {
                    let regions: Vec<(Style, &str)> = h.highlight(&line);
                    if self.truecolor {
                        as_24_bit_terminal_escaped(buf, &regions[..], false)?;
                    } else {
                        write_as_ansi(buf, &regions)?;
                    }
                    write!(buf, "\n")?;
                }
                // Clear the formatting
                write!(buf, "\x1b[0m")?;
            }
            _ => {}
        }
        Ok(())
    }
}

fn fresh_line<W: Write>(buf: &mut W) -> Result<()> {
    write!(buf, "\n")?;
    Ok(())
}

fn as_24_bit_terminal_escaped<W: Write>(w: &mut W, v: &[(Style, &str)], bg: bool) -> Result<()> {
    for &(ref style, text) in v.iter() {
        if bg {
            write!(
                w,
                "\x1b[48;2;{};{};{}m",
                style.background.r, style.background.g, style.background.b
            )?;
        }
        write!(
            w,
            "\x1b[38;2;{};{};{}m{}",
            style.foreground.r, style.foreground.g, style.foreground.b, text
        )?;
    }
    write!(w, "\x1b[0m")?;

    Ok(())
}

fn write_as_ansi<W: Write>(w: &mut W, regions: &[(Style, &str)]) -> Result<()> {
    for &(style, text) in regions {
        let rgb = {
            let fg = style.foreground;
            (fg.r, fg.g, fg.b)
        };
        match rgb {
            // base03, base02, base01, base00, base0, base1, base2, and base3
            (0x00, 0x2b, 0x36)
            | (0x07, 0x36, 0x42)
            | (0x58, 0x6e, 0x75)
            | (0x65, 0x7b, 0x83)
            | (0x83, 0x94, 0x96)
            | (0x93, 0xa1, 0xa1)
            | (0xee, 0xe8, 0xd5)
            | (0xfd, 0xf6, 0xe3) => write!(w, "{}", color::Fg(color::Reset))?,
            (0xb5, 0x89, 0x00) => write!(w, "{}", color::Fg(color::Yellow))?, // yellow
            (0xcb, 0x4b, 0x16) => write!(w, "{}", color::Fg(color::LightRed))?, // orange
            (0xdc, 0x32, 0x2f) => write!(w, "{}", color::Fg(color::Red))?,    // red
            (0xd3, 0x36, 0x82) => write!(w, "{}", color::Fg(color::Magenta))?, // magenta
            (0x6c, 0x71, 0xc4) => write!(w, "{}", color::Fg(color::LightMagenta))?, /* violet */
            (0x26, 0x8b, 0xd2) => write!(w, "{}", color::Fg(color::Blue))?,   // blue
            (0x2a, 0xa1, 0x98) => write!(w, "{}", color::Fg(color::Cyan))?,   // cyan
            (0x85, 0x99, 0x00) => write!(w, "{}", color::Fg(color::Green))?,  // green
            (r, g, b) => panic!("Unexpected RGB colour: #{:2>0x}{:2>0x}{:2>0x}", r, g, b),
        };
        let font = style.font_style;
        if font.contains(FontStyle::BOLD) {
            write!(w, "{}", style::Bold)?;
        } else {
            write!(w, "{}", style::NoBold)?;
        }
        if font.contains(FontStyle::ITALIC) {
            write!(w, "{}", style::Italic)?;
        } else {
            write!(w, "{}", style::NoItalic)?;
        }
        if font.contains(FontStyle::UNDERLINE) {
            write!(w, "{}", style::Underline)?;
        } else {
            write!(w, "{}", style::NoUnderline)?;
        }
        write!(w, "{}{}", text, style::Reset)?;
    }
    Ok(())
}
