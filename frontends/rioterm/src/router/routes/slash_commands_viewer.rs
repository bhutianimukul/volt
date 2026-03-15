use crate::context::grid::ContextDimension;
use crate::slash_commands::{all_commands, CommandCategory};
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(sugarloaf: &mut Sugarloaf, context_dimension: &ContextDimension, scroll_offset: usize) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.98, 0.73, 0.16, 1.0]; // yellow/gold (Volt brand)
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let key_color = [0.4, 0.8, 1.0, 1.0];

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

    let mut line_idx: usize = 0;
    for category in &categories {
        if line_idx > scroll_offset {
            body.new_line()
                .add_text(&category.name().to_uppercase(), header_style)
                .new_line();
        }
        line_idx += 1;

        for cmd in commands.iter().filter(|c| c.category == *category) {
            if line_idx > scroll_offset {
                body.add_text(&format!("  /{}", cmd.name), cmd_style);

                // Padding dots
                let pad_len = 16usize.saturating_sub(cmd.name.len() + 1);
                let dots: String = " .".repeat(pad_len / 2);
                body.add_text(&dots, dim_style);
                body.add_text(" ", dim_style);

                body.add_text(cmd.description, desc_style).new_line();
                body.add_text(&format!("    {}", cmd.usage), dim_style)
                    .new_line();
            }
            line_idx += 1;
        }
    }

    // Footer
    body.new_line().new_line();
    body.add_text(" \u{2191}\u{2193} ", key_bg_style)
        .add_text(" scroll  ", dim_style)
        .add_text(" Escape ", key_bg_style)
        .add_text(" close", dim_style);

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 85.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}
