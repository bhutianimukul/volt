use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    sessions: &[(String, String, bool)],
    selected_index: usize,
) {
    let bg = [0.07, 0.07, 0.07, 1.0];
    let accent = [0.3, 0.85, 0.4, 1.0]; // green
    let dim = [0.45, 0.45, 0.5, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let sidebar_bg = [0.09, 0.09, 0.09, 1.0];
    let divider_color = [0.15, 0.15, 0.15, 1.0];
    let selected_bg = [0.12, 0.18, 0.12, 1.0];
    let pane_inner = [0.06, 0.07, 0.06, 1.0];
    let pane_border = [0.2, 0.35, 0.2, 0.8];
    let term_green = [0.3, 0.8, 0.4, 1.0];
    let red = [0.9, 0.3, 0.3, 1.0];

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

    // Left sidebar (session list) — 200px wide
    let sidebar_width = 200.0;
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: sidebar_bg,
        size: [sidebar_width, full_h],
        ..Quad::default()
    }));

    // Divider
    objects.push(Object::Quad(Quad {
        position: [sidebar_width, 0.0],
        color: divider_color,
        size: [1.0, full_h],
        ..Quad::default()
    }));

    // Sidebar title
    let title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&title_rt, 11.0);
    sugarloaf
        .content()
        .sel(title_rt)
        .clear()
        .add_text(
            "TMUX SESSIONS",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        )
        .build();
    objects.push(Object::RichText(RichText {
        id: title_rt,
        position: [16., context_dimension.margin.top_y + 25.],
        lines: None,
    }));

    // Session list in sidebar
    let list_start_y = context_dimension.margin.top_y + 48.0;
    let row_height = 28.0;

    if sessions.is_empty() {
        let empty_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&empty_rt, 12.0);
        let ec = sugarloaf.content().sel(empty_rt);
        ec.clear();
        ec.add_text(
            "No sessions",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
        ec.new_line();
        ec.new_line();
        ec.add_text(
            "Press ",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
        ec.add_text(
            "n",
            FragmentStyle {
                color: highlight,
                ..FragmentStyle::default()
            },
        );
        ec.add_text(
            " to create one",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
        ec.build();
        objects.push(Object::RichText(RichText {
            id: empty_rt,
            position: [16., list_start_y + 5.0],
            lines: None,
        }));
    } else {
        for (i, (_id, name, attached)) in sessions.iter().enumerate() {
            let row_y = list_start_y + (i as f32 * row_height);
            let is_selected = i == selected_index;

            // Selection highlight
            if is_selected {
                objects.push(Object::Quad(Quad {
                    position: [0., row_y - 3.0],
                    color: selected_bg,
                    size: [sidebar_width, row_height],
                    ..Quad::default()
                }));
            }

            let row_rt = sugarloaf.create_temp_rich_text();
            sugarloaf.set_rich_text_font_size(&row_rt, 12.0);
            let rc = sugarloaf.content().sel(row_rt);
            rc.clear();

            // Selection indicator
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

            // Status dot
            if *attached {
                rc.add_text(
                    "\u{25CF} ",
                    FragmentStyle {
                        color: accent,
                        ..FragmentStyle::default()
                    },
                );
            } else {
                rc.add_text(
                    "\u{25CB} ",
                    FragmentStyle {
                        color: dim,
                        ..FragmentStyle::default()
                    },
                );
            }

            // Session name
            let name_color = if is_selected { white } else { dim };
            rc.add_text(
                name,
                FragmentStyle {
                    color: name_color,
                    ..FragmentStyle::default()
                },
            );

            rc.build();
            objects.push(Object::RichText(RichText {
                id: row_rt,
                position: [12., row_y + 3.0],
                lines: None,
            }));
        }
    }

    // Right panel — detail view for selected session
    let panel_x = sidebar_width + 20.0;
    let panel_y = context_dimension.margin.top_y + 20.0;

    if !sessions.is_empty() {
        let clamped = selected_index.min(sessions.len().saturating_sub(1));
        let (id, name, attached) = &sessions[clamped];

        // Session title
        let detail_title_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&detail_title_rt, 20.0);
        sugarloaf
            .content()
            .sel(detail_title_rt)
            .clear()
            .add_text(
                name,
                FragmentStyle {
                    color: white,
                    ..FragmentStyle::default()
                },
            )
            .build();
        objects.push(Object::RichText(RichText {
            id: detail_title_rt,
            position: [panel_x, panel_y],
            lines: None,
        }));

        // Status + ID
        let detail_sub_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&detail_sub_rt, 12.0);
        let dc = sugarloaf.content().sel(detail_sub_rt);
        dc.clear();
        if *attached {
            dc.add_text(
                "\u{25CF} attached",
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            );
        } else {
            dc.add_text(
                "\u{25CB} detached",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
        }
        dc.add_text(
            &format!("  id: {}", id),
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
        dc.build();
        objects.push(Object::RichText(RichText {
            id: detail_sub_rt,
            position: [panel_x, panel_y + 28.0],
            lines: None,
        }));

        // Terminal preview card
        let preview_y = panel_y + 58.0;
        let preview_w = full_w - panel_x - 20.0;
        let preview_h = 80.0_f32;

        objects.push(Object::Quad(Quad {
            position: [panel_x, preview_y],
            color: pane_inner,
            size: [preview_w.min(360.0), preview_h],
            border_radius: [6.0; 4],
            border_color: pane_border,
            border_width: 1.0,
            ..Quad::default()
        }));

        let term_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&term_rt, 11.0);
        let tc = sugarloaf.content().sel(term_rt);
        tc.clear();
        tc.add_text(
            &format!(" ~ $ tmux attach -t {}", name),
            FragmentStyle {
                color: term_green,
                ..FragmentStyle::default()
            },
        );
        tc.new_line();
        tc.add_text(
            &format!(" ~/{} $ ", name),
            FragmentStyle {
                color: term_green,
                ..FragmentStyle::default()
            },
        );
        tc.add_text(
            "_",
            FragmentStyle {
                color: [0.5, 0.9, 0.5, 0.7],
                ..FragmentStyle::default()
            },
        );
        tc.build();
        objects.push(Object::RichText(RichText {
            id: term_rt,
            position: [panel_x + 8.0, preview_y + 10.0],
            lines: None,
        }));

        // Action buttons
        let actions_y = preview_y + preview_h + 20.0;
        let actions_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&actions_rt, 12.0);
        let ac = sugarloaf.content().sel(actions_rt);
        ac.clear();

        if *attached {
            ac.add_text(
                " d ",
                FragmentStyle {
                    color: [0.0, 0.0, 0.0, 1.0],
                    background_color: Some(highlight),
                    ..FragmentStyle::default()
                },
            );
            ac.add_text(
                " Detach    ",
                FragmentStyle {
                    color: white,
                    ..FragmentStyle::default()
                },
            );
        } else {
            ac.add_text(
                " Enter ",
                FragmentStyle {
                    color: [0.0, 0.0, 0.0, 1.0],
                    background_color: Some(accent),
                    ..FragmentStyle::default()
                },
            );
            ac.add_text(
                " Attach    ",
                FragmentStyle {
                    color: white,
                    ..FragmentStyle::default()
                },
            );
        }

        ac.add_text(
            " x ",
            FragmentStyle {
                color: [0.0, 0.0, 0.0, 1.0],
                background_color: Some(red),
                ..FragmentStyle::default()
            },
        );
        ac.add_text(
            " Kill    ",
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        );

        ac.add_text(
            " n ",
            FragmentStyle {
                color: [0.0, 0.0, 0.0, 1.0],
                background_color: Some(highlight),
                ..FragmentStyle::default()
            },
        );
        ac.add_text(
            " New",
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        );

        ac.build();
        objects.push(Object::RichText(RichText {
            id: actions_rt,
            position: [panel_x, actions_y],
            lines: None,
        }));
    }

    // Footer
    let footer_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&footer_rt, 11.);
    let key_bg = FragmentStyle {
        color: [0.0, 0.0, 0.0, 1.0],
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
    fc.add_text(" Enter ", key_bg).add_text(" attach  ", dim_s);
    fc.add_text(" n ", key_bg).add_text(" new  ", dim_s);
    fc.add_text(" d ", key_bg).add_text(" detach  ", dim_s);
    fc.add_text(" x ", key_bg).add_text(" kill  ", dim_s);
    fc.add_text(" r ", key_bg).add_text(" rename  ", dim_s);
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
        position: [16., footer_y + 4.0],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}
