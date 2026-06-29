# Rag

Rag is a feed reader dynamic module for Emacs.

For installation see [INSTALL.org](./INSTALL.org), and for usage see
[USAGE.org](./USAGE.org).

## Motivations

One of the problems that I have with other Emacs feed readers is that
they are all very slow because they are written in Emacs LISP. I need
my feed reader (or anything else I use in Emacs) to be very fast
because on Linux (I use an Arch derivative btw) I use
[EXWM](https://github.com/emacs-exwm/exwm) as my window manager so
whenever ELISP code blocks, I cannot use my computer to do anything
else (like remembering to use Emacs as a text editor). As a result I
want my feed reader to be as fast as possible.

### Prior art

There exists multiple feed readers in Emacs and so far I have only
used newsticker and elfeed and I have issues with the feed readers
that I have tried mainly from performance because they are all written
in ELISP. The feed reader that I like the most is elfeed because of
its ui so it will be the one that I compare my new feed reader to.

| name       | url                                                                 | status              | comments                                                                |
|------------|---------------------------------------------------------------------|---------------------|-------------------------------------------------------------------------|
| newsticker | https://www.gnu.org/software/emacs/manual/html_mono/newsticker.html | builtin             | it downloads feeds at an interval and I don't know how to turn it off   |
| gnus       | https://www.gnu.org/software/emacs/manual/html_node/emacs/Gnus.html | builtin             | it is meant to be used as an email client so atom support is bad        |
| elfeed     | https://github.com/emacs-elfeed/elfeed                              | third-party (melpa) | it is best feed reader as of writing                                    |

## Performance

The rag xml parser parses xml at speeds much faster than elfeed. While
I have not benchmarked, it can tell from an eye test that it is
multitudes faster than elfeed. When I use elfeed, after it downloads
an url I would notice that Emacs would freeze for numerous seconds
while it is parsing. While using rag I have never noticed this issue.

Many design decisions in RAG are made in order to maximize its
performance.

### Architecture

In elfeed, the way xml parsing works is that a [xml buffer is parsed
by libxml into a LISP
linked-list](https://github.com/emacs-elfeed/elfeed/blob/67b7f05ad4ef2695dbbebd4aa573259344d57d73/elfeed-lib.el#L182)
or by a fallback xml parser written in ELISP that is much
slower. Then, the xml data is passed to a function for the
corresponding feed type [to be parsed inefficiently multiple
times](https://github.com/emacs-elfeed/elfeed/blob/67b7f05ad4ef2695dbbebd4aa573259344d57d73/elfeed.el#L402)
(by being walked at every call to `xml-query*`).

In contrast, [rag only needs a string that is immediately passed to a
parser for each feed
type](https://github.com/butterfingres/rag/blob/34648f0355ff7bb560003ed5a22b54af3701c71e/src/lib.rs#L84),
without having to be converted into an intermediate AST which lowers
the memory usage of the parser. Following this, [a parser for the feed
type immediately starts streaming xml
elements](https://github.com/butterfingres/rag/blob/34648f0355ff7bb560003ed5a22b54af3701c71e/src/xml/atom/mod.rs#L104)
so feeds are only ever parsed once.

### Allocator

Elfeed uses the default ELISP allocator + garbage collector because
you cannot change it from ELISP code meaning that it can be very
inefficient since every new `cons` cell created could be a new call to
`malloc` and every garbage collection might cause calls to `free`
which are very heavy functions performance-wise.

To avoid this, [all parsers use a bump
allocator](https://github.com/butterfingres/rag/blob/34648f0355ff7bb560003ed5a22b54af3701c71e/src/xml/parser.rs#L39)
when they need to allocate memory. Bump allocators are a type of
allocator that pre-allocate a block and then allocate memory with a
pointer to the next available section of memory so that allocations
and deallocation operations are fast. A property of bump allocators is
that [their memory can be
re-used](https://github.com/butterfingres/rag/blob/34648f0355ff7bb560003ed5a22b54af3701c71e/lisp/rag-pool.el#L22)
by resetting the pointer so you only have to pay the cost of
allocation once [which is something that is used to get
performance](https://github.com/butterfingres/rag/blob/34648f0355ff7bb560003ed5a22b54af3701c71e/lisp/rag-source.el#L122)
[even in
tests](https://github.com/butterfingres/rag/blob/34648f0355ff7bb560003ed5a22b54af3701c71e/src/xml/rss/mod.rs#L661).

### Display update

Elfeed updates its buffer after every update by [erasing everything
and redrawing
it](https://github.com/emacs-elfeed/elfeed/blob/67b7f05ad4ef2695dbbebd4aa573259344d57d73/elfeed-search.el#L987)
which is an operation that might max out once of your CPU cores when
you have thousands of entries.

To not repeat this mistake, after updates rag [uses a list of new
entries to only perform the minimal amount of
work](https://github.com/butterfingres/rag/blob/34648f0355ff7bb560003ed5a22b54af3701c71e/lisp/rag.el#L250)
by using binary search to find entry locations to insert/delete.

### Memory usage

This was found by opening their respective buffers followed by `M-x
memory-report RET`. RAG performs better than elfeed in memory usage
because [elfeed stores everything in-memory even when
unused](https://github.com/emacs-elfeed/elfeed/blob/67b7f05ad4ef2695dbbebd4aa573259344d57d73/elfeed-db.el#L99)
whereas we use a sqlite database which can lazily import data when
backed by a file.

#### Rag

```txt
    14 MiB  Overall Object Memory Usage
   5.3 MiB  Memory Used By Global Variables
   4.8 MiB  Memory Used By Symbol Plists
   1.5 MiB  Reserved (But Unused) Object Memory
   119 KiB  Total Buffer Memory Usage
       0 B  Total Image Cache Size
```

#### Elfeed

```txt
    31 MiB  Overall Object Memory Usage
    25 MiB  Memory Used By Global Variables
   9.5 MiB  Total Buffer Memory Usage
   5.2 MiB  Memory Used By Symbol Plists
     2 MiB  Reserved (But Unused) Object Memory
       0 B  Total Image Cache Size
```

## Limitations

While I personally think that my feed reader is better than elfeed in
terms of performance, there are some limitations to the approaches
that I took.

###  Development is a pain.

[Since xml parsing is written as a state machine, the code for it is
hideous to read and painstaking to
write.](https://github.com/butterfingres/rag/blob/34648f0355ff7bb560003ed5a22b54af3701c71e/src/xml/rss/mod.rs#L277)

[I even nuked an old
version](https://github.com/butterfingres/rag/commit/886d5a2cc89cf8ebb53103c30952c4446c22b5ef).

### Less portable.

Elfeed and the other feed readers are written in ELISP meaning they
can run on anywhere Emacs can run whereas since this is a dynamic
module, you would need to compile it first which is less portable
because you could only run rag if you have Emacs support and Rust
support for your computer.

### Less hackable.

ELISP is very hackable by design so by making this a dynamic module,
people can only modify built in behaviour by changing the Rust source
code.

As an example, this is a snippet from my dotfiles that parses the [xml
media namespace](https://datatracker.ietf.org/doc/html/rfc7303) (its
used in [youtube's rss
feeds](https://www.youtube.com/feeds/videos.xml?playlist_id=UULPuAXFkgsw1L7xaCfnd5JJOw)
(its the elements in the `media:` namespace)) because upstream code
doesn't parse it.

```elisp
(defun my-elfeed-media-parse-hook (ty xml entry)
  "Parse the media extension to the atom specs.

TY should be :atom.
XML should be the parsed xml entry.
ENTRY should be the `elfeed-entry' object."
  (when-let* ((content (and (eq ty :atom)
                            (null (elfeed-entry-content entry))
                            (xml-query* (group description *) xml))))
    (setf (elfeed-entry-content entry) content)))

(add-hook 'elfeed-new-entry-parse-hook 'my-elfeed-media-parse-hook)
```

The only way to do this in rag would be to modify the source code and
then building so the other Emacs feed readers are actually more
powerful because you can modify their code downstream with things like
hooks and
[advice](https://www.gnu.org/software/emacs/manual/html_node/elisp/Advising-Functions.html). This
isn't a problem for me, since I am both the developer and user so I
can put whatever I want in but this might be an issue for users.
