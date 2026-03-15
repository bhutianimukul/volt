use crate::context::grid::ContextDimension;
use crate::settings_editor::{SettingValue, SettingsEditor};
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    editor: &SettingsEditor,
    settings_category: usize,
    settings_in_sidebar: bool,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.2, 0.5, 1.0, 1.0];
    let dim = [0.45, 0.45, 0.5, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let selected_bg = [0.15, 0.15, 0.2, 1.0];
    let editing_bg = [0.2, 0.15, 0.05, 1.0];
    let green = [0.3, 0.85, 0.4, 1.0];
    let red = [0.85, 0.3, 0.3, 1.0];
    let sidebar_bg = [0.08, 0.08, 0.11, 1.0];
    let sidebar_selected = [0.15, 0.15, 0.22, 1.0];
    let sidebar_hover = [0.10, 0.10, 0.14, 1.0];
    let divider_color = [0.15, 0.15, 0.2, 1.0];

    let layout = sugarloaf.window_size();
    let scale = context_dimension.dimension.scale;
    let full_w = layout.width / scale;
    let full_h = layout.height / scale;
    let mut objects = Vec::with_capacity(64);

    // Background
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: bg,
        size: [full_w, full_h],
        ..Quad::default()
    }));

    // Sidebar background (x=0 to x=140)
    let sidebar_width = 140.0;
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: sidebar_bg,
        size: [sidebar_width, full_h],
        ..Quad::default()
    }));

    // Vertical divider between sidebar and panel
    objects.push(Object::Quad(Quad {
        position: [sidebar_width, 0.0],
        color: divider_color,
        size: [1.0, full_h],
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
        position: [155., context_dimension.margin.top_y + 25.],
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
            "Interactive settings editor",
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        );
    }
    sub.build();

    objects.push(Object::RichText(RichText {
        id: subtitle_rt,
        position: [155., context_dimension.margin.top_y + 55.],
        lines: None,
    }));

    // --- Category Sidebar ---
    let categories = editor.categories();
    let sidebar_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&sidebar_rt, 13.0);

    // Sidebar title
    let sidebar_title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&sidebar_title_rt, 11.0);
    {
        let content = sugarloaf.content();
        content
            .sel(sidebar_title_rt)
            .clear()
            .add_text(
                "CATEGORIES",
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            )
            .build();
    }
    objects.push(Object::RichText(RichText {
        id: sidebar_title_rt,
        position: [16., context_dimension.margin.top_y + 25.],
        lines: None,
    }));

    // Highlight quad for selected category
    let cat_start_y = context_dimension.margin.top_y + 48.0;
    let cat_line_height = 22.0;

    for (i, _cat) in categories.iter().enumerate() {
        let cat_y = cat_start_y + (i as f32 * cat_line_height);
        let is_selected = i == settings_category;

        if is_selected {
            let bg_color = if settings_in_sidebar {
                sidebar_selected
            } else {
                sidebar_hover
            };
            objects.push(Object::Quad(Quad {
                position: [0., cat_y - 2.0],
                color: bg_color,
                size: [sidebar_width, cat_line_height],
                ..Quad::default()
            }));
        }
    }

    {
        let content = sugarloaf.content();
        let body = content.sel(sidebar_rt).clear();

        for (i, cat) in categories.iter().enumerate() {
            let is_selected = i == settings_category;

            let style = if is_selected && settings_in_sidebar {
                FragmentStyle {
                    color: white,
                    ..FragmentStyle::default()
                }
            } else if is_selected {
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                }
            } else {
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                }
            };

            let icon = match cat.as_str() {
                "Font" => "Aa ",
                "Window" => "[] ",
                "Navigation" => ">> ",
                "Colors" => "## ",
                "General" => ":: ",
                "Cursor" => "|_ ",
                "Renderer" => "<> ",
                "Shell" => "$_ ",
                "Developer" => "// ",
                "Scroll" => "^v ",
                "Keyboard" => "kb ",
                "Title" => "Tt ",
                "Bell" => "() ",
                "Hints" => "?? ",
                _ => "   ",
            };

            body.add_text(
                icon,
                FragmentStyle {
                    color: if is_selected && settings_in_sidebar {
                        accent
                    } else {
                        dim
                    },
                    ..FragmentStyle::default()
                },
            );
            body.add_text(cat, style);
            body.new_line();
        }

        body.build();
    }

    objects.push(Object::RichText(RichText {
        id: sidebar_rt,
        position: [12., cat_start_y + 3.0],
        lines: None,
    }));

    // --- Settings Panel (right side) ---
    let current_cat = categories
        .get(settings_category)
        .cloned()
        .unwrap_or_default();
    let cat_items = editor.items_for_category(&current_cat);

    // Category header with accent underline
    let header_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&header_rt, 14.0);
    {
        let content = sugarloaf.content();
        content
            .sel(header_rt)
            .clear()
            .add_text(
                &current_cat,
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            )
            .add_text(
                &format!("  ({} settings)", cat_items.len()),
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            )
            .build();
    }

    let panel_x = 155.0;
    let header_y = context_dimension.margin.top_y + 80.0;

    objects.push(Object::RichText(RichText {
        id: header_rt,
        position: [panel_x, header_y],
        lines: None,
    }));

    // Settings items
    let items_start_y = header_y + 30.0;
    let item_line_height = 20.0;
    let desc_extra_height = 14.0;

    // Highlight quads for selected item
    if !settings_in_sidebar {
        for (i, _item) in cat_items.iter().enumerate() {
            if i == editor.selected_index {
                let item_y = items_start_y
                    + (i as f32 * (item_line_height + desc_extra_height))
                    - 3.0;
                let bg_color = if editor.editing {
                    editing_bg
                } else {
                    selected_bg
                };
                objects.push(Object::Quad(Quad {
                    position: [panel_x - 10.0, item_y],
                    color: bg_color,
                    size: [full_w - panel_x, item_line_height + 2.0],
                    ..Quad::default()
                }));
            }
        }
    }

    let panel_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&panel_rt, 12.0);

    {
        let content = sugarloaf.content();
        let body = content.sel(panel_rt).clear();

        for (i, item) in cat_items.iter().enumerate() {
            let is_selected = !settings_in_sidebar && i == editor.selected_index;

            // Indicator
            if is_selected {
                body.add_text(
                    "> ",
                    FragmentStyle {
                        color: highlight,
                        ..FragmentStyle::default()
                    },
                );
            } else {
                body.add_text(
                    "  ",
                    FragmentStyle {
                        color: dim,
                        ..FragmentStyle::default()
                    },
                );
            }

            // Label
            let label_style = if is_selected {
                FragmentStyle {
                    color: white,
                    ..FragmentStyle::default()
                }
            } else {
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                }
            };
            let padded_label = format!("{:<28}", item.label);
            body.add_text(&padded_label, label_style);

            // Value
            if is_selected && editor.editing {
                body.add_text(
                    &editor.edit_buffer,
                    FragmentStyle {
                        color: highlight,
                        background_color: Some(editing_bg),
                        ..FragmentStyle::default()
                    },
                );
                body.add_text(
                    "_",
                    FragmentStyle {
                        color: highlight,
                        ..FragmentStyle::default()
                    },
                );
            } else {
                match &item.value {
                    SettingValue::Bool(val) => {
                        if *val {
                            body.add_text(
                                "[ON]",
                                FragmentStyle {
                                    color: green,
                                    ..FragmentStyle::default()
                                },
                            );
                        } else {
                            body.add_text(
                                "[OFF]",
                                FragmentStyle {
                                    color: red,
                                    ..FragmentStyle::default()
                                },
                            );
                        }
                    }
                    SettingValue::String(s) if item.key == "window.background-image" => {
                        // Image path with picker hint
                        if s.is_empty() {
                            body.add_text(
                                "[Choose Image...]",
                                FragmentStyle {
                                    color: if is_selected { accent } else { dim },
                                    ..FragmentStyle::default()
                                },
                            );
                        } else {
                            // Show filename only (not full path) with image icon
                            let filename = std::path::Path::new(s.as_str())
                                .file_name()
                                .and_then(|f| f.to_str())
                                .unwrap_or(s);
                            body.add_text(
                                filename,
                                FragmentStyle {
                                    color: if is_selected { accent } else { white },
                                    ..FragmentStyle::default()
                                },
                            );
                        }
                        if is_selected {
                            body.add_text(
                                "  [Enter to browse]",
                                FragmentStyle {
                                    color: dim,
                                    ..FragmentStyle::default()
                                },
                            );
                        }
                    }
                    SettingValue::String(s) if looks_like_color(s) => {
                        // Show colored square next to hex code
                        if let Some(parsed) = parse_hex_color(s) {
                            body.add_text(
                                "\u{25A0} ",
                                FragmentStyle {
                                    color: parsed,
                                    ..FragmentStyle::default()
                                },
                            );
                        }
                        body.add_text(
                            s,
                            FragmentStyle {
                                color: if is_selected { accent } else { white },
                                ..FragmentStyle::default()
                            },
                        );
                    }
                    SettingValue::Float(_) | SettingValue::Integer(_) => {
                        let val_display = item.value.display();
                        body.add_text(
                            &val_display,
                            FragmentStyle {
                                color: if is_selected { accent } else { white },
                                ..FragmentStyle::default()
                            },
                        );
                        if is_selected {
                            body.add_text(
                                "  [+/-]",
                                FragmentStyle {
                                    color: dim,
                                    ..FragmentStyle::default()
                                },
                            );
                        }
                    }
                    _ => {
                        let val_display = item.value.display();
                        body.add_text(
                            &val_display,
                            FragmentStyle {
                                color: if is_selected { accent } else { white },
                                ..FragmentStyle::default()
                            },
                        );
                    }
                }
            }

            body.new_line();

            // Description below selected item (always shown in dim)
            if is_selected && !editor.editing {
                body.add_text(
                    &format!("    {}", item.description),
                    FragmentStyle {
                        color: dim,
                        ..FragmentStyle::default()
                    },
                );
                body.new_line();
            } else {
                // Empty line for spacing consistency
                body.new_line();
            }
        }

        body.build();
    }

    objects.push(Object::RichText(RichText {
        id: panel_rt,
        position: [panel_x, items_start_y],
        lines: None,
    }));

    // --- Footer with keyboard hints ---
    let footer_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&footer_rt, 11.0);

    let key_bg_style = FragmentStyle {
        color: black,
        background_color: Some(highlight),
        ..FragmentStyle::default()
    };

    {
        let content = sugarloaf.content();
        let footer = content.sel(footer_rt).clear();

        if editor.editing {
            footer
                .add_text(" Enter ", key_bg_style)
                .add_text(" confirm  ", dim_style())
                .add_text(" Escape ", key_bg_style)
                .add_text(" cancel", dim_style());
        } else {
            footer
                .add_text(" Tab ", key_bg_style)
                .add_text(" switch panel  ", dim_style())
                .add_text(" Up/Down ", key_bg_style)
                .add_text(" navigate  ", dim_style())
                .add_text(" Enter ", key_bg_style)
                .add_text(" edit  ", dim_style())
                .add_text(" / ", key_bg_style)
                .add_text(" search  ", dim_style())
                .add_text(" i ", key_bg_style)
                .add_text(" import  ", dim_style())
                .add_text(" Esc ", key_bg_style)
                .add_text(" close", dim_style());
        }

        footer.build();
    }

    // Footer background bar
    let footer_y = full_h - 28.0;
    objects.push(Object::Quad(Quad {
        position: [0., footer_y - 4.0],
        color: sidebar_bg,
        size: [full_w, 28.0],
        ..Quad::default()
    }));

    objects.push(Object::RichText(RichText {
        id: footer_rt,
        position: [155., footer_y + 4.0],
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

/// Check if a string looks like a hex color (#RRGGBB or #RGB).
fn looks_like_color(s: &str) -> bool {
    if let Some(rest) = s.strip_prefix('#') {
        let len = rest.len();
        (len == 6 || len == 3) && rest.chars().all(|c| c.is_ascii_hexdigit())
    } else {
        false
    }
}

/// Parse a hex color string like "#RRGGBB" into [f32; 4].
fn parse_hex_color(s: &str) -> Option<[f32; 4]> {
    let hex = s.strip_prefix('#')?;
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0;
        Some([r, g, b, 1.0])
    } else if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1], 16).ok()? as f32 / 15.0;
        let g = u8::from_str_radix(&hex[1..2], 16).ok()? as f32 / 15.0;
        let b = u8::from_str_radix(&hex[2..3], 16).ok()? as f32 / 15.0;
        Some([r, g, b, 1.0])
    } else {
        None
    }
}
