# rePocket

Another reMarkable Pocket client for the reMarkable, running on the tablet and _inexpertly_ written in Rust.

## Building

To build `rePocket` 

```bash
# From the repository root
cd rePocket
cargo build --release --target=armv7-unknown-linux-gnueabihf
```

## Notes (to self) ...

_... and to whomever wants to mess with this_

Hhere are some notes regarding the use of certain crates and the implementation of certain features.

### Extracting HTML

Extracting _readable_ HTML from a website seems something that is fairly mature for other languages, but perhaps not as mature in Rust. Two crates are used `readability` and `readable_readability`. The latter is sufficient for 9x% of websites, but I found webistes for which it returned almost nothing. I do admit that it may have been user error, but found that the former works nicely in those cases. So, as it is, the program is doing work twice as much as necessary. However, there are two reasons why I decided to not go with `readability` alone:

1. At the time it did not support async calls (although someone provided a PR for that)

2. It provides less metadata than the alternative, which is kind of a pity (I forked it, perhaps some day...)

### ePUB vs PDF

Creating and ePUB from a HTML seemed a lot easier than creating a PDF. For the PDF I looked at using [headless_chrome](https://crates.io/crates/headless_chrome). However, this seemed overkill. I do know that reMarkable turns ePUBs into PDFs, so perhaps one day I'll look into that.

### HTML vs XHTML

ePUB does expect XHTML rather than HTML, and that was also a challenge. I found that most HTML to XHTML libraries either didn't really _fix_ HTML to make it XHTML. Others were a wrapper around a binary. Since the goal was to make this run on the device, the latter was not an option.

In the end, I ended with a mix of HTLM cleaning via ammonia, regex substitution and plain string substitution. Far from perfect, but working.

### Miscellanea

I had to point to the latest commit of `readability` rather than using crates.io. This was done because I could not, for the life of me, cross-compile OpenSSL. So I decided to go with `rustls`, which is supported in the latest commits but not the official release.
