# Contributing to Quillwright

Thanks for your interest! Quillwright is a small project, and every kind of help is welcome:
bug reports, ideas from writers, documentation, and code.

## Questions and ideas

- **Questions, ideas, and show-and-tell:** use
  [Discussions](https://github.com/fabiocmazzo/quillwright/discussions).
- **Bugs and concrete feature requests:** open an
  [issue](https://github.com/fabiocmazzo/quillwright/issues/new/choose).

You don't need to be a programmer to help. Telling us how you write, what slows you down, and
what you miss from other tools shapes the project as much as code does.

## Working on the code

Start with [`AGENTS.md`](AGENTS.md). It is written for coding agents but serves people just
as well: it covers the architecture, the repository map, the ground rules, and a list of
suggested next steps. Issues labelled
[good first issue](https://github.com/fabiocmazzo/quillwright/labels/good%20first%20issue)
are a good place to start.

You need a recent stable Rust toolchain. On Linux you also need the GTK 3 development headers
(see the README).

```bash
cargo run                     # run the app
cargo test                    # unit tests, about a second
```

To try the app without touching your real settings:

```bash
XDG_CONFIG_HOME=/tmp/qw/cfg XDG_DATA_HOME=/tmp/qw/data cargo run
```

### Before opening a pull request

CI runs these on Linux, Windows, and macOS, and all three must be clean:

```bash
cargo fmt
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

Also:

- **Write everything in English:** code, comments, UI text, and docs.
- **Follow the ground rules in `AGENTS.md`.** The most important:
  - never lose or silently overwrite the author's text;
  - AI output never enters the manuscript without the author accepting it;
  - projects stay plain Markdown, with no hidden metadata;
  - export stays pure Rust, with no external programs.
- **Add tests next to the code** you change (`#[cfg(test)] mod tests`).
- **When you add a config option,** update `docs/config.example.toml` and the README.
- **Keep pull requests focused,** one change at a time, and describe what you tested.

## License

By contributing, you agree that your contributions are licensed under the
[MIT License](LICENSE).
