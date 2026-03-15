use crate::context::grid::ContextDimension;
use crate::settings_editor::SettingsEditor;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    editor: &SettingsEditor,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.2, 0.5, 1.0, 1.0]; // blue
    let dim = [0.45, 0.45, 0.5, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0]; // yellow
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let selected_bg = [0.15, 0.15, 0.2, 1.0];
    let editing_bg = [0.2, 0.15, 0.05, 1.0];
    let green = [0.3, 0.85, 0.4, 1.0];
    let red = [0.85, 0.3, 0.3, 1.0];

    let layout = sugarloaf.window_size();
    let mut objects = Vec::with_capacity(32);

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

    // --- Title ---
    let title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&title_rt, 22.0);

    let content = sugarloaf.content();
    content
        .sel(title_rt)
        .clear()
        .add_text(
            "Volt Settings",
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

    // --- Subtitle / Search bar ---
    let subtitle_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&subtitle_rt, 13.0);

    let content = sugarloaf.content();
    let sub = content.sel(subtitle_rt).clear();

    if editor.searching {
        sub.add_text(
            "Search: ",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        )
        .add_text(
            &editor.search_query,
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            },
        )
        .add_text(
            "_",
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        );
    } else if !editor.search_query.is_empty() {
        sub.add_text(
            &format!("Filtered: \"{}\"  ", editor.search_query),
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        )
        .add_text(
            "/ to clear",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
    } else {
        sub.add_text(
            "Interactive settings editor  |  / to search",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
    }
    sub.build();

    objects.push(Object::RichText(RichText {
        id: subtitle_rt,
        position: [40., context_dimension.margin.top_y + 55.],
        lines: None,
    }));

    // --- Settings list ---
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 13.0);

    let label_style = FragmentStyle {
        color: dim,
        ..FragmentStyle::default()
    };
    let value_style = FragmentStyle {
        color: white,
        ..FragmentStyle::default()
    };
    let category_style = FragmentStyle {
        color: accent,
        ..FragmentStyle::default()
    };
    let key_bg_style = FragmentStyle {
        background_color: Some(highlight),
        color: black,
        ..FragmentStyle::default()
    };
    let selected_style = FragmentStyle {
        background_color: Some(selected_bg),
        color: white,
        ..FragmentStyle::default()
    };
    let editing_style = FragmentStyle {
        background_color: Some(editing_bg),
        color: highlight,
        ..FragmentStyle::default()
    };

    let content = sugarloaf.content();
    let body = content.sel(body_rt).clear();

    let filtered = editor.filtered_items();
    let mut last_category = String::new();

    let end = std::cmp::min(
        editor.scroll_offset + editor.visible_rows,
        filtered.len(),
    );
    let visible_range = editor.scroll_offset..end;

    for (display_idx, &item) in filtered.iter().enumerate() {
        if !visible_range.contains(&display_idx) {
            continue;
        }

        let is_selected = display_idx == editor.selected_index;

        // Category header
        if item.category != last_category {
            if !last_category.is_empty() {
                body.add_text("", label_style).new_line();
            }
            body.add_text(&item.category.to_uppercase(), category_style)
                .new_line();
            last_category = item.category.clone();
        }

        // Selection indicator
        if is_selected {
            body.add_text(" > ", FragmentStyle {
                color: highlight,
                ..FragmentStyle::default()
            });
        } else {
            body.add_text("   ", label_style);
        }

        // Label
        let row_label_style = if is_selected {
            selected_style
        } else {
            label_style
        };

        let padded_label = format!("{:<28}", item.label);
        body.add_text(&padded_label, row_label_style);

        // Value
        if is_selected && editor.editing {
            body.add_text(&editor.edit_buffer, editing_style);
            body.add_text("_", FragmentStyle {
                color: highlight,
                ..FragmentStyle::default()
            });
        } else {
            let val_display = item.value.display();
            let val_style = if item.value.is_bool() {
                let is_true = matches!(item.value, crate::settings_editor::SettingValue::Bool(true));
                FragmentStyle {
                    color: if is_true { green } else { red },
                    ..FragmentStyle::default()
                }
            } else if is_selected {
                FragmentStyle {
                    color: highlight,
                    ..FragmentStyle::default()
                }
            } else {
                value_style
            };
            body.add_text(&val_display, val_style);
        }

        // Description for selected item
        if is_selected && !editor.editing {
            body.add_text(&format!("  {}", item.description), FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            });
        }

        body.new_line();
    }

    // Scroll indicator
    if filtered.len() > editor.visible_rows {
        body.add_text("", label_style).new_line();
        body.add_text(
            &format!(
                "   Showing {}-{} of {}",
                editor.scroll_offset + 1,
                end,
                filtered.len()
            ),
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        )
        .new_line();
    }

    body.add_text("", label_style).new_line();
    body.add_text("", label_style).new_line();

    // --- Footer with keybindings ---
    if editor.editing {
        body.add_text(" Enter ", key_bg_style)
            .add_text(" confirm  ", dim_style())
            .add_text(" Escape ", key_bg_style)
            .add_text(" cancel", dim_style());
    } else {
        body.add_text(" Up/Down ", key_bg_style)
            .add_text(" navigate  ", dim_style())
            .add_text(" Enter ", key_bg_style)
            .add_text(" edit  ", dim_style())
            .add_text(" / ", key_bg_style)
            .add_text(" search  ", dim_style())
            .add_text(" Escape ", key_bg_style)
            .add_text(" close", dim_style());
    }

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 85.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}

fn dim_style() -> FragmentStyle {
    FragmentStyle {
        color: [0.45, 0.45, 0.5, 1.0],
        ..FragmentStyle::default()
    }
}
