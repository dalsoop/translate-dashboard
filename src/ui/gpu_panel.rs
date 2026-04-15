use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(" GPU ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let gpus = &app.gpu.gpus;
    if gpus.is_empty() {
        let msg = app.gpu.error.clone().unwrap_or_else(|| "폴링 대기…".into());
        f.render_widget(Paragraph::new(msg).style(Style::default().fg(Color::DarkGray)), inner);
        return;
    }

    let row_h = 2u16;
    for (i, g) in gpus.iter().enumerate() {
        let y = inner.y + i as u16 * row_h;
        if y + 1 >= inner.y + inner.height { break; }
        let label_rect = Rect { x: inner.x, y, width: inner.width, height: 1 };
        let gauge_rect = Rect { x: inner.x, y: y + 1, width: inner.width, height: 1 };

        let label = Line::from(vec![
            Span::styled(format!("GPU{} ", g.index), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:>3}%  ", g.util_pct)),
            Span::raw(format!("{:>5}/{} MiB  ", g.mem_used_mib, g.mem_total_mib)),
            Span::raw(format!("{}°C", g.temp_c)),
        ]);
        f.render_widget(Paragraph::new(label), label_rect);

        let mem_ratio = if g.mem_total_mib > 0 {
            g.mem_used_mib as f64 / g.mem_total_mib as f64
        } else { 0.0 };
        let color = if g.util_pct > 80 { Color::Red }
                    else if g.util_pct > 30 { Color::Yellow }
                    else { Color::Green };
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(color).bg(Color::DarkGray))
            .ratio(mem_ratio.min(1.0).max(0.0))
            .label(format!("mem {:>3.0}% util {}%", mem_ratio * 100.0, g.util_pct));
        f.render_widget(gauge, gauge_rect);
    }
}
