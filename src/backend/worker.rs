//! Job 실행자. 큐에서 job 꺼내 status 갱신하며 실행.

use crate::config::Config;
use crate::jobs::{Job, JobKind, JobStatus};
use crate::jobs::sentry::SentryStep;
use crate::jobs::translate::TranslateInput;
use anyhow::Result;
use chrono::Utc;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use super::translate::TranslateClient;

pub struct WorkerHandle {
    pub log_rx: mpsc::UnboundedReceiver<String>,
    pub job_tx: mpsc::UnboundedSender<Job>,
    pub state: Arc<Mutex<Vec<Job>>>,
    pub history: Arc<Mutex<Vec<Job>>>,
    pub _task: JoinHandle<()>,
}

pub fn spawn_worker(cfg: Arc<Config>) -> WorkerHandle {
    let (log_tx, log_rx) = mpsc::unbounded_channel::<String>();
    let (job_tx, mut job_rx) = mpsc::unbounded_channel::<Job>();
    let state = Arc::new(Mutex::new(Vec::<Job>::new()));
    let history = Arc::new(Mutex::new(Vec::<Job>::new()));
    let client = TranslateClient::new(cfg.api_endpoints.clone());

    let state_c = state.clone();
    let history_c = history.clone();
    let task = tokio::spawn(async move {
        while let Some(mut job) = job_rx.recv().await {
            job.status = JobStatus::Running;
            job.started_at = Some(Utc::now());
            state_c.lock().await.push(job.clone());
            let _ = log_tx.send(format!("[{}] start: {}", short_id(&job), job.kind.title()));

            let result = run_job(&mut job, &cfg, &client, &log_tx).await;
            job.finished_at = Some(Utc::now());
            match result {
                Ok(()) => {
                    job.status = JobStatus::Done;
                    job.progress = 1.0;
                    let _ = log_tx.send(format!("[{}] done", short_id(&job)));
                }
                Err(e) => {
                    job.status = JobStatus::Failed;
                    job.message = e.to_string();
                    let _ = log_tx.send(format!("[{}] FAILED: {e}", short_id(&job)));
                }
            }
            // 현재 state 에서 제거 + history 로
            {
                let mut s = state_c.lock().await;
                s.retain(|j| j.id != job.id);
            }
            let mut h = history_c.lock().await;
            h.insert(0, job);
            if h.len() > cfg.ui.history_max { h.truncate(cfg.ui.history_max); }
        }
    });

    WorkerHandle { log_rx, job_tx, state, history, _task: task }
}

fn short_id(job: &Job) -> String {
    job.id.to_string().chars().take(8).collect()
}

async fn run_job(
    job: &mut Job,
    cfg: &Config,
    client: &TranslateClient,
    log: &mpsc::UnboundedSender<String>,
) -> Result<()> {
    match &job.kind.clone() {
        JobKind::Translate(t) => match &t.input {
            TranslateInput::Text(text) => {
                let r = client
                    .translate(text, &t.source_lang, &t.target_lang, t.context.as_deref())
                    .await?;
                job.message = r.translation.clone();
                let _ = log.send(format!("  → {}", r.translation));
                Ok(())
            }
            TranslateInput::List(items) => {
                for (i, item) in items.iter().enumerate() {
                    let r = client
                        .translate(item, &t.source_lang, &t.target_lang, t.context.as_deref())
                        .await?;
                    job.progress = (i as f32 + 1.0) / items.len() as f32;
                    let _ = log.send(format!("  [{}/{}] {} → {}", i + 1, items.len(), item, r.translation));
                }
                Ok(())
            }
            TranslateInput::File { path, out } => {
                // delegate to phs-translate CLI for robustness
                let mut cmd = Command::new(&cfg.jobs.translate.cli);
                cmd.arg("-i").arg(path)
                    .arg("-s").arg(&t.source_lang)
                    .arg("-t").arg(&t.target_lang)
                    .arg("-w").arg(cfg.defaults.workers.to_string());
                if let Some(o) = out { cmd.arg("-o").arg(o); }
                if let Some(c) = &t.context { cmd.arg("-c").arg(c); }
                cmd.env("TRANSLATE_API", cfg.api_endpoints.join(","));
                stream_cmd(cmd, job, log).await
            }
        },
        JobKind::SentryI18n(s) => {
            let mut cmd = Command::new(&cfg.jobs.sentry_i18n.cli);
            cmd.arg("--workers").arg(s.workers.to_string());
            if s.cache_bust { cmd.arg("--bust"); }
            if let Some(src) = &s.sources { cmd.arg("--sources").arg(src); }
            if let Some(l) = s.limit { cmd.arg("--limit").arg(l.to_string()); }
            match s.step {
                SentryStep::Extract => { cmd.arg("extract"); }
                SentryStep::Scan => { cmd.arg("scan"); }
                SentryStep::Translate => { cmd.arg("translate"); }
                SentryStep::Build => { cmd.arg("build"); }
                SentryStep::Deploy => { cmd.arg("deploy"); }
                SentryStep::Sync => { cmd.arg("sync"); }
            }
            cmd.env("TRANSLATE_API", cfg.api_endpoints.join(","));
            stream_cmd(cmd, job, log).await
        }
    }
}

async fn stream_cmd(
    mut cmd: Command,
    job: &mut Job,
    log: &mpsc::UnboundedSender<String>,
) -> Result<()> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let log1 = log.clone();
    let log2 = log.clone();
    let j_id = job.id;
    let t1 = tokio::spawn(async move {
        let mut r = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = r.next_line().await {
            let _ = log1.send(format!("[{}] {}", short(j_id), line));
        }
    });
    let t2 = tokio::spawn(async move {
        let mut r = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = r.next_line().await {
            let _ = log2.send(format!("[{}] {}", short(j_id), line));
            // progress parse: "N/M (X%)"
        }
    });
    let _ = tokio::join!(t1, t2);
    let status = child.wait().await?;
    if !status.success() {
        anyhow::bail!("exit code: {}", status.code().unwrap_or(-1));
    }
    Ok(())
}

fn short(id: uuid::Uuid) -> String {
    id.to_string().chars().take(8).collect()
}
