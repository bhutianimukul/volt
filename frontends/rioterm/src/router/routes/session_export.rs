use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

/// Export format options
pub const EXPORT_FORMATS: &[(&str, &str, &str)] = &[
    ("Asciinema", ".cast", "Playable recording for asciinema.org"),
    ("Plain Text", ".txt", "Command history as plain text"),
    ("HTML", ".html", "Styled terminal output as HTML page"),
    ("JSON", ".json", "Structured session data"),
];

/// Result of the last export attempt
#[derive(Debug, Clone, Default)]
pub struct ExportResult {
    pub message: String,
    pub success: bool,
}

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    selected_format: usize,
    last_result: &Option<ExportResult>,
    command_count: usize,
) {
    let bg = [0.07, 0.07, 0.07, 1.0];
    let accent = [0.5, 0.8, 0.3, 1.0]; // green for export
    let dim = [0.45, 0.45, 0.5, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let green = [0.3, 0.85, 0.4, 1.0];
    let red = [0.9, 0.3, 0.3, 1.0];
    let card_bg = [0.1, 0.1, 0.14, 1.0];
    let card_border = [0.2, 0.3, 0.2, 0.8];
    let sidebar_bg = [0.09, 0.09, 0.09, 1.0];

    let layout = sugarloaf.window_size();
    let scale = context_dimension.dimension.scale;
    let full_w = layout.width / scale;
    let full_h = layout.height / scale;
    let mut objects = Vec::with_capacity(32);

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
    sugarloaf
        .content()
        .sel(title_rt)
        .clear()
        .add_text(
            "Export Session",
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
    sugarloaf
        .content()
        .sel(subtitle_rt)
        .clear()
        .add_text(
            &format!(
                "{} commands recorded  |  Select format and press Enter",
                command_count
            ),
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

    // Format cards — vertical list
    let card_start_y = context_dimension.margin.top_y + 85.0;
    let card_h = 50.0_f32;
    let card_gap = 8.0_f32;
    let card_w = (full_w - 80.0).min(450.0);

    for (i, (name, ext, desc)) in EXPORT_FORMATS.iter().enumerate() {
        let cy = card_start_y + i as f32 * (card_h + card_gap);
        let is_selected = i == selected_format;

        let bg_c = if is_selected {
            [0.12, 0.15, 0.12, 1.0]
        } else {
            card_bg
        };
        objects.push(Object::Quad(Quad {
            position: [40., cy],
            color: bg_c,
            size: [card_w, card_h],
            border_radius: [4.0; 4],
            border_color: if is_selected { accent } else { card_border },
            border_width: if is_selected { 2.0 } else { 1.0 },
            ..Quad::default()
        }));

        let row_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&row_rt, 12.0);
        let rc = sugarloaf.content().sel(row_rt);
        rc.clear();

        if is_selected {
            rc.add_text(
                "> ",
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            );
        } else {
            rc.add_text(
                "  ",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
        }

        rc.add_text(
            name,
            FragmentStyle {
                color: if is_selected { white } else { white },
                ..FragmentStyle::default()
            },
        );
        rc.add_text(
            &format!("  ({})", ext),
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        );
        rc.new_line();
        rc.add_text(
            &format!("    {}", desc),
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );

        rc.build();
        objects.push(Object::RichText(RichText {
            id: row_rt,
            position: [48., cy + 6.0],
            lines: None,
        }));
    }

    // Export destination info
    let info_y = card_start_y + EXPORT_FORMATS.len() as f32 * (card_h + card_gap) + 10.0;
    let info_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&info_rt, 11.0);
    let ic = sugarloaf.content().sel(info_rt);
    ic.clear();
    ic.add_text(
        "Export saves to ~/Desktop/volt-session-<timestamp>",
        FragmentStyle {
            color: dim,
            ..FragmentStyle::default()
        },
    );
    ic.build();
    objects.push(Object::RichText(RichText {
        id: info_rt,
        position: [40., info_y],
        lines: None,
    }));

    // Last export result
    if let Some(result) = last_result {
        let result_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&result_rt, 12.0);
        let rc_result = sugarloaf.content().sel(result_rt);
        rc_result.clear();
        if result.success {
            rc_result.add_text(
                &format!("\u{2713} {}", result.message),
                FragmentStyle {
                    color: green,
                    ..FragmentStyle::default()
                },
            );
        } else {
            rc_result.add_text(
                &format!("\u{2717} {}", result.message),
                FragmentStyle {
                    color: red,
                    ..FragmentStyle::default()
                },
            );
        }
        rc_result.build();
        objects.push(Object::RichText(RichText {
            id: result_rt,
            position: [40., info_y + 20.0],
            lines: None,
        }));
    }

    // Footer
    let footer_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&footer_rt, 11.0);
    let key_bg = FragmentStyle {
        color: black,
        background_color: Some(highlight),
        ..FragmentStyle::default()
    };
    let dim_s = FragmentStyle {
        color: dim,
        ..FragmentStyle::default()
    };
    let fc = sugarloaf.content().sel(footer_rt);
    fc.clear().new_line();
    fc.add_text(" \u{2191}\u{2193} ", key_bg)
        .add_text(" select  ", dim_s);
    fc.add_text(" Enter ", key_bg).add_text(" export  ", dim_s);
    fc.add_text(" Esc ", key_bg).add_text(" close", dim_s);
    fc.build();

    let footer_y = full_h - 28.0;
    objects.push(Object::Quad(Quad {
        position: [0., footer_y - 4.0],
        color: sidebar_bg,
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
