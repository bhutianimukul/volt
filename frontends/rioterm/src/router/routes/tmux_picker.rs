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
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let card_bg = [0.1, 0.12, 0.1, 1.0];
    let card_selected = [0.12, 0.18, 0.15, 1.0];
    let pane_inner = [0.06, 0.07, 0.06, 1.0];
    let pane_border = [0.2, 0.35, 0.2, 0.8];
    let term_green = [0.3, 0.8, 0.4, 1.0];
    let term_dim = [0.25, 0.5, 0.3, 0.7];

    let layout = sugarloaf.window_size();
    let scale = context_dimension.dimension.scale;
    let mut objects = Vec::with_capacity(48);

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
        .add_text("tmux Sessions", FragmentStyle { color: white, ..FragmentStyle::default() })
        .build();
    objects.push(Object::RichText(RichText {
        id: title_rt, position: [40., context_dimension.margin.top_y + 25.], lines: None,
    }));

    let count_text = if sessions.is_empty() {
        "No sessions — press n to create one".to_string()
    } else {
        format!("{} session{}", sessions.len(), if sessions.len() == 1 { "" } else { "s" })
    };
    let sub_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&sub_rt, 13.0);
    sugarloaf.content().sel(sub_rt).clear()
        .add_text(&count_text, FragmentStyle { color: dim, ..FragmentStyle::default() })
        .build();
    objects.push(Object::RichText(RichText {
        id: sub_rt, position: [40., context_dimension.margin.top_y + 55.], lines: None,
    }));

    if sessions.is_empty() {
        // Empty state card
        let card_w = 380.0_f32;
        let card_h = 140.0_f32;
        let cx = 40.0;
        let cy = context_dimension.margin.top_y + 80.0;

        objects.push(Object::Quad(Quad {
            position: [cx, cy], color: card_bg,
            size: [card_w, card_h], border_radius: [6.0; 4],
            border_color: pane_border, border_width: 1.0,
            ..Quad::default()
        }));

        // Fake terminal lines inside
        let inner_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&inner_rt, 10.);
        sugarloaf.content().sel(inner_rt).clear().new_line()
            .add_text("  $ tmux new -s my-project", FragmentStyle { color: term_green, ..FragmentStyle::default() })
            .new_line()
            .add_text("  [creates a new tmux session]", FragmentStyle { color: term_dim, ..FragmentStyle::default() })
            .new_line().new_line()
            .add_text("  Press ", FragmentStyle { color: dim, ..FragmentStyle::default() })
            .add_text("n", FragmentStyle { color: highlight, ..FragmentStyle::default() })
            .add_text(" to create a new session", FragmentStyle { color: dim, ..FragmentStyle::default() })
            .build();
        objects.push(Object::RichText(RichText {
            id: inner_rt, position: [cx + 10.0, cy + 10.0], lines: None,
        }));
    } else {
        // 2-column grid of session cards
        let card_w = 180.0_f32;
        let card_h = 110.0_f32;
        let card_gap = 20.0_f32;
        let grid_x = 40.0_f32;
        let grid_y = context_dimension.margin.top_y + 80.0;
        let inner_pad = 8.0_f32;

        for (i, (_id, name, attached)) in sessions.iter().enumerate() {
            let col = i % 2;
            let row = i / 2;
            let cx = grid_x + col as f32 * (card_w + card_gap);
            let cy = grid_y + row as f32 * (card_h + card_gap);
            let is_selected = i == selected_index;

            // Card background
            let bg_color = if is_selected { card_selected } else { card_bg };
            objects.push(Object::Quad(Quad {
                position: [cx, cy], color: bg_color,
                size: [card_w, card_h], border_radius: [6.0; 4],
                border_color: if is_selected { highlight } else { pane_border },
                border_width: if is_selected { 2.0 } else { 1.0 },
                ..Quad::default()
            }));

            // Fake terminal content inside card
            let inner_x = cx + inner_pad;
            let inner_y = cy + inner_pad;
            let inner_w = card_w - inner_pad * 2.0;
            let inner_h = card_h - inner_pad * 2.0 - 20.0;

            objects.push(Object::Quad(Quad {
                position: [inner_x, inner_y], color: pane_inner,
                size: [inner_w, inner_h], border_radius: [3.0; 4],
                border_color: pane_border, border_width: 1.0,
                ..Quad::default()
            }));

            // Simulated terminal lines
            let term_rt = sugarloaf.create_temp_rich_text();
            sugarloaf.set_rich_text_font_size(&term_rt, 9.);
            let c = sugarloaf.content().sel(term_rt);
            c.clear().new_line();
            c.add_text(&format!(" ~ $ cd {}", name), FragmentStyle { color: term_green, ..FragmentStyle::default() });
            c.new_line();
            c.add_text(&format!(" ~/{} $ ", name), FragmentStyle { color: term_green, ..FragmentStyle::default() });
            c.add_text("_", FragmentStyle { color: [0.5, 0.9, 0.5, 0.7], ..FragmentStyle::default() });
            c.build();
            objects.push(Object::RichText(RichText {
                id: term_rt, position: [inner_x + 4.0, inner_y + 4.0], lines: None,
            }));

            // Session name + status label below card content
            let label_rt = sugarloaf.create_temp_rich_text();
            sugarloaf.set_rich_text_font_size(&label_rt, 11.);
            let name_color = if is_selected { highlight } else { white };
            let lc = sugarloaf.content().sel(label_rt);
            lc.clear().new_line();
            lc.add_text(name, FragmentStyle { color: name_color, ..FragmentStyle::default() });
            if *attached {
                lc.add_text("  attached", FragmentStyle { color: accent, ..FragmentStyle::default() });
            }
            lc.build();
            objects.push(Object::RichText(RichText {
                id: label_rt, position: [cx + 6.0, cy + card_h - 18.0], lines: None,
            }));
        }
    }

    // Footer
    let footer_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&footer_rt, 11.);
    let key_bg = FragmentStyle { background_color: Some(highlight), color: [0.0; 4], ..FragmentStyle::default() };
    let dim_s = FragmentStyle { color: dim, ..FragmentStyle::default() };
    let fc = sugarloaf.content().sel(footer_rt);
    fc.clear().new_line();
    fc.add_text(" Arrows ", key_bg).add_text(" select ", dim_s);
    fc.add_text(" Enter ", key_bg).add_text(" attach ", dim_s);
    fc.add_text(" n ", key_bg).add_text(" new ", dim_s);
    fc.add_text(" d ", key_bg).add_text(" detach ", dim_s);
    fc.add_text(" x ", key_bg).add_text(" kill ", dim_s);
    fc.add_text(" Esc ", key_bg).add_text(" close", dim_s);
    fc.build();

    let footer_y = if sessions.is_empty() {
        context_dimension.margin.top_y + 240.0
    } else {
        let rows = (sessions.len() + 1) / 2;
        context_dimension.margin.top_y + 80.0 + rows as f32 * 130.0 + 10.0
    };
    objects.push(Object::RichText(RichText {
        id: footer_rt, position: [40., footer_y], lines: None,
    }));

    sugarloaf.set_objects(objects);
}
