# mdt

---

_mdt_ is a markdown previewer for the **terminal**. It takes markdown as input from `stdin` and prints it out formatted for easy reading. It should support all CommonMark docs (but won't attempt inner html). There are a few things still not working, and the code could use a refactor, but the main work is done.

It uses [pulldown_cmark](http://www.github.com/google/pulldown-cmark) for parsing markdown.

## Usage

```sh
$ cat README.md | mdt
[...]
```

or, more tersely,

```sh
$ mdt < README.md
[...]
```

If you have a terminal that supports truecolor (24-bit color), you can pass a flag `-t` to improve the output color. Default terminal colors map to 256-bit color. Unfortunately it doesn't seem like there's a good way to detect truecolor. If anyone knows of a way please PR or suggest how.

### Supported Markdown Features

1.  paragraph
1.  rule
1.  headers
1.  lists (ordered and unordered)
1.  bold
1.  italic
1.  footnotes
1.  links
1.  tables (ascii tables only for now)

Not working:

1.  Images
1.  Inline html (not planned)

#### TEST SECTION (ignore below)

try highlighting,

```js
function foo(x) {
  return x * x;
}
```

and rust,

```rust
fn highlight_lines(&self, s: &str, buf: &mut String) {
    let ts = ThemeSet::load_defaults();
    let ps = SyntaxSet::load_defaults_nonewlines();

    let syntax = if let Some(ref lang) = self.lang {
        ps.find_syntax_by_token(lang)
    } else {
        ps.find_syntax_by_first_line(s)
    }.unwrap_or_else(|| ps.find_syntax_plain_text());

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    for line in s.lines() {
        let ranges: Vec<(Style, &str)> = h.highlight(line);
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        buf.push_str(&escaped);
    }
}
```

Tables:

| Feature     | Working                                                             | Other |
| ----------- | ------------------------------------------------------------------- | ----- |
| paragraphs  | foo other stuff that will run on and on and on and on and on and on | foo   |
| rule        | done                                                                | foo   |
| headers     | done                                                                | foo   |
| lists       | done                                                                | foo   |
| bold        | done                                                                | foo   |
| italic      | done                                                                | foo   |
| footnotes   | done                                                                | fooo  |
| links       | done                                                                | foo   |
| tables      | done                                                                | fooo  |
| images      | :poop:                                                              | foo   |
| inline html | :poop:                                                              | foo   |

Unordered List:

* list item
* list two
* list three

This is a footer.[^1] And I'm going to reference [a link][1]. Let's do some other stuff:

> quote me plz senpai.

Goodbye!!

[^1]: Footnote1 this is a footnote ref

[1]: http://www.github.com/leshow/mdt
