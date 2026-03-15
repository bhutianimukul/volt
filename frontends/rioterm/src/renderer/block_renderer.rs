//! Block UI renderer — visual decorations for OSC 133 command blocks.

use crate::context::blocks::BlockManager;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

const BLOCK_HEADER_HEIGHT: f32 = 16.0;
const EXIT_BADGE_WIDTH: f32 = 20.0;

/// Render block decorations as objects to overlay on the terminal.
///
/// For each visible command block, this draws:
/// - A thin horizontal separator line above the block
/// - An exit code badge (green checkmark for 0, red X for non-zero, grey circle if still running)
/// - Command duration text (if the command has finished)
#[allow(clippy::too_many_arguments)]
pub fn render_block_decorations(
    sugarloaf: &mut Sugarloaf,
    block_manager: &BlockManager,
    scroll_offset: usize,
    visible_rows: usize,
    cell_height: f32,
    _cell_width: f32,
    scale: f32,
    margin_x: f32,
    margin_top: f32,
) -> Vec<Object> {
    let mut objects = Vec::new();

    for block in block_manager.blocks() {
        // Check if this block's header is visible
        let header_row = block.start_row;
        if header_row < scroll_offset || header_row >= scroll_offset + visible_rows {
            continue;
        }

        let relative_row = header_row - scroll_offset;
        let y = margin_top + relative_row as f32 * cell_height / scale;
        let x = margin_x;

        // Exit code color and badge symbol
        let (badge_color, badge_text) = match block.exit_code {
            Some(0) => ([0.3, 0.8, 0.3, 1.0], "\u{2713}"), // Green checkmark
            Some(_) => ([0.9, 0.3, 0.3, 1.0], "\u{2717}"), // Red X
            None => ([0.6, 0.6, 0.6, 0.5], "\u{25cb}"),    // Running indicator
        };

        // Block separator line (thin horizontal rule)
        objects.push(Object::Quad(Quad {
            position: [x, y - 1.0],
            color: [0.3, 0.3, 0.35, 0.5],
            size: [800.0, 1.0],
            ..Quad::default()
        }));

        // Exit code badge background
        objects.push(Object::Quad(Quad {
            position: [x, y],
            color: badge_color,
            size: [EXIT_BADGE_WIDTH, BLOCK_HEADER_HEIGHT],
            border_radius: [3.0, 3.0, 3.0, 3.0],
            ..Quad::default()
        }));

        // Badge text (checkmark or X)
        let badge_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&badge_rt, 10.0);
        let content = sugarloaf.content();
        content
            .sel(badge_rt)
            .clear()
            .new_line()
            .add_text(
                badge_text,
                FragmentStyle {
                    color: [1.0, 1.0, 1.0, 1.0],
                    ..FragmentStyle::default()
                },
            )
            .build();
        objects.push(Object::RichText(RichText {
            id: badge_rt,
            position: [x + 4.0, y],
            lines: None,
        }));

        // Duration text (if command has finished)
        if let Some(duration) = &block.duration {
            let dur_text = format_block_duration(duration);
            let dur_rt = sugarloaf.create_temp_rich_text();
            sugarloaf.set_rich_text_font_size(&dur_rt, 10.0);
            let content = sugarloaf.content();
            content
                .sel(dur_rt)
                .clear()
                .new_line()
                .add_text(
                    &dur_text,
                    FragmentStyle {
                        color: [0.6, 0.6, 0.6, 0.8],
                        ..FragmentStyle::default()
                    },
                )
                .build();
            objects.push(Object::RichText(RichText {
                id: dur_rt,
                position: [x + EXIT_BADGE_WIDTH + 8.0, y],
                lines: None,
            }));
        }
    }

    objects
}

fn format_block_duration(duration: &std::time::Duration) -> String {
    let ms = duration.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m{}s", mins, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_millis() {
        assert_eq!(
            format_block_duration(&std::time::Duration::from_millis(500)),
            "500ms"
        );
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(
            format_block_duration(&std::time::Duration::from_millis(2500)),
            "2.5s"
        );
    }

    #[test]
    fn test_format_duration_minutes() {
        assert_eq!(
            format_block_duration(&std::time::Duration::from_millis(65000)),
            "1m5s"
        );
    }
}
