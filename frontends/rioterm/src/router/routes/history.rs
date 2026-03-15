use crate::context::grid::ContextDimension;
use crate::time_travel::SessionRecorder;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    recorder: &SessionRecorder,
) {
    let accent = [0.9882353, 0.7294118, 0.15686275, 1.0]; // yellow (Volt brand)
    let dim = [0.5, 0.5, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let green = [0.3, 0.9, 0.3, 1.0];
    let red = [0.9, 0.3, 0.3, 1.0];

    let layout = sugarloaf.window_size();
    let mut objects = Vec::with_capacity(16);

    // Full-screen black background
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: black,
        size: [
            layout.width / context_dimension.dimension.scale,
            layout.height,
        ],
        ..Quad::default()
    }));

    // Yellow accent bar on the left
    objects.push(Object::Quad(Quad {
        position: [0., 30.0],
        color: accent,
        size: [4., layout.height],
        ..Quad::default()
    }));

    // Title
    let title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&title_rt, 24.0);
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
        position: [40., context_dimension.margin.top_y + 30.],
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

    let content = sugarloaf.content();
    let body = content.sel(body_rt);
    body.clear();

    let entries = recorder.recent(50);

    if entries.is_empty() {
        body.new_line()
            .add_text("  No commands recorded yet.", dim_style)
            .new_line();
        body.new_line()
            .add_text(
                "  Commands will appear here as you use the terminal.",
                dim_style,
            )
            .new_line();
    } else {
        body.new_line()
            .add_text(
                &format!("RECENT COMMANDS ({})", entries.len()),
                header_style,
            )
            .new_line();
        body.new_line();

        for entry in entries.iter().rev() {
            // Status indicator
            let (status, style) = match entry.exit_code {
                Some(0) => (" ok ", ok_style),
                Some(code) => {
                    // We can't easily format dynamic strings into a static style,
                    // so just show a generic fail marker
                    let _ = code;
                    ("FAIL", fail_style)
                }
                None => (" .. ", dim_style),
            };

            body.add_text("  [", dim_style);
            body.add_text(status, style);
            body.add_text("]  ", dim_style);

            // Command text (or placeholder)
            let cmd_display = if entry.command.is_empty() {
                "(command)"
            } else {
                &entry.command
            };
            body.add_text(cmd_display, cmd_style);

            // Duration if available
            if let Some(ms) = entry.duration_ms {
                if ms < 1000 {
                    body.add_text(&format!("  {}ms", ms), dim_style);
                } else {
                    body.add_text(&format!("  {:.1}s", ms as f64 / 1000.0), dim_style);
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
    body.add_text(
        "  Press ",
        dim_style,
    );
    body.add_text(
        "Escape",
        FragmentStyle {
            color: [0.4, 0.8, 1.0, 1.0],
            ..FragmentStyle::default()
        },
    );
    body.add_text(" to close", dim_style);

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 70.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}
