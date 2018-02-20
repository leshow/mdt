# mdt

---

[![Build Status](https://travis-ci.org/leshow/mdt.svg?branch=master)](https://travis-ci.org/leshow/mdt)

_mdt_ is a markdown previewer for the **terminal**. It takes markdown as input from `stdin` and prints it out formatted for easy reading. It should support all CommonMark docs (but won't attempt inner html). There are a few things still not working, and the code could use a refactor, but the main work is done.

It uses [pulldown_cmark](http://www.github.com/google/pulldown-cmark) for parsing markdown.

## Usage

**mdt** can take a file name or input from `stdin`. Therefore, the follow all work:

```sh
$ mdt README.md
```

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

1. paragraph
1. rule
1. headers
1. lists (ordered and unordered)
1. bold
1. italic
1. footnotes
1. links
1. tables (ascii tables only for now)

Not working:

1. Images
1. Inline html (not planned)
