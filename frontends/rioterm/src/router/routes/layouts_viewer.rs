use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

/// Built-in layout presets
pub struct LayoutPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
}

pub fn presets() -> Vec<LayoutPreset> {
    vec![
        LayoutPreset {
            name: "Side by Side",
            description: "Two panes split vertically — great for code + terminal",
            icon: "[|]",
        },
        LayoutPreset {
            name: "Dev",
            description: "Main editor pane on the left, two stacked panes on the right",
            icon: "[|=]",
        },
        LayoutPreset {
            name: "Quad",
            description: "Four equal panes in a 2x2 grid",
            icon: "[=|=]",
        },
        LayoutPreset {
            name: "Monitoring",
            description: "One large pane on top, three smaller panes on the bottom",
            icon: "[---/|||]",
        },
    ]
}

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    selected_index: usize,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.3, 0.7, 0.9, 1.0]; // blue accent
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let selected_bg = [0.15, 0.15, 0.25, 1.0];

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
            "Layout Presets",
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
            "Choose a layout to arrange your panes",
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

    // Body with layout presets
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 13.0);

    let name_style = FragmentStyle {
        color: accent,
        ..FragmentStyle::default()
    };
    let desc_style = FragmentStyle {
        color: white,
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

    let layout_presets = presets();
    let clamped_selected = if layout_presets.is_empty() {
        0
    } else {
        selected_index.min(layout_presets.len() - 1)
    };

    for (i, preset) in layout_presets.iter().enumerate() {
        let is_selected = i == clamped_selected;

        if is_selected {
            body.add_text(" > ", FragmentStyle {
                color: highlight,
                ..FragmentStyle::default()
            });
        } else {
            body.add_text("   ", dim_style);
        }

        let row_name_style = if is_selected {
            FragmentStyle {
                background_color: Some(selected_bg),
                color: accent,
                ..FragmentStyle::default()
            }
        } else {
            name_style
        };

        let row_desc_style = if is_selected {
            FragmentStyle {
                background_color: Some(selected_bg),
                color: white,
                ..FragmentStyle::default()
            }
        } else {
            desc_style
        };

        body.add_text(preset.icon, dim_style);
        body.add_text("  ", dim_style);
        body.add_text(preset.name, row_name_style);
        body.new_line();

        if is_selected {
            body.add_text("     ", dim_style);
        } else {
            body.add_text("     ", dim_style);
        }
        body.add_text("     ", dim_style);
        body.add_text(preset.description, row_desc_style);
        body.new_line().new_line();
    }

    // Footer
    body.new_line();
    body.add_text(" \u{2191}\u{2193} ", key_bg_style)
        .add_text(" navigate  ", dim_style)
        .add_text(" Enter ", key_bg_style)
        .add_text(" apply  ", dim_style)
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
