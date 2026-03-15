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

    // --- Settings list (two-column layout) ---
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

    let filtered = editor.filtered_items();

    // Split items into two columns:
    // Left: Font, Window, Navigation, Colors
    // Right: everything else (General, Cursor, Scroll, Renderer, Developer, Shell, Keyboard, Title, Bell, Hints)
    let left_categories = ["Font", "Window", "Navigation", "Colors"];
    let mut left_items: Vec<(usize, &crate::settings_editor::SettingItem)> = Vec::new();
    let mut right_items: Vec<(usize, &crate::settings_editor::SettingItem)> = Vec::new();

    for (idx, &item) in filtered.iter().enumerate() {
        if left_categories.contains(&item.category.as_str()) {
            left_items.push((idx, item));
        } else {
            right_items.push((idx, item));
        }
    }

    // Helper closure to build a column's rich text content
    // We render each column as a separate RichText object at different x positions
    let body_y = context_dimension.margin.top_y + 85.;

    // --- Left column ---
    let left_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&left_rt, 12.0);

    {
        let content = sugarloaf.content();
        let body = content.sel(left_rt).clear();
        let mut last_category = String::new();

        for &(display_idx, item) in left_items.iter() {
            let is_selected = display_idx == editor.selected_index;

            if item.category != last_category {
                body.add_text(&format!("[{}]", item.category), category_style);
                body.new_line();
                last_category = item.category.clone();
            }

            if is_selected {
                body.add_text("> ", FragmentStyle {
                    color: highlight,
                    ..FragmentStyle::default()
                });
            } else {
                body.add_text("  ", label_style);
            }

            let row_label_style = if is_selected { selected_style } else { label_style };
            let padded_label = format!("{:<22}", item.label);
            body.add_text(&padded_label, row_label_style);

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

            if is_selected && !editor.editing {
                body.add_text(&format!("  {}", item.description), FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                });
            }

            body.new_line();
        }

        body.new_line();
        body.new_line();

        // Footer (only in left column)
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
                .add_text(" i ", key_bg_style)
                .add_text(" import  ", dim_style())
                .add_text(" Escape ", key_bg_style)
                .add_text(" close", dim_style());
        }

        body.build();
    }

    objects.push(Object::RichText(RichText {
        id: left_rt,
        position: [40., body_y],
        lines: None,
    }));

    // --- Right column ---
    let right_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&right_rt, 12.0);

    {
        let content = sugarloaf.content();
        let body = content.sel(right_rt).clear();
        let mut last_category = String::new();

        for &(display_idx, item) in right_items.iter() {
            let is_selected = display_idx == editor.selected_index;

            if item.category != last_category {
                body.add_text(&format!("[{}]", item.category), category_style);
                body.new_line();
                last_category = item.category.clone();
            }

            if is_selected {
                body.add_text("> ", FragmentStyle {
                    color: highlight,
                    ..FragmentStyle::default()
                });
            } else {
                body.add_text("  ", label_style);
            }

            let row_label_style = if is_selected { selected_style } else { label_style };
            let padded_label = format!("{:<22}", item.label);
            body.add_text(&padded_label, row_label_style);

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

            if is_selected && !editor.editing {
                body.add_text(&format!("  {}", item.description), FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                });
            }

            body.new_line();
        }

        body.build();
    }

    objects.push(Object::RichText(RichText {
        id: right_rt,
        position: [380., body_y],
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
