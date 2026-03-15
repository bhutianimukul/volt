use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

/// Session sharing status
#[derive(Debug, Clone, PartialEq)]
pub enum SharingState {
    Idle,
    Hosting { port: u16 },
    Connecting { host: String },
}

impl Default for SharingState {
    fn default() -> Self {
        Self::Idle
    }
}

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    state: &SharingState,
    selected_action: usize,
) {
    let bg = [0.07, 0.07, 0.07, 1.0];
    let accent = [0.4, 0.7, 1.0, 1.0]; // blue
    let dim = [0.45, 0.45, 0.5, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let green = [0.3, 0.85, 0.4, 1.0];
    let card_bg = [0.1, 0.1, 0.14, 1.0];
    let card_border = [0.2, 0.25, 0.35, 0.8];

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
            "Session Sharing",
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
    let subtitle = match state {
        SharingState::Idle => "Share your terminal session over the network",
        SharingState::Hosting { .. } => "You are hosting a shared session",
        SharingState::Connecting { .. } => "Connecting to a shared session...",
    };
    sugarloaf
        .content()
        .sel(subtitle_rt)
        .clear()
        .add_text(
            subtitle,
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

    let body_y = context_dimension.margin.top_y + 85.0;

    match state {
        SharingState::Idle => {
            // Two action cards: Host / Connect
            let card_w = 180.0_f32;
            let card_h = 120.0_f32;
            let card_gap = 20.0_f32;

            for (i, (title, desc, icon)) in [
                ("Host Session", "Others can view your terminal", "\u{25B6}"),
                ("Connect", "Join a shared session", "\u{25C0}"),
            ]
            .iter()
            .enumerate()
            {
                let cx = 40.0 + i as f32 * (card_w + card_gap);
                let cy = body_y;
                let is_selected = i == selected_action;

                let bg_c = if is_selected {
                    [0.12, 0.15, 0.25, 1.0]
                } else {
                    card_bg
                };
                objects.push(Object::Quad(Quad {
                    position: [cx, cy],
                    color: bg_c,
                    size: [card_w, card_h],
                    border_radius: [6.0; 4],
                    border_color: if is_selected { highlight } else { card_border },
                    border_width: if is_selected { 2.0 } else { 1.0 },
                    ..Quad::default()
                }));

                let card_rt = sugarloaf.create_temp_rich_text();
                sugarloaf.set_rich_text_font_size(&card_rt, 12.0);
                let cc = sugarloaf.content().sel(card_rt);
                cc.clear();
                cc.new_line();
                cc.add_text(
                    &format!("  {} ", icon),
                    FragmentStyle {
                        color: if is_selected { accent } else { dim },
                        ..FragmentStyle::default()
                    },
                );
                cc.add_text(
                    title,
                    FragmentStyle {
                        color: if is_selected { white } else { white },
                        ..FragmentStyle::default()
                    },
                );
                cc.new_line();
                cc.new_line();
                cc.add_text(
                    &format!("  {}", desc),
                    FragmentStyle {
                        color: dim,
                        ..FragmentStyle::default()
                    },
                );
                if is_selected {
                    cc.new_line();
                    cc.new_line();
                    cc.add_text(
                        "  Press Enter",
                        FragmentStyle {
                            color: accent,
                            ..FragmentStyle::default()
                        },
                    );
                }
                cc.build();
                objects.push(Object::RichText(RichText {
                    id: card_rt,
                    position: [cx + 4.0, cy + 10.0],
                    lines: None,
                }));
            }

            // Info text below cards
            let info_rt = sugarloaf.create_temp_rich_text();
            sugarloaf.set_rich_text_font_size(&info_rt, 11.0);
            let ic = sugarloaf.content().sel(info_rt);
            ic.clear();
            ic.add_text(
                "Sessions are shared over TCP on your local network.",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
            ic.new_line();
            ic.add_text(
                "The host streams terminal output; viewers are read-only.",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
            ic.build();
            objects.push(Object::RichText(RichText {
                id: info_rt,
                position: [40., body_y + card_h + 20.0],
                lines: None,
            }));
        }
        SharingState::Hosting { port } => {
            let status_rt = sugarloaf.create_temp_rich_text();
            sugarloaf.set_rich_text_font_size(&status_rt, 14.0);
            let sc = sugarloaf.content().sel(status_rt);
            sc.clear();
            sc.add_text(
                "\u{25CF} Hosting on port ",
                FragmentStyle {
                    color: green,
                    ..FragmentStyle::default()
                },
            );
            sc.add_text(
                &port.to_string(),
                FragmentStyle {
                    color: highlight,
                    ..FragmentStyle::default()
                },
            );
            sc.new_line();
            sc.new_line();
            sc.add_text(
                "Others can connect with:  volt --connect <your-ip>:",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
            sc.add_text(
                &port.to_string(),
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            );
            sc.new_line();
            sc.new_line();
            sc.add_text(
                "Press ",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
            sc.add_text(
                "q",
                FragmentStyle {
                    color: highlight,
                    ..FragmentStyle::default()
                },
            );
            sc.add_text(
                " to stop hosting",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
            sc.build();
            objects.push(Object::RichText(RichText {
                id: status_rt,
                position: [40., body_y],
                lines: None,
            }));
        }
        SharingState::Connecting { host } => {
            let status_rt = sugarloaf.create_temp_rich_text();
            sugarloaf.set_rich_text_font_size(&status_rt, 14.0);
            let sc = sugarloaf.content().sel(status_rt);
            sc.clear();
            sc.add_text(
                "Connecting to ",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
            sc.add_text(
                host,
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            );
            sc.add_text(
                " ...",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
            sc.build();
            objects.push(Object::RichText(RichText {
                id: status_rt,
                position: [40., body_y],
                lines: None,
            }));
        }
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
    fc.add_text(" \u{2190}\u{2192} ", key_bg)
        .add_text(" select  ", dim_s);
    fc.add_text(" Enter ", key_bg).add_text(" confirm  ", dim_s);
    fc.add_text(" Esc ", key_bg).add_text(" close", dim_s);
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
