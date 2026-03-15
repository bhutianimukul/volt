use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    selected_index: usize,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let dim = [0.45, 0.45, 0.5, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let accent = [0.3, 0.7, 0.9, 1.0];
    let pane_bg = [0.10, 0.10, 0.14, 1.0];
    let pane_border = [0.25, 0.25, 0.3, 0.8];
    let pane_selected_bg = [0.12, 0.16, 0.25, 1.0];
    let pane_inner = [0.06, 0.06, 0.09, 1.0];
    let pane_inner_border = [0.2, 0.2, 0.28, 0.6];
    let sidebar_bg = [0.08, 0.08, 0.11, 1.0];

    let layout = sugarloaf.window_size();
    let scale = context_dimension.dimension.scale;
    let full_w = layout.width / scale;
    let full_h = layout.height / scale;
    let mut objects = Vec::with_capacity(48);

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
    sugarloaf
        .content()
        .sel(subtitle_rt)
        .clear()
        .add_text(
            "Choose a layout and press Enter to apply",
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

    // Layout cards — responsive sizing
    struct LayoutCard {
        name: &'static str,
        desc: &'static str,
        panes: Vec<(f32, f32, f32, f32)>,
    }

    let cards = vec![
        LayoutCard {
            name: "Side by Side",
            desc: "Two equal vertical panes",
            panes: vec![(0.0, 0.0, 0.48, 1.0), (0.52, 0.0, 0.48, 1.0)],
        },
        LayoutCard {
            name: "Dev",
            desc: "Editor left, terminals right",
            panes: vec![
                (0.0, 0.0, 0.55, 1.0),
                (0.58, 0.0, 0.42, 0.48),
                (0.58, 0.52, 0.42, 0.48),
            ],
        },
        LayoutCard {
            name: "Quad",
            desc: "Four equal panes",
            panes: vec![
                (0.0, 0.0, 0.48, 0.48),
                (0.52, 0.0, 0.48, 0.48),
                (0.0, 0.52, 0.48, 0.48),
                (0.52, 0.52, 0.48, 0.48),
            ],
        },
        LayoutCard {
            name: "Monitoring",
            desc: "Wide main + two stacked",
            panes: vec![
                (0.0, 0.0, 0.6, 1.0),
                (0.63, 0.0, 0.37, 0.48),
                (0.63, 0.52, 0.37, 0.48),
            ],
        },
    ];

    // Calculate card size based on window width — fit 2 columns with margins
    let margin_x = 40.0_f32;
    let card_gap = 24.0_f32;
    let available_w = full_w - margin_x * 2.0;
    let card_w = ((available_w - card_gap) / 2.0).min(280.0);
    let card_h = 130.0_f32;
    let row_gap = 30.0_f32;
    let inner_pad = 10.0_f32;

    // Center the grid horizontally
    let grid_total_w = card_w * 2.0 + card_gap;
    let grid_x = ((full_w - grid_total_w) / 2.0).max(margin_x);
    let grid_y = context_dimension.margin.top_y + 80.0;

    for (i, card) in cards.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let cx = grid_x + col as f32 * (card_w + card_gap);
        let cy = grid_y + row as f32 * (card_h + row_gap);
        let is_selected = i == selected_index;

        // Card background
        let card_bg_color = if is_selected {
            pane_selected_bg
        } else {
            pane_bg
        };
        objects.push(Object::Quad(Quad {
            position: [cx, cy],
            color: card_bg_color,
            size: [card_w, card_h],
            border_radius: [8.0; 4],
            border_color: if is_selected { highlight } else { pane_border },
            border_width: if is_selected { 2.0 } else { 1.0 },
            ..Quad::default()
        }));

        // Pane preview area (leave room for label below)
        let preview_x = cx + inner_pad;
        let preview_y = cy + inner_pad;
        let preview_w = card_w - inner_pad * 2.0;
        let preview_h = card_h - inner_pad * 2.0 - 30.0;

        for pane in &card.panes {
            let px = preview_x + pane.0 * preview_w;
            let py = preview_y + pane.1 * preview_h;
            let pw = pane.2 * preview_w;
            let ph = pane.3 * preview_h;

            let inner_color = if is_selected {
                [0.08, 0.1, 0.15, 1.0]
            } else {
                pane_inner
            };
            let inner_border = if is_selected {
                [0.3, 0.4, 0.55, 0.7]
            } else {
                pane_inner_border
            };

            objects.push(Object::Quad(Quad {
                position: [px, py],
                color: inner_color,
                size: [pw, ph],
                border_radius: [4.0; 4],
                border_color: inner_border,
                border_width: 1.0,
                ..Quad::default()
            }));
        }

        // Label below the preview
        let label_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&label_rt, 12.0);
        let lc = sugarloaf.content().sel(label_rt);
        lc.clear();

        let name_color = if is_selected { highlight } else { white };
        lc.add_text(
            card.name,
            FragmentStyle {
                color: name_color,
                ..FragmentStyle::default()
            },
        );
        lc.new_line();
        lc.add_text(
            card.desc,
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
        lc.build();
        objects.push(Object::RichText(RichText {
            id: label_rt,
            position: [cx + inner_pad, cy + card_h - 28.0],
            lines: None,
        }));

        // Selection indicator arrow
        if is_selected {
            let indicator_rt = sugarloaf.create_temp_rich_text();
            sugarloaf.set_rich_text_font_size(&indicator_rt, 11.0);
            sugarloaf
                .content()
                .sel(indicator_rt)
                .clear()
                .add_text(
                    "Press Enter to apply",
                    FragmentStyle {
                        color: accent,
                        ..FragmentStyle::default()
                    },
                )
                .build();
            objects.push(Object::RichText(RichText {
                id: indicator_rt,
                position: [cx + inner_pad, cy + card_h + 4.0],
                lines: None,
            }));
        }
    }

    // Footer — pinned to bottom
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
    fc.add_text(" \u{2190}\u{2191}\u{2193}\u{2192} ", key_bg)
        .add_text(" select  ", dim_s);
    fc.add_text(" Enter ", key_bg).add_text(" apply  ", dim_s);
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
        position: [grid_x, footer_y + 4.0],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}
