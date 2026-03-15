use crate::context::grid::ContextDimension;
use crate::time_travel::SessionRecorder;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    recorder: &SessionRecorder,
    scroll_offset: usize,
    selected_index: usize,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.7, 0.4, 0.9, 1.0]; // purple
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0]; // yellow for key badges
    let green = [0.3, 0.9, 0.3, 1.0];
    let red = [0.9, 0.3, 0.3, 1.0];
    let selected_bg = [0.15, 0.15, 0.25, 1.0];

    let layout = sugarloaf.window_size();
    let mut objects = Vec::with_capacity(16);

    // Background
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: bg,
        size: [
            layout.width / context_dimension.dimension.scale,
            layout.height / context_dimension.dimension.scale,
        ],
        ..Quad::default()
    }));

    // Accent bar on the left
    objects.push(Object::Quad(Quad {
        position: [0., 30.0],
        color: accent,
        size: [4., layout.height / context_dimension.dimension.scale],
        ..Quad::default()
    }));

    // Title
    let title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&title_rt, 22.0);
    let content = sugarloaf.content();
    content
        .sel(title_rt)
        .clear()
        .add_text(
            "Session History",
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        )
        .build();
    objects.push(Object::RichText(RichText {
        id: title_rt,
        position: [40., context_dimension.margin.top_y + 25.],
        lines: None,
    }));

    // Subtitle
    let subtitle_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&subtitle_rt, 13.0);
    let content = sugarloaf.content();
    let entry_count = recorder.len();
    let subtitle_text = if entry_count == 0 {
        "No commands recorded yet".to_string()
    } else {
        format!("{} commands recorded this session", entry_count)
    };
    content
        .sel(subtitle_rt)
        .clear()
        .add_text(
            &subtitle_text,
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        )
        .build();
    objects.push(Object::RichText(RichText {
        id: subtitle_rt,
        position: [40., context_dimension.margin.top_y + 55.],
        lines: None,
    }));

    // Body with recent commands
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 13.0);

    let header_style = FragmentStyle {
        color: accent,
        ..FragmentStyle::default()
    };
    let cmd_style = FragmentStyle {
        color: white,
        ..FragmentStyle::default()
    };
    let dim_style = FragmentStyle {
        color: dim,
        ..FragmentStyle::default()
    };
    let ok_style = FragmentStyle {
        color: green,
        ..FragmentStyle::default()
    };
    let fail_style = FragmentStyle {
        color: red,
        ..FragmentStyle::default()
    };
    let key_bg_style = FragmentStyle {
        background_color: Some(highlight),
        color: black,
        ..FragmentStyle::default()
    };

    let content = sugarloaf.content();
    let body = content.sel(body_rt);
    body.clear();

    let entries = recorder.recent(50);

    if entries.is_empty() {
        body.add_text("No commands recorded yet.", dim_style)
            .new_line()
            .new_line();
        body.add_text(
            "Commands will appear here as you use the terminal.",
            dim_style,
        )
        .new_line()
        .new_line();
        body.add_text(
            "Volt records commands automatically via shell integration.",
            dim_style,
        )
        .new_line();
        body.add_text(
            "If commands are not appearing, ensure your shell is",
            dim_style,
        )
        .new_line();
        body.add_text(
            "configured with Volt's shell integration hooks.",
            dim_style,
        )
        .new_line();
    } else {
        body.add_text(
            &format!("RECENT COMMANDS ({})", entries.len()),
            header_style,
        )
        .new_line();

        // Clamp selected_index to valid range
        let entry_count = entries.len();
        let clamped_selected = if entry_count > 0 {
            selected_index.min(entry_count - 1)
        } else {
            0
        };

        let mut line_idx: usize = 0;
        for entry in entries.iter().rev() {
            let display_idx = line_idx;
            line_idx += 1;
            if line_idx <= scroll_offset {
                continue;
            }

            let is_selected = display_idx == clamped_selected;

            // Selection indicator
            if is_selected {
                body.add_text(" > ", FragmentStyle {
                    color: highlight,
                    ..FragmentStyle::default()
                });
            } else {
                body.add_text("   ", dim_style);
            }

            let row_cmd_style = if is_selected {
                FragmentStyle {
                    background_color: Some(selected_bg),
                    color: white,
                    ..FragmentStyle::default()
                }
            } else {
                cmd_style
            };

            // Status indicator
            let (status, style) = match entry.exit_code {
                Some(0) => (" ok ", ok_style),
                Some(_code) => ("FAIL", fail_style),
                None => (" .. ", dim_style),
            };

            body.add_text("[", dim_style);
            body.add_text(status, style);
            body.add_text("]  ", dim_style);

            // Command text
            let cmd_display = if entry.command.is_empty() {
                "(no command text captured)"
            } else {
                &entry.command
            };
            // Truncate very long commands for display
            let cmd_truncated = if cmd_display.len() > 80 {
                format!("{}...", &cmd_display[..77])
            } else {
                cmd_display.to_string()
            };
            body.add_text(&cmd_truncated, row_cmd_style);

            // Duration if available
            if let Some(ms) = entry.duration_ms {
                if ms < 1000 {
                    body.add_text(&format!("  {}ms", ms), dim_style);
                } else {
                    body.add_text(
                        &format!("  {:.1}s", ms as f64 / 1000.0),
                        dim_style,
                    );
                }
            }

            // Timestamp
            if let Ok(elapsed) = entry.timestamp.elapsed() {
                let secs = elapsed.as_secs();
                let ago = if secs < 60 {
                    format!("{}s ago", secs)
                } else if secs < 3600 {
                    format!("{}m ago", secs / 60)
                } else {
                    format!("{}h ago", secs / 3600)
                };
                body.add_text(&format!("  ({})", ago), dim_style);
            }

            body.new_line();
        }
    }

    // Footer
    body.new_line().new_line();
    body.add_text(" \u{2191}\u{2193} ", key_bg_style)
        .add_text(" navigate  ", footer_dim_style())
        .add_text(" Enter ", key_bg_style)
        .add_text(" paste  ", footer_dim_style())
        .add_text(" b ", key_bg_style)
        .add_text(" bookmark  ", footer_dim_style())
        .add_text(" e ", key_bg_style)
        .add_text(" export  ", footer_dim_style())
        .add_text(" Escape ", key_bg_style)
        .add_text(" close", footer_dim_style());

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 85.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}

fn footer_dim_style() -> FragmentStyle {
    FragmentStyle {
        color: [0.45, 0.45, 0.5, 1.0],
        ..FragmentStyle::default()
    }
}
