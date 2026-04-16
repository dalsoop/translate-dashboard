# translate-dashboard

Ratatui-based terminal UI for managing a self-hosted translation pipeline.
Multiple backends via **connectors**, multi-job queue, GPU fleet visibility.

Pairs with [gemma-translate](https://github.com/dalsoop/gemma-translate) as its default backend.

## Screenshots

![Main view](docs/screenshot-main.svg)
![New-job modal](docs/screenshot-newjob.svg)

## Features

- **4-panel TUI**: GPU (ssh nvidia-smi polling) / active jobs / history / log
- **Connector registry** — swap gemma / deepl / claude at runtime (`c` key)
- **Multi-job queue** — enqueue many translate or sentry-i18n jobs concurrently
- **tui-input** editable fields in "New Job" modal
- **Progress parsing** — picks up `N/M (X%)` from subprocess stderr
- **Cancel** running job with `x`
- **Persistent history** at `~/.local/share/translate-dashboard/history.json`
- **Help overlay** `?`
- **Nickel config** (falls back to JSON)

## Keybindings

| Key | Action |
|-----|--------|
| `n` | New job |
| `?` | Help |
| `Tab`, `Shift-Tab` | Move focus / form fields |
| `↑ ↓` | Select job |
| `x` | Cancel selected |
| `c` | Cycle connector |
| `q`, `Ctrl-C` | Quit |

## Build & Run

```bash
git clone https://github.com/dalsoop/translate-dashboard
cd translate-dashboard
cargo build --release

# optional: Nickel CLI for .ncl config
cargo install nickel-lang-cli

# edit config
vi config.ncl

./target/release/translate-dashboard config.ncl
```

## Smoke Test (no TUI)

```bash
cargo run --release --bin smoke config.ncl
```
Exercises config → connectors → translate roundtrip → GPU poll.

## Repository Layout

```
translate-dashboard/
├── config.ncl                  Nickel config (endpoints, gpu, defaults, connectors)
├── src/
│   ├── main.rs                 event loop + keybindings
│   ├── app.rs                  App state + NewJobForm (tui-input)
│   ├── config.rs               Nickel → JSON → struct
│   ├── ui/                     Ratatui layout + panels + modals
│   ├── backend/
│   │   ├── gpu.rs              ssh nvidia-smi poller (tokio watch)
│   │   ├── translate.rs        TranslateClient (round-robin + retry)
│   │   └── worker.rs           job runner + cancel + history persist
│   ├── connectors/
│   │   ├── gemma.rs            TranslateGemma (our server)
│   │   ├── deepl.rs            DeepL REST
│   │   └── claude.rs           Anthropic Messages API
│   └── bin/
│       ├── screenshot.rs       SVG screenshot generator
│       └── smoke.rs            Headless smoke test
└── docs/screenshot-*.svg
```

## License

MIT. Screenshots and code free to reuse.

## Links

- Backend: [gemma-translate](https://github.com/dalsoop/gemma-translate)
- Example output (Sentry Korean locale): [sentry-korean-locale](https://github.com/dalsoop/sentry-korean-locale)
