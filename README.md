# mdt (WIP)

---

mdt is a markdown previewer for the terminal. It takes markdown as input from `stdin` and prints it out formatted for easy reading.

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
