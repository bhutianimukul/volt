use crate::context::grid::ContextDimension;
use crate::time_travel::SessionRecorder;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    recorder: &SessionRecorder,
    selected_index: usize,
    scroll_offset: usize,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.9, 0.6, 0.2, 1.0]; // amber/orange for time travel
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let green = [0.3, 0.9, 0.3, 1.0];
    let red = [0.9, 0.3, 0.3, 1.0];
    let sidebar_bg = [0.08, 0.08, 0.11, 1.0];
    let divider_color = [0.15, 0.15, 0.2, 1.0];
    let selected_bg = [0.2, 0.15, 0.08, 1.0];

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

    // Left sidebar — timeline
    let sidebar_width = 250.0;
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: sidebar_bg,
        size: [sidebar_width, full_h],
        ..Quad::default()
    }));
    objects.push(Object::Quad(Quad {
        position: [sidebar_width, 0.0],
        color: divider_color,
        size: [1.0, full_h],
        ..Quad::default()
    }));

    // Title
    let title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&title_rt, 11.0);
    sugarloaf
        .content()
        .sel(title_rt)
        .clear()
        .add_text(
            "TIME TRAVEL",
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

    // Command timeline in sidebar
    let entries = recorder.recent(100);
    let display_order: Vec<_> = entries.into_iter().rev().collect();
    let entry_count = display_order.len();
    let clamped_selected = if entry_count > 0 {
        selected_index.min(entry_count - 1)
    } else {
        0
    };

    let list_start_y = context_dimension.margin.top_y + 48.0;
    let row_height = 22.0;

    if display_order.is_empty() {
        let empty_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&empty_rt, 12.0);
        sugarloaf
            .content()
            .sel(empty_rt)
            .clear()
            .add_text(
                "No commands recorded yet.",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            )
            .build();
        objects.push(Object::RichText(RichText {
            id: empty_rt,
            position: [16., list_start_y + 5.0],
            lines: None,
        }));
    } else {
        let timeline_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&timeline_rt, 11.0);
        let tc = sugarloaf.content().sel(timeline_rt);
        tc.clear();

        let mut line_idx: usize = 0;
        for (i, entry) in display_order.iter().enumerate() {
            if line_idx < scroll_offset {
                line_idx += 1;
                continue;
            }

            let is_selected = i == clamped_selected;

            // Status indicator
            let (status_char, status_color) = match entry.exit_code {
                Some(0) => ("\u{2713}", green),
                Some(_) => ("\u{2717}", red),
                None => ("\u{2022}", dim),
            };

            if is_selected {
                tc.add_text(
                    "> ",
                    FragmentStyle {
                        color: accent,
                        ..FragmentStyle::default()
                    },
                );
            } else {
                tc.add_text(
                    "  ",
                    FragmentStyle {
                        color: dim,
                        ..FragmentStyle::default()
                    },
                );
            }

            tc.add_text(
                &format!("{} ", status_char),
                FragmentStyle {
                    color: status_color,
                    ..FragmentStyle::default()
                },
            );

            // Truncated command
            let cmd = if entry.command.len() > 28 {
                format!("{}..", &entry.command[..26])
            } else {
                entry.command.clone()
            };
            let cmd_color = if is_selected { white } else { dim };
            tc.add_text(
                &cmd,
                FragmentStyle {
                    color: cmd_color,
                    ..FragmentStyle::default()
                },
            );

            tc.new_line();
            line_idx += 1;
        }

        tc.build();

        // Selection highlight — must be pushed BEFORE text so text renders on top
        if entry_count > 0 {
            let visible_sel = clamped_selected.saturating_sub(scroll_offset);
            let sel_y = list_start_y + (visible_sel as f32 * row_height) - 3.0;
            objects.push(Object::Quad(Quad {
                position: [0., sel_y],
                color: selected_bg,
                size: [sidebar_width, row_height],
                ..Quad::default()
            }));
        }

        objects.push(Object::RichText(RichText {
            id: timeline_rt,
            position: [12., list_start_y + 3.0],
            lines: None,
        }));
    }

    // Right panel — detail view for selected command
    let panel_x = sidebar_width + 20.0;
    let panel_y = context_dimension.margin.top_y + 20.0;

    // Panel title
    let panel_title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&panel_title_rt, 20.0);
    sugarloaf
        .content()
        .sel(panel_title_rt)
        .clear()
        .add_text(
            "Command Detail",
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        )
        .build();
    objects.push(Object::RichText(RichText {
        id: panel_title_rt,
        position: [panel_x, panel_y],
        lines: None,
    }));

    if !display_order.is_empty() {
        let entry = &display_order[clamped_selected];

        let detail_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&detail_rt, 12.0);
        let dc = sugarloaf.content().sel(detail_rt);
        dc.clear();

        // Command
        dc.add_text(
            "COMMAND",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        );
        dc.new_line();
        dc.add_text(
            &format!("  $ {}", entry.command),
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        );
        dc.new_line();
        dc.new_line();

        // Status
        dc.add_text(
            "STATUS",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        );
        dc.new_line();
        match entry.exit_code {
            Some(0) => {
                dc.add_text(
                    "  \u{2713} Success (exit 0)",
                    FragmentStyle {
                        color: green,
                        ..FragmentStyle::default()
                    },
                );
            }
            Some(code) => {
                dc.add_text(
                    &format!("  \u{2717} Failed (exit {})", code),
                    FragmentStyle {
                        color: red,
                        ..FragmentStyle::default()
                    },
                );
            }
            None => {
                dc.add_text(
                    "  \u{2022} Running...",
                    FragmentStyle {
                        color: dim,
                        ..FragmentStyle::default()
                    },
                );
            }
        }
        dc.new_line();
        dc.new_line();

        // Duration
        if let Some(ms) = entry.duration_ms {
            dc.add_text(
                "DURATION",
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            );
            dc.new_line();
            let dur = if ms < 1000 {
                format!("  {}ms", ms)
            } else {
                format!("  {:.1}s", ms as f64 / 1000.0)
            };
            dc.add_text(
                &dur,
                FragmentStyle {
                    color: white,
                    ..FragmentStyle::default()
                },
            );
            dc.new_line();
            dc.new_line();
        }

        // Working directory
        dc.add_text(
            "DIRECTORY",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        );
        dc.new_line();
        dc.add_text(
            &format!("  {}", entry.working_dir.display()),
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
        dc.new_line();
        dc.new_line();

        // Timestamp
        dc.add_text(
            "TIME",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        );
        dc.new_line();
        if let Ok(elapsed) = entry.timestamp.elapsed() {
            let secs = elapsed.as_secs();
            let ago = if secs < 60 {
                format!("  {}s ago", secs)
            } else if secs < 3600 {
                format!("  {}m {}s ago", secs / 60, secs % 60)
            } else {
                format!("  {}h {}m ago", secs / 3600, (secs % 3600) / 60)
            };
            dc.add_text(
                &ago,
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            );
        }
        dc.new_line();
        dc.new_line();

        // Output preview
        if !entry.output_preview.is_empty() {
            dc.add_text(
                "OUTPUT PREVIEW",
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            );
            dc.new_line();
            // Show first few lines of output
            for line in entry.output_preview.lines().take(6) {
                let truncated = if line.len() > 60 {
                    format!("  {}..", &line[..58])
                } else {
                    format!("  {}", line)
                };
                dc.add_text(
                    &truncated,
                    FragmentStyle {
                        color: dim,
                        ..FragmentStyle::default()
                    },
                );
                dc.new_line();
            }
        }

        dc.build();
        objects.push(Object::RichText(RichText {
            id: detail_rt,
            position: [panel_x, panel_y + 35.0],
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
        .add_text(" navigate  ", dim_s);
    fc.add_text(" Enter ", key_bg).add_text(" replay  ", dim_s);
    fc.add_text(" c ", key_bg).add_text(" copy  ", dim_s);
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
