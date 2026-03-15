use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    selected_index: usize,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.3, 0.7, 0.9, 1.0];
    let dim = [0.45, 0.45, 0.5, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let pane_bg = [0.12, 0.12, 0.16, 1.0];
    let pane_border = [0.3, 0.3, 0.35, 1.0];
    let pane_selected = [0.15, 0.2, 0.3, 1.0];

    let layout = sugarloaf.window_size();
    let scale = context_dimension.dimension.scale;
    let mut objects = Vec::with_capacity(32);

    // Background
    objects.push(Object::Quad(Quad {
        position: [0., 0.0], color: bg,
        size: [layout.width / scale, layout.height / scale],
        ..Quad::default()
    }));

    // Title
    let title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&title_rt, 22.0);
    sugarloaf.content().sel(title_rt).clear()
        .add_text("Layout Presets", FragmentStyle { color: white, ..FragmentStyle::default() })
        .build();
    objects.push(Object::RichText(RichText {
        id: title_rt, position: [40., context_dimension.margin.top_y + 25.], lines: None,
    }));

    let subtitle_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&subtitle_rt, 13.0);
    sugarloaf.content().sel(subtitle_rt).clear()
        .add_text("Select a layout and press Enter to apply", FragmentStyle { color: dim, ..FragmentStyle::default() })
        .build();
    objects.push(Object::RichText(RichText {
        id: subtitle_rt, position: [40., context_dimension.margin.top_y + 55.], lines: None,
    }));

    // 2x2 grid of layout preview cards
    let card_w = 180.0_f32;
    let card_h = 120.0_f32;
    let card_gap = 20.0_f32;
    let grid_x = 40.0_f32;
    let grid_y = context_dimension.margin.top_y + 80.0;
    let inner_pad = 8.0_f32;

    struct LayoutCard {
        name: &'static str,
        shortcut: &'static str,
        // Pane rects as (x_frac, y_frac, w_frac, h_frac) within the card
        panes: Vec<(f32, f32, f32, f32)>,
    }

    let cards = vec![
        LayoutCard {
            name: "Side by Side",
            shortcut: "Two equal panes",
            panes: vec![(0.0, 0.0, 0.48, 1.0), (0.52, 0.0, 0.48, 1.0)],
        },
        LayoutCard {
            name: "Dev",
            shortcut: "Editor + 2 terminals",
            panes: vec![(0.0, 0.0, 0.55, 1.0), (0.58, 0.0, 0.42, 0.48), (0.58, 0.52, 0.42, 0.48)],
        },
        LayoutCard {
            name: "Quad",
            shortcut: "Four equal panes",
            panes: vec![(0.0, 0.0, 0.48, 0.48), (0.52, 0.0, 0.48, 0.48), (0.0, 0.52, 0.48, 0.48), (0.52, 0.52, 0.48, 0.48)],
        },
        LayoutCard {
            name: "Monitoring",
            shortcut: "Main + 2 side",
            panes: vec![(0.0, 0.0, 0.6, 1.0), (0.63, 0.0, 0.37, 0.48), (0.63, 0.52, 0.37, 0.48)],
        },
    ];

    for (i, card) in cards.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let cx = grid_x + col as f32 * (card_w + card_gap);
        let cy = grid_y + row as f32 * (card_h + card_gap + 25.0);
        let is_selected = i == selected_index;

        // Card background
        let card_bg = if is_selected { pane_selected } else { pane_bg };
        objects.push(Object::Quad(Quad {
            position: [cx, cy], color: card_bg,
            size: [card_w, card_h],
            border_radius: [6.0; 4],
            border_color: if is_selected { highlight } else { pane_border },
            border_width: if is_selected { 2.0 } else { 1.0 },
            ..Quad::default()
        }));

        // Pane preview rectangles inside the card
        let inner_x = cx + inner_pad;
        let inner_y = cy + inner_pad;
        let inner_w = card_w - inner_pad * 2.0;
        let inner_h = card_h - inner_pad * 2.0 - 18.0; // leave room for label

        for pane in &card.panes {
            let px = inner_x + pane.0 * inner_w;
            let py = inner_y + pane.1 * inner_h;
            let pw = pane.2 * inner_w;
            let ph = pane.3 * inner_h;
            objects.push(Object::Quad(Quad {
                position: [px, py],
                color: [0.08, 0.08, 0.1, 1.0],
                size: [pw, ph],
                border_radius: [3.0; 4],
                border_color: [0.25, 0.25, 0.3, 0.8],
                border_width: 1.0,
                ..Quad::default()
            }));
        }

        // Card label
        let label_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&label_rt, 11.);
        let label_color = if is_selected { highlight } else { white };
        sugarloaf.content().sel(label_rt).clear().new_line()
            .add_text(card.name, FragmentStyle { color: label_color, ..FragmentStyle::default() })
            .add_text("  ", FragmentStyle { color: dim, ..FragmentStyle::default() })
            .add_text(card.shortcut, FragmentStyle { color: dim, ..FragmentStyle::default() })
            .build();
        objects.push(Object::RichText(RichText {
            id: label_rt, position: [cx + 6.0, cy + card_h - 16.0], lines: None,
        }));
    }

    // Footer
    let footer_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&footer_rt, 11.);
    let key_bg = FragmentStyle { background_color: Some(highlight), color: [0.0; 4], ..FragmentStyle::default() };
    let dim_s = FragmentStyle { color: dim, ..FragmentStyle::default() };
    sugarloaf.content().sel(footer_rt).clear().new_line()
        .add_text(" Arrow keys ", key_bg).add_text(" select  ", dim_s)
        .add_text(" Enter ", key_bg).add_text(" apply  ", dim_s)
        .add_text(" Escape ", key_bg).add_text(" close", dim_s)
        .build();
    let footer_y = grid_y + 2.0 * (card_h + card_gap + 25.0) + 10.0;
    objects.push(Object::RichText(RichText {
        id: footer_rt, position: [40., footer_y], lines: None,
    }));

    sugarloaf.set_objects(objects);
}
