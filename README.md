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

> quote me plz senpai

Oh hai!

[^1]: Footnote1 this is a footnote ref

[1]: http://www.google.com
