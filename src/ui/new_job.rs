use crate::app::{App, NewJobField, NewJobType};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, full: Rect, app: &App) {
    let w = full.width.min(80);
    let h = 18u16;
    let x = full.x + (full.width.saturating_sub(w)) / 2;
    let y = full.y + full.height.saturating_sub(h) / 2;
    let area = Rect { x, y, width: w, height: h };

    f.render_widget(Clear, area);
    let block = Block::default()
        .title(" 새 작업 추가 ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let form = &app.new_job;

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // type
            Constraint::Length(2), // src
            Constraint::Length(2), // tgt
            Constraint::Length(2), // input/step
            Constraint::Length(2), // context / misc
            Constraint::Min(1),    // help
        ])
        .split(inner);

    let typ_text = match form.job_type {
        NewJobType::Translate => "Translate  (< / > 로 변경)",
        NewJobType::Sentry => "Sentry i18n  (< / > 로 변경)",
    };
    field(f, rows[0], "종류  ", typ_text, form.focus == NewJobField::Type);

    field(f, rows[1], "src   ", &form.src_lang, form.focus == NewJobField::Src);
    field(f, rows[2], "tgt   ", &form.tgt_lang, form.focus == NewJobField::Tgt);

    let (label, value) = match form.job_type {
        NewJobType::Translate => ("text  ", form.text.as_str()),
        NewJobType::Sentry => ("step  ", form.sentry_step.as_str()),
    };
    field(f, rows[3], label, value, form.focus == NewJobField::Main);

    let (label, value) = match form.job_type {
        NewJobType::Translate => ("ctx   ", form.context.as_str()),
        NewJobType::Sentry => ("opts  ", if form.cache_bust { "--bust" } else { "" }),
    };
    field(f, rows[4], label, value, form.focus == NewJobField::Extra);

    let hint = Line::from(vec![Span::styled(
        "[Tab] 필드 이동   [Enter] 추가   [Esc] 취소",
        Style::default().fg(Color::DarkGray),
    )]);
    f.render_widget(Paragraph::new(hint).alignment(Alignment::Center), rows[5]);
}

fn field(f: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let style = if focused {
        Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    let line = Line::from(vec![
        Span::styled(format!(" {label} "), Style::default().fg(Color::Cyan)),
        Span::styled(format!(" {value} "), style),
    ]);
    f.render_widget(Paragraph::new(line), area);
}
