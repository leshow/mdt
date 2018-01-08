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

### Test header

* list item
* list two
* list three

This is a new paragraph.[^1] And I'm going to reference [a link][1]. Let's do some other stuff:

> quote me plz senpai

Oh hai!

1. one
1. two
1. three

[^1]: Footnote1 this is a footnote ref

[1]: http://www.google.com
