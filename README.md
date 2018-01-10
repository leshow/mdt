# mdt (WIP)

---

_mdt_ is a markdown previewer for the **terminal**. It takes markdown as input from `stdin` and prints it out formatted for easy reading. It should support all CommonMark docs (but won't attempt inner html). Work in progress, it mostly still prints html instead of terminal characters.

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

```js
function foo(x) {
  return x * x;
}
```

```rust
    fn highlight_lines(&self, s: &str, buf: &mut String) {
        let ts = ThemeSet::load_defaults();
        let ps = SyntaxSet::load_defaults_nonewlines();

        let syntax = if let Some(ref lang) = self.lang {
            ps.find_syntax_by_name(lang)
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

### Features (currently)

1. paragraph
1. rule
1. headers
1. lists (ordered and unordered)
1. bold
1. italic
1. footnotes
1. links

#### Test header (ignore this)

* list item
* list two
* list three

This is a new paragraph.[^1] And I'm going to reference [a link][1]. Let's do some other stuff:

> quote me plz senpai. This line is part
> of the same quote.

Oh hai!

[^1]: Footnote1 this is a footnote ref

[1]: http://www.google.com
