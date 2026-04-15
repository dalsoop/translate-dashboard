mod app;
mod backend;
mod config;
mod jobs;
mod ui;

use crate::app::{App, Mode, NewJobField, NewJobType};
use crate::backend::{gpu, worker};
use crate::config::Config;
use crate::jobs::sentry::SentryStep;
use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cfg_path = std::env::args().nth(1).map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config.ncl"));
    let cfg: Arc<Config> = Arc::new(
        Config::load(&cfg_path).with_context(|| format!("load {}", cfg_path.display()))?,
    );

    let gpu_rx = gpu::spawn_poller(
        cfg.gpu.host.clone(),
        Duration::from_secs(cfg.gpu.poll_interval_s),
    );
    let mut worker = worker::spawn_worker(cfg.clone());

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, cfg, gpu_rx, &mut worker).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(e) = res {
        eprintln!("error: {e:?}");
        std::process::exit(1);
    }
    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    cfg: Arc<Config>,
    mut gpu_rx: watch::Receiver<backend::gpu::GpuSnapshot>,
    worker: &mut worker::WorkerHandle,
) -> Result<()> {
    let mut app = App::new(cfg.clone());
    let tick = Duration::from_millis(cfg.ui.refresh_ms);
    loop {
        // sync gpu snapshot
        if gpu_rx.has_changed().unwrap_or(false) {
            app.gpu = gpu_rx.borrow_and_update().clone();
        }
        // drain logs
        while let Ok(line) = worker.log_rx.try_recv() {
            app.push_log(line);
        }
        // sync active jobs from worker state
        {
            let active = worker.state.lock().await;
            app.active = active.clone();
        }
        {
            let hist = worker.history.lock().await;
            app.history = hist.clone();
        }

        terminal.draw(|f| ui::draw(f, &app))?;

        if event::poll(tick)? {
            if let Event::Key(key) = event::read()? {
                handle_key(&mut app, key, worker);
                if app.should_quit { break; }
            }
        }
    }
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent, worker: &worker::WorkerHandle) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }
    match app.mode {
        Mode::Normal => match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('n') => app.mode = Mode::NewJob,
            KeyCode::Tab => {
                app.focus = match app.focus {
                    app::Focus::Gpu => app::Focus::Jobs,
                    app::Focus::Jobs => app::Focus::History,
                    app::Focus::History => app::Focus::Log,
                    app::Focus::Log => app::Focus::Gpu,
                };
            }
            _ => {}
        },
        Mode::NewJob => handle_new_job_key(app, key, worker),
    }
}

fn handle_new_job_key(app: &mut App, key: KeyEvent, worker: &worker::WorkerHandle) {
    match (app.new_job.focus, key.code) {
        (_, KeyCode::Esc) => app.mode = Mode::Normal,
        (_, KeyCode::Tab) => app.new_job.next_field(),
        (_, KeyCode::Enter) => {
            if let Some(job) = app.new_job.to_job() {
                let _ = worker.job_tx.send(job);
                app.mode = Mode::Normal;
            }
        }
        (NewJobField::Type, KeyCode::Char('<') | KeyCode::Left) |
        (NewJobField::Type, KeyCode::Char('>') | KeyCode::Right) => {
            app.new_job.job_type = match app.new_job.job_type {
                NewJobType::Translate => NewJobType::Sentry,
                NewJobType::Sentry => NewJobType::Translate,
            };
        }
        (NewJobField::Src, KeyCode::Backspace) => { app.new_job.src_lang.pop(); }
        (NewJobField::Src, KeyCode::Char(c)) if c.is_ascii_alphanumeric() || c == '-' => {
            app.new_job.src_lang.push(c);
        }
        (NewJobField::Tgt, KeyCode::Backspace) => { app.new_job.tgt_lang.pop(); }
        (NewJobField::Tgt, KeyCode::Char(c)) if c.is_ascii_alphanumeric() || c == '-' => {
            app.new_job.tgt_lang.push(c);
        }
        (NewJobField::Main, KeyCode::Backspace) => {
            match app.new_job.job_type {
                NewJobType::Translate => { app.new_job.text.pop(); }
                NewJobType::Sentry => {}
            }
        }
        (NewJobField::Main, KeyCode::Char(c)) => match app.new_job.job_type {
            NewJobType::Translate => app.new_job.text.push(c),
            NewJobType::Sentry => {
                app.new_job.sentry_step = match c {
                    '1' => SentryStep::Extract,
                    '2' => SentryStep::Scan,
                    '3' => SentryStep::Translate,
                    '4' => SentryStep::Build,
                    '5' => SentryStep::Deploy,
                    '6' => SentryStep::Sync,
                    _ => app.new_job.sentry_step,
                };
            }
        },
        (NewJobField::Extra, KeyCode::Backspace) => match app.new_job.job_type {
            NewJobType::Translate => { app.new_job.context.pop(); }
            NewJobType::Sentry => { app.new_job.cache_bust = !app.new_job.cache_bust; }
        },
        (NewJobField::Extra, KeyCode::Char(c)) => match app.new_job.job_type {
            NewJobType::Translate => app.new_job.context.push(c),
            NewJobType::Sentry => {
                if c == 'b' { app.new_job.cache_bust = !app.new_job.cache_bust; }
            }
        },
        _ => {}
    }
}
