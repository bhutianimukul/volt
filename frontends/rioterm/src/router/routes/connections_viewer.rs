use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

/// Render the connections viewer screen.
/// `connections` is a list of (name, type_name, host_info, command).
#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    connections: &[(String, String, String, String)],
    selected_index: usize,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.2, 0.7, 0.5, 1.0]; // teal
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0]; // yellow
    let selected_bg = [0.12, 0.18, 0.15, 1.0];

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
            "Connections",
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
    let count = connections.len();
    let subtitle_text = if count == 0 {
        "No connections configured".to_string()
    } else {
        format!(
            "{} connection{}  |  ~/.config/volt/connections.toml",
            count,
            if count == 1 { "" } else { "s" }
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

    // Body: connection list
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 13.0);

    let key_bg_style = FragmentStyle {
        background_color: Some(highlight),
        color: black,
        ..FragmentStyle::default()
    };

    let content = sugarloaf.content();
    let body = content.sel(body_rt).clear();

    if connections.is_empty() {
        body.add_text(
            "No connections configured yet.",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        )
        .new_line()
        .new_line();
        body.add_text(
            "Config file: ",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
        body.add_text(
            "~/.config/volt/connections.toml",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        )
        .new_line()
        .new_line();
        body.add_text(
            "Add a connection by putting this in the file:",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        )
        .new_line()
        .new_line();
        body.add_text(
            "  [connections.my-server]",
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        )
        .new_line();
        body.add_text(
            "  type = \"ssh\"",
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        )
        .new_line();
        body.add_text(
            "  host = \"example.com\"",
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        )
        .new_line();
        body.add_text(
            "  user = \"deploy\"",
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        )
        .new_line()
        .new_line();
        body.add_text(
            "Supported types: ssh, mysql, postgres, redis, kubectl, docker",
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
        );
        body.add_text(
            " e ",
            FragmentStyle {
                background_color: Some(highlight),
                color: black,
                ..FragmentStyle::default()
            },
        );
        body.add_text(
            " to open the config file in your editor.",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
    } else {
        body.add_text(
            "SAVED CONNECTIONS",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        )
        .new_line();

        for (i, (name, type_name, host_info, _command)) in connections.iter().enumerate() {
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

            // Connection name
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

            let padded_name = format!("{:<20}", name);
            body.add_text(&padded_name, name_style);

            // Type badge
            body.add_text(
                &format!(" [{}] ", type_name),
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            );

            // Host info
            body.add_text(
                host_info,
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );

            body.new_line();
        }
    }

    // Footer
    body.new_line().new_line();
    body.add_text(" \u{2191}\u{2193} ", key_bg_style)
        .add_text(" navigate  ", dim_style())
        .add_text(" Enter ", key_bg_style)
        .add_text(" connect  ", dim_style())
        .add_text(" e ", key_bg_style)
        .add_text(" edit config  ", dim_style())
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
