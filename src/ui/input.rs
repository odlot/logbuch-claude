use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub fn draw_input_line(frame: &mut Frame, area: Rect, label: &str, buffer: &str, cursor: usize) {
    let text = format!("{}{}", label, buffer);
    let input = Paragraph::new(text).style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, area);

    frame.set_cursor_position(Position::new(
        area.x + label.len() as u16 + cursor as u16,
        area.y,
    ));
}
