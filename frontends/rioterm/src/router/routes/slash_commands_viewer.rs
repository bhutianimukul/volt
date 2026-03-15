use crate::context::grid::ContextDimension;
use crate::slash_commands::{all_commands, CommandCategory};
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    scroll_offset: usize,
    selected_index: usize,
) {
    let bg = [0.07, 0.07, 0.07, 1.0];
    let accent = [0.98, 0.73, 0.16, 1.0]; // yellow/gold (Volt brand)
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let key_color = [0.4, 0.8, 1.0, 1.0];

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
            "Slash Commands",
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
    content
        .sel(subtitle_rt)
        .clear()
        .add_text(
            "Type / at the prompt to use these commands",
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

    // Body with all slash commands grouped by category
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 13.0);

    let cmd_style = FragmentStyle {
        color: key_color,
        ..FragmentStyle::default()
    };
    let desc_style = FragmentStyle {
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

    let commands = all_commands();
    let categories = [
        CommandCategory::Navigation,
        CommandCategory::Appearance,
        CommandCategory::Tools,
        CommandCategory::Session,
        CommandCategory::Debug,
    ];

    let selected_style = FragmentStyle {
        color: white,
        ..FragmentStyle::default()
    };
    let selected_cmd_style = FragmentStyle {
        color: [0.5, 0.9, 1.0, 1.0],
        ..FragmentStyle::default()
    };

    let mut line_idx: usize = 0;
    let mut flat_cmd_idx: usize = 0;
    for category in &categories {
        if line_idx > scroll_offset {
            body.new_line()
                .add_text(&category.name().to_uppercase(), header_style)
                .new_line();
        }
        line_idx += 1;

        for cmd in commands.iter().filter(|c| c.category == *category) {
            let is_selected = flat_cmd_idx == selected_index;
            if line_idx > scroll_offset {
                if is_selected {
                    body.add_text(&format!("> /{}", cmd.name), selected_cmd_style);
                } else {
                    body.add_text(&format!("  /{}", cmd.name), cmd_style);
                }

                // Padding dots
                let pad_len = 16usize.saturating_sub(cmd.name.len() + 1);
                let dots: String = " .".repeat(pad_len / 2);
                body.add_text(&dots, dim_style);
                body.add_text(" ", dim_style);

                let d_style = if is_selected {
                    selected_style
                } else {
                    desc_style
                };
                body.add_text(cmd.description, d_style).new_line();
                body.add_text(&format!("    {}", cmd.usage), dim_style)
                    .new_line();
            }
            line_idx += 1;
            flat_cmd_idx += 1;
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
        .add_text(" navigate  ", dim_style)
        .add_text(" Enter ", key_bg_style)
        .add_text(" insert  ", dim_style)
        .add_text(" Escape ", key_bg_style)
        .add_text(" close", dim_style);
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
