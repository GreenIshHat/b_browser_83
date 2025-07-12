# bBrowser83

**A blazing-fast Rust terminal browser for RSS feeds and HTML, with smart link sorting and interactive navigation.**

## Features

- Accepts any RSS/Atom feed URL or HTML page.
- Detects and parses feeds—lets you pick articles to read.
- For HTML: extracts main text, fetches and sorts links by content size.
- Shows links in pages of 10, sorted largest-first for signal over noise.
- You pick which link to follow; fully recursive navigation.
- Parallel fetching for speed.
- Clean CLI—ready for ratatui (TUI) upgrade and note-taking.
- Pure Rust, minimal dependencies.

## Usage

```sh
cargo run --release
# or use the binary in target/release/b_browser_83

    Enter a feed URL (e.g. https://meneame.net/rss) or HTML page.

    For feeds, select an article by number.

    For HTML, review main content, then choose a link (1–10, or next page).

    Repeat; hit q to quit any time.
```


Next Steps / TODO

ratatui TUI for scrollable, interactive UI

Add notes/bookmarks support

Smarter article parsing (readability)

    Export session to Markdown

No data leaves your machine. No analytics. Pure browsing.

© GreenIshHat
