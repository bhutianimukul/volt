use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    sessions: &[(String, String, bool)],
    selected_index: usize,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.3, 0.85, 0.4, 1.0]; // green
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0]; // yellow
    let selected_bg = [0.15, 0.2, 0.15, 1.0];
    let attached_color = [0.4, 0.8, 1.0, 1.0];

    let layout = sugarloaf.window_size();
    let mut objects = Vec::with_capacity(16);

    // Background
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: bg,
        size: [
            layout.width / context_dimension.dimension.scale,
            layout.height,
        ],
        ..Quad::default()
    }));

    // Accent bar on the left
    objects.push(Object::Quad(Quad {
        position: [0., 30.0],
        color: accent,
        size: [4., layout.height],
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
            "tmux Sessions",
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
    let session_count = sessions.len();
    let subtitle_text = if session_count == 0 {
        "No active sessions".to_string()
    } else {
        format!(
            "{} session{}  |  Select to attach or create new",
            session_count,
            if session_count == 1 { "" } else { "s" }
        )
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

    // Body: session list
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 13.0);

    let key_bg_style = FragmentStyle {
        background_color: Some(highlight),
        color: black,
        ..FragmentStyle::default()
    };

    let content = sugarloaf.content();
    let body = content.sel(body_rt).clear();

    if sessions.is_empty() {
        body.add_text(
            "No tmux sessions found.",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        )
        .new_line()
        .new_line();
        body.add_text(
            "Press ",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        )
        .add_text(
            "n",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        )
        .add_text(
            " to create a new session.",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
    } else {
        body.add_text(
            "SESSIONS",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        )
        .new_line();

        for (i, (_id, name, attached)) in sessions.iter().enumerate() {
            let is_selected = i == selected_index;

            // Selection indicator
            if is_selected {
                body.add_text(
                    " > ",
                    FragmentStyle {
                        color: highlight,
                        ..FragmentStyle::default()
                    },
                );
            } else {
                body.add_text(
                    "   ",
                    FragmentStyle {
                        color: dim,
                        ..FragmentStyle::default()
                    },
                );
            }

            // Session name
            let name_style = if is_selected {
                FragmentStyle {
                    background_color: Some(selected_bg),
                    color: white,
                    ..FragmentStyle::default()
                }
            } else {
                FragmentStyle {
                    color: white,
                    ..FragmentStyle::default()
                }
            };

            let padded_name = format!("{:<24}", name);
            body.add_text(&padded_name, name_style);

            // Attached status
            if *attached {
                body.add_text(
                    " (attached)",
                    FragmentStyle {
                        color: attached_color,
                        ..FragmentStyle::default()
                    },
                );
            }

            body.new_line();
        }
    }

    // Footer
    body.new_line().new_line();
    body.add_text(" \u{2191}\u{2193} ", key_bg_style)
        .add_text(" navigate ", dim_style())
        .add_text(" Enter ", key_bg_style)
        .add_text(" attach ", dim_style())
        .add_text(" n ", key_bg_style)
        .add_text(" new ", dim_style())
        .add_text(" d ", key_bg_style)
        .add_text(" detach ", dim_style())
        .add_text(" x ", key_bg_style)
        .add_text(" kill ", dim_style())
        .add_text(" r ", key_bg_style)
        .add_text(" rename ", dim_style())
        .add_text(" Esc ", key_bg_style)
        .add_text(" close", dim_style());

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 85.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}

fn dim_style() -> FragmentStyle {
    FragmentStyle {
        color: [0.45, 0.45, 0.5, 1.0],
        ..FragmentStyle::default()
    }
}
