use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    scroll_offset: usize,
    selected_index: usize,
    bookmarks: &[crate::bookmarks::Bookmark],
) {
    let bg = [0.07, 0.07, 0.07, 1.0];
    let accent = [0.9, 0.5, 0.3, 1.0]; // orange
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0]; // yellow for key badges
    let key_color = [0.4, 0.8, 1.0, 1.0]; // light blue for labels
    let success_color = [0.3, 0.85, 0.4, 1.0];
    let fail_color = [0.9, 0.3, 0.3, 1.0];
    let tag_color = [0.7, 0.5, 1.0, 1.0]; // purple for tags

    let layout = sugarloaf.window_size();
    let scale = context_dimension.dimension.scale;
    let full_w = layout.width / scale;
    let full_h = layout.height / scale;
    let mut objects = Vec::with_capacity(16);

    // Background
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: bg,
        size: [full_w, full_h],
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
            "Bookmarks",
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
    let subtitle_text = if bookmarks.is_empty() {
        "No bookmarks saved yet".to_string()
    } else {
        format!(
            "{} saved command{}",
            bookmarks.len(),
            if bookmarks.len() == 1 { "" } else { "s" }
        )
    };
    let content = sugarloaf.content();
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

    // Body with bookmarks
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 13.0);

    let key_style = FragmentStyle {
        color: key_color,
        ..FragmentStyle::default()
    };
    let cmd_style = FragmentStyle {
        color: white,
        ..FragmentStyle::default()
    };
    let header_style = FragmentStyle {
        color: accent,
        ..FragmentStyle::default()
    };
    let dim_style = FragmentStyle {
        color: dim,
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

    if bookmarks.is_empty() {
        body.add_text("No bookmarks yet.", dim_style)
            .new_line()
            .new_line();
        body.add_text(
            "To bookmark a command, right-click on a command block",
            dim_style,
        )
        .new_line();
        body.add_text(
            "and select \"Bookmark\", or use the bookmark API from",
            dim_style,
        )
        .new_line();
        body.add_text("shell integration hooks.", dim_style)
            .new_line();
        body.new_line();
        body.add_text("Open with ", dim_style)
            .add_text(
                "Cmd+Shift+K",
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            )
            .add_text(" anytime.", dim_style)
            .new_line();
    } else {
        body.add_text("SAVED COMMANDS", header_style).new_line();

        let clamped_selected = if bookmarks.is_empty() {
            0
        } else {
            selected_index.min(bookmarks.len() - 1)
        };

        let mut line_idx: usize = 0;
        for (i, bm) in bookmarks.iter().enumerate() {
            line_idx += 1;
            if line_idx <= scroll_offset {
                continue;
            }

            let is_selected = i == clamped_selected;

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
                body.add_text("   ", dim_style);
            }

            let label = bm
                .name
                .as_deref()
                .map(|n| format!("[{}] ", n))
                .unwrap_or_default();
            if !label.is_empty() {
                body.add_text(&label, key_style);
            }
            let cmd_display = if bm.command.len() > 60 {
                format!("{}...", &bm.command[..57])
            } else {
                bm.command.clone()
            };
            body.add_text(&cmd_display, cmd_style);

            // Exit code
            if let Some(code) = bm.exit_code {
                let code_style = if code == 0 {
                    FragmentStyle {
                        color: success_color,
                        ..FragmentStyle::default()
                    }
                } else {
                    FragmentStyle {
                        color: fail_color,
                        ..FragmentStyle::default()
                    }
                };
                body.add_text(&format!("  exit:{}", code), code_style);
            }

            // Tags
            if !bm.tags.is_empty() {
                let tag_style = FragmentStyle {
                    color: tag_color,
                    ..FragmentStyle::default()
                };
                let tags_str = format!("  [{}]", bm.tags.join(", "));
                body.add_text(&tags_str, tag_style);
            }

            body.new_line();
        }
    }

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 85.],
        lines: None,
    }));

    // Footer — pinned to bottom
    let footer_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&footer_rt, 11.0);
    let fc = sugarloaf.content().sel(footer_rt);
    fc.clear().new_line();
    fc.add_text(" \u{2191}\u{2193} ", key_bg_style)
        .add_text(" navigate  ", footer_dim_style())
        .add_text(" Enter ", key_bg_style)
        .add_text(" paste  ", footer_dim_style())
        .add_text(" d ", key_bg_style)
        .add_text(" delete  ", footer_dim_style())
        .add_text(" Escape ", key_bg_style)
        .add_text(" close", footer_dim_style());
    fc.build();

    let footer_y = full_h - 28.0;
    objects.push(Object::Quad(Quad {
        position: [0., footer_y - 4.0],
        color: [0.07, 0.07, 0.07, 1.0],
        size: [full_w, 28.0],
        ..Quad::default()
    }));
    objects.push(Object::RichText(RichText {
        id: footer_rt,
        position: [40., footer_y + 4.0],
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
