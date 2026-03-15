use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(sugarloaf: &mut Sugarloaf, context_dimension: &ContextDimension) {
    let accent = [0.9882353, 0.7294118, 0.15686275, 1.0]; // yellow (Volt brand)
    let dim = [0.5, 0.5, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let key_color = [0.4, 0.8, 1.0, 1.0]; // light blue for keys
    let success_color = [0.3, 0.85, 0.4, 1.0]; // green for exit code 0
    let fail_color = [0.9, 0.3, 0.3, 1.0]; // red for non-zero exit codes
    let tag_color = [0.7, 0.5, 1.0, 1.0]; // purple for tags

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
            "Bookmarks",
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

    let content = sugarloaf.content();
    let body = content.sel(body_rt);
    body.clear();

    let store = crate::bookmarks::BookmarkStore::load();
    let bookmarks = store.list();

    if bookmarks.is_empty() {
        body.new_line().add_text("  No bookmarks saved yet.", dim_style);
    } else {
        body.new_line()
            .add_text("SAVED COMMANDS", header_style)
            .new_line();

        for bm in &bookmarks {
            // Command
            body.add_text("  ", dim_style);
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

    // Footer
    body.new_line().new_line();
    body.add_text("  Press ", dim_style)
        .add_text("Escape", key_style)
        .add_text(" to close", dim_style);

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 70.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}
