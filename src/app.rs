use crate::backend::gpu::GpuSnapshot;
use crate::config::Config;
use crate::jobs::{sentry::{SentryJob, SentryStep}, translate::{TranslateInput, TranslateJob}, Job, JobKind};
use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode { Normal, NewJob }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus { Gpu, Jobs, History, Log }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewJobType { Translate, Sentry }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewJobField { Type, Src, Tgt, Main, Extra }

pub struct NewJobForm {
    pub focus: NewJobField,
    pub job_type: NewJobType,
    pub src_lang: String,
    pub tgt_lang: String,
    pub text: String,
    pub context: String,
    pub sentry_step: SentryStep,
    pub cache_bust: bool,
}

impl NewJobForm {
    pub fn new(cfg: &Config) -> Self {
        Self {
            focus: NewJobField::Type,
            job_type: NewJobType::Translate,
            src_lang: cfg.defaults.source_lang.clone(),
            tgt_lang: cfg.defaults.target_lang.clone(),
            text: String::new(),
            context: cfg.defaults.context.clone(),
            sentry_step: SentryStep::Sync,
            cache_bust: true,
        }
    }

    pub fn next_field(&mut self) {
        self.focus = match self.focus {
            NewJobField::Type => NewJobField::Src,
            NewJobField::Src => NewJobField::Tgt,
            NewJobField::Tgt => NewJobField::Main,
            NewJobField::Main => NewJobField::Extra,
            NewJobField::Extra => NewJobField::Type,
        };
    }

    pub fn to_job(&self) -> Option<Job> {
        match self.job_type {
            NewJobType::Translate => {
                if self.text.trim().is_empty() { return None; }
                let input = if std::path::Path::new(&self.text).exists() {
                    TranslateInput::File { path: self.text.clone(), out: None }
                } else {
                    TranslateInput::Text(self.text.clone())
                };
                Some(Job::new(JobKind::Translate(TranslateJob {
                    source_lang: self.src_lang.clone(),
                    target_lang: self.tgt_lang.clone(),
                    context: if self.context.is_empty() { None } else { Some(self.context.clone()) },
                    input,
                })))
            }
            NewJobType::Sentry => Some(Job::new(JobKind::SentryI18n(SentryJob {
                step: self.sentry_step,
                cache_bust: self.cache_bust,
                workers: 32,
                sources: None,
                limit: None,
            }))),
        }
    }
}

pub struct App {
    pub cfg: Arc<Config>,
    pub mode: Mode,
    pub focus: Focus,
    pub new_job: NewJobForm,
    pub gpu: GpuSnapshot,
    pub active: Vec<Job>,
    pub queue: VecDeque<Job>,
    pub history: Vec<Job>,
    pub log: VecDeque<String>,
    pub should_quit: bool,
}

impl App {
    pub fn new(cfg: Arc<Config>) -> Self {
        let new_job = NewJobForm::new(&cfg);
        Self {
            cfg,
            mode: Mode::Normal,
            focus: Focus::Jobs,
            new_job,
            gpu: GpuSnapshot::default(),
            active: Vec::new(),
            queue: VecDeque::new(),
            history: Vec::new(),
            log: VecDeque::new(),
            should_quit: false,
        }
    }

    pub fn push_log(&mut self, line: String) {
        self.log.push_back(line);
        if self.log.len() > 500 { self.log.pop_front(); }
    }
}
