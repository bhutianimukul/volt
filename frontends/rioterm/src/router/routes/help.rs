use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

/// Help categories
pub const HELP_CATEGORIES: &[&str] = &["Shortcuts", "Features", "Actions", "Commands"];

/// Number of items in each category (for navigation bounds)
pub fn category_item_count(category: usize) -> usize {
    match category {
        0 => 5,  // Shortcut groups: Tabs, Splits, Navigation, Features, Safety
        1 => 11, // Features list
        2 => 11, // Actions list
        3 => {
            // Slash commands count (one per command)
            crate::slash_commands::all_commands().len()
        }
        _ => 0,
    }
}

/// Map action index to the RoutePath that should be opened
pub fn action_route(index: usize) -> Option<&'static str> {
    match index {
        0 => Some("ai"),
        1 => Some("history"),
        2 => Some("env"),
        3 => Some("bookmarks"),
        4 => Some("connections"),
        5 => Some("tmux"),
        6 => Some("slash"),
        7 => Some("layouts"),
        8 => Some("sharing"),
        9 => Some("timetravel"),
        10 => Some("export"),
        _ => None,
    }
}

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    selected_category: usize,
    selected_item: usize,
    in_sidebar: bool,
) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.2, 0.5, 1.0, 1.0];
    let dim = [0.45, 0.45, 0.5, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let sidebar_bg = [0.08, 0.08, 0.11, 1.0];
    let sidebar_selected = [0.15, 0.15, 0.22, 1.0];
    let sidebar_hover = [0.10, 0.10, 0.14, 1.0];
    let divider_color = [0.15, 0.15, 0.2, 1.0];
    let key_color = [0.4, 0.8, 1.0, 1.0];
    let green = [0.3, 0.85, 0.4, 1.0];

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

    // Sidebar background
    let sidebar_width = 140.0;
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: sidebar_bg,
        size: [sidebar_width, full_h],
        ..Quad::default()
    }));

    // Vertical divider
    objects.push(Object::Quad(Quad {
        position: [sidebar_width, 0.0],
        color: divider_color,
        size: [1.0, full_h],
        ..Quad::default()
    }));

    // --- Title ---
    let title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&title_rt, 22.0);
    {
        let content = sugarloaf.content();
        content
            .sel(title_rt)
            .clear()
            .add_text(
                "Volt Help",
                FragmentStyle {
                    color: white,
                    ..FragmentStyle::default()
                },
            )
            .build();
    }
    objects.push(Object::RichText(RichText {
        id: title_rt,
        position: [155., context_dimension.margin.top_y + 25.],
        lines: None,
    }));

    // --- Subtitle ---
    let subtitle_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&subtitle_rt, 13.0);
    {
        let content = sugarloaf.content();
        content
            .sel(subtitle_rt)
            .clear()
            .add_text(
                match selected_category {
                    0 => "Keyboard shortcuts and actions",
                    1 => "Feature overview and shortcuts",
                    2 => "Quick launch — press Enter to open",
                    3 => "Type / at the prompt to use",
                    _ => "",
                },
                FragmentStyle {
                    color: dim,
                    ..FragmentStyle::default()
                },
            )
            .build();
    }
    objects.push(Object::RichText(RichText {
        id: subtitle_rt,
        position: [155., context_dimension.margin.top_y + 55.],
        lines: None,
    }));

    // --- Sidebar title ---
    let sidebar_title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&sidebar_title_rt, 11.0);
    {
        let content = sugarloaf.content();
        content
            .sel(sidebar_title_rt)
            .clear()
            .add_text(
                "HELP",
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

    // --- Sidebar categories ---
    let cat_start_y = context_dimension.margin.top_y + 48.0;
    let cat_line_height = 22.0;

    let icons = ["## ", "** ", "-> ", "// "];

    for (i, _cat) in HELP_CATEGORIES.iter().enumerate() {
        let cat_y = cat_start_y + (i as f32 * cat_line_height);
        let is_selected = i == selected_category;

        if is_selected {
            let bg_color = if in_sidebar {
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

    let sidebar_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&sidebar_rt, 13.0);
    {
        let content = sugarloaf.content();
        let body = content.sel(sidebar_rt).clear();

        for (i, cat) in HELP_CATEGORIES.iter().enumerate() {
            let is_selected = i == selected_category;

            let style = if is_selected && in_sidebar {
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

            body.add_text(
                icons[i],
                FragmentStyle {
                    color: if is_selected && in_sidebar {
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

    // --- Panel content (right side) ---
    let panel_x = 155.0;
    let header_y = context_dimension.margin.top_y + 80.0;

    // Category header with accent underline
    let header_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&header_rt, 14.0);
    {
        let content = sugarloaf.content();
        content
            .sel(header_rt)
            .clear()
            .add_text(
                HELP_CATEGORIES[selected_category],
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            )
            .build();
    }
    objects.push(Object::RichText(RichText {
        id: header_rt,
        position: [panel_x, header_y],
        lines: None,
    }));

    let items_start_y = header_y + 30.0;

    let panel_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&panel_rt, 12.0);

    let key_style = FragmentStyle {
        color: key_color,
        ..FragmentStyle::default()
    };
    let desc_style = FragmentStyle {
        color: white,
        ..FragmentStyle::default()
    };
    let section_style = FragmentStyle {
        color: highlight,
        ..FragmentStyle::default()
    };
    let dim_style = FragmentStyle {
        color: dim,
        ..FragmentStyle::default()
    };

    {
        let content = sugarloaf.content();
        let body = content.sel(panel_rt).clear();

        match selected_category {
            0 => render_shortcuts(
                body,
                selected_item,
                in_sidebar,
                key_style,
                desc_style,
                section_style,
                dim_style,
                white,
                accent,
            ),
            1 => render_features(
                body,
                selected_item,
                in_sidebar,
                key_style,
                desc_style,
                dim_style,
                white,
                accent,
                green,
            ),
            2 => render_actions(
                body,
                selected_item,
                in_sidebar,
                desc_style,
                dim_style,
                white,
                accent,
                green,
            ),
            3 => render_slash_commands(
                body,
                selected_item,
                in_sidebar,
                key_style,
                desc_style,
                dim_style,
                white,
                accent,
            ),
            _ => {}
        }

        body.build();
    }

    objects.push(Object::RichText(RichText {
        id: panel_rt,
        position: [panel_x, items_start_y],
        lines: None,
    }));

    // --- Footer ---
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

        footer
            .add_text(" Tab ", key_bg_style)
            .add_text(" switch panel  ", dim_style)
            .add_text(" Up/Down ", key_bg_style)
            .add_text(" navigate  ", dim_style);

        if selected_category == 2 && !in_sidebar {
            footer
                .add_text(" Enter ", key_bg_style)
                .add_text(" open  ", dim_style);
        }

        footer
            .add_text(" Esc ", key_bg_style)
            .add_text(" close", dim_style);

        footer.build();
    }

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

// Use a trait object to avoid generic type issues with the content builder
fn render_shortcuts(
    body: &mut rio_backend::sugarloaf::Content,
    selected_item: usize,
    in_sidebar: bool,
    key_style: FragmentStyle,
    desc_style: FragmentStyle,
    section_style: FragmentStyle,
    dim_style: FragmentStyle,
    white: [f32; 4],
    accent: [f32; 4],
) {
    let groups: &[(&str, &[(&str, &str, &str)])] = &[
        (
            "TABS",
            &[
                ("Cmd+T", ".............", "New tab"),
                ("Cmd+W", ".............", "Close tab/split"),
                ("Cmd+1-9", "...........", "Jump to tab N"),
                ("Cmd+Shift+]", ".......", "Next tab"),
                ("Cmd+Shift+[", ".......", "Previous tab"),
                ("Cmd+Shift+R", ".......", "Rename tab"),
                ("Double-click", "......", "Rename tab (mouse)"),
                ("Click tab", ".........", "Switch to tab"),
            ],
        ),
        (
            "SPLITS",
            &[
                ("Cmd+D", ".............", "Split right"),
                ("Cmd+Shift+Enter", "..", "Zoom pane"),
                ("Cmd+Shift+B", ".......", "Broadcast to all panes"),
                ("Ctrl+Cmd+Arrow", "....", "Resize pane"),
                ("Drag divider", "......", "Resize pane (mouse)"),
            ],
        ),
        (
            "NAVIGATION",
            &[
                ("Cmd+Up", "............", "Previous command block"),
                ("Cmd+Down", "..........", "Next command block"),
                ("Cmd+K", ".............", "Clear scrollback"),
                ("Cmd+L", ".............", "Clear screen"),
            ],
        ),
        (
            "FEATURES",
            &[
                ("Cmd+,", ".............", "Settings viewer"),
                ("Cmd+?", ".............", "This help screen"),
                ("Cmd+Shift+H", ".......", "Session history"),
                ("Cmd+Shift+E", ".......", "Environment viewer"),
                ("Cmd+Shift+K", ".......", "Bookmarks"),
                ("Cmd+Shift+I", ".......", "AI Assistant"),
                ("Cmd+Shift+P", ".......", "Slash commands"),
                ("Cmd+Shift+L", ".......", "Layout presets"),
                ("Cmd+=/- /0", "........", "Zoom in/out/reset"),
                ("Cmd+Enter", ".........", "Toggle fullscreen"),
                ("Cmd+N", ".............", "New window"),
            ],
        ),
        (
            "SAFETY",
            &[
                (
                    "Auto-detect",
                    ".......",
                    "Warns on rm -rf, git push -f, etc.",
                ),
                ("!command", "..........", "Bypass safety check"),
            ],
        ),
    ];

    for (gi, (group_name, shortcuts)) in groups.iter().enumerate() {
        let is_group_selected = !in_sidebar && gi == selected_item;

        if gi > 0 {
            body.new_line();
        }

        let s = if is_group_selected {
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            }
        } else {
            section_style
        };

        if is_group_selected {
            body.add_text(
                "> ",
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            );
        } else {
            body.add_text("  ", dim_style);
        }

        body.add_text(group_name, s).new_line();

        for (key, dots, desc) in shortcuts.iter() {
            body.add_text("    ", dim_style);
            body.add_text(key, key_style)
                .add_text(&format!(" {} ", dots), dim_style)
                .add_text(desc, desc_style)
                .new_line();
        }
    }
}

fn render_features(
    body: &mut rio_backend::sugarloaf::Content,
    selected_item: usize,
    in_sidebar: bool,
    key_style: FragmentStyle,
    _desc_style: FragmentStyle,
    dim_style: FragmentStyle,
    white: [f32; 4],
    accent: [f32; 4],
    _green: [f32; 4],
) {
    let features: &[(&str, &str, &str)] = &[
        (
            "AI Assistant",
            "Cmd+Shift+I",
            "Opens Claude Code in a split pane",
        ),
        (
            "Session History",
            "Cmd+Shift+H",
            "View and search recorded commands. Enter to copy.",
        ),
        (
            "Environment",
            "Cmd+Shift+E",
            "Inspect environment variables grouped by category",
        ),
        (
            "Bookmarks",
            "Cmd+Shift+K",
            "Save and recall important commands",
        ),
        (
            "Connections",
            "",
            "Manage SSH, MySQL, Redis, Kubernetes connections",
        ),
        ("Tmux", "", "Attach, detach, create tmux sessions"),
        (
            "Slash Commands",
            "Cmd+Shift+P",
            "20 built-in terminal commands",
        ),
        (
            "Layout Presets",
            "Cmd+Shift+L",
            "Predefined pane arrangements",
        ),
        (
            "Session Sharing",
            "Cmd+Shift+S",
            "Share terminal over network (host or connect)",
        ),
        (
            "Time Travel",
            "Cmd+Shift+T",
            "Browse command timeline with detail view",
        ),
        ("Session Export", "", "Export terminal session to file"),
    ];

    for (i, (name, shortcut, description)) in features.iter().enumerate() {
        let is_selected = !in_sidebar && i == selected_item;

        if is_selected {
            body.add_text(
                "> ",
                FragmentStyle {
                    color: accent,
                    ..FragmentStyle::default()
                },
            );
        } else {
            body.add_text("  ", dim_style);
        }

        let name_style = if is_selected {
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            }
        } else {
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            }
        };

        body.add_text(name, name_style);

        if !shortcut.is_empty() {
            body.add_text(&format!("  ({})", shortcut), key_style);
        }

        body.new_line();

        body.add_text(&format!("    {}", description), dim_style);
        body.new_line();

        if is_selected {
            body.new_line();
        }
    }
}

fn render_actions(
    body: &mut rio_backend::sugarloaf::Content,
    selected_item: usize,
    in_sidebar: bool,
    _desc_style: FragmentStyle,
    dim_style: FragmentStyle,
    white: [f32; 4],
    accent: [f32; 4],
    green: [f32; 4],
) {
    let actions: &[(&str, &str)] = &[
        ("AI Assistant", "Open Claude Code in a split pane"),
        ("Session History", "Browse and search command history"),
        ("Environment", "Inspect environment variables"),
        ("Bookmarks", "Manage saved commands"),
        ("Connections", "Open connections manager"),
        ("Tmux", "Open tmux session picker"),
        ("Slash Commands", "Open slash command palette"),
        ("Layout Presets", "Apply a layout preset"),
        ("Session Sharing", "Share terminal over network"),
        ("Time Travel", "Browse command timeline"),
        ("Session Export", "Export current session"),
    ];

    for (i, (name, description)) in actions.iter().enumerate() {
        let is_selected = !in_sidebar && i == selected_item;

        if is_selected {
            body.add_text(
                "> ",
                FragmentStyle {
                    color: green,
                    ..FragmentStyle::default()
                },
            );
        } else {
            body.add_text("  ", dim_style);
        }

        let name_style = if is_selected {
            FragmentStyle {
                color: white,
                ..FragmentStyle::default()
            }
        } else {
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            }
        };

        body.add_text(name, name_style);
        body.add_text(&format!("  {}", description), dim_style);
        body.new_line();

        if is_selected {
            body.add_text(
                "    Press Enter to open",
                FragmentStyle {
                    color: green,
                    ..FragmentStyle::default()
                },
            );
            body.new_line();
        }
    }
}

fn render_slash_commands(
    body: &mut rio_backend::sugarloaf::Content,
    selected_item: usize,
    in_sidebar: bool,
    key_style: FragmentStyle,
    desc_style: FragmentStyle,
    dim_style: FragmentStyle,
    white: [f32; 4],
    accent: [f32; 4],
) {
    use crate::slash_commands::{all_commands, CommandCategory};

    let commands = all_commands();
    let categories = [
        CommandCategory::Navigation,
        CommandCategory::Appearance,
        CommandCategory::Tools,
        CommandCategory::Session,
        CommandCategory::Debug,
    ];

    let mut flat_idx: usize = 0;
    for category in &categories {
        let cat_cmds: Vec<_> = commands
            .iter()
            .filter(|c| c.category == *category)
            .collect();
        if cat_cmds.is_empty() {
            continue;
        }

        body.add_text(
            &category.name().to_uppercase(),
            FragmentStyle {
                color: accent,
                ..FragmentStyle::default()
            },
        );
        body.new_line();

        for cmd in &cat_cmds {
            let is_selected = !in_sidebar && flat_idx == selected_item;

            if is_selected {
                body.add_text(
                    "> ",
                    FragmentStyle {
                        color: accent,
                        ..FragmentStyle::default()
                    },
                );
            } else {
                body.add_text("  ", dim_style);
            }

            let name_style = if is_selected {
                FragmentStyle {
                    color: white,
                    ..FragmentStyle::default()
                }
            } else {
                key_style
            };

            body.add_text(&format!("/{}", cmd.name), name_style);

            // Padding dots
            let pad_len = 16usize.saturating_sub(cmd.name.len() + 1);
            let dots: String = " .".repeat(pad_len / 2);
            body.add_text(&dots, dim_style);
            body.add_text(" ", dim_style);

            body.add_text(cmd.description, desc_style);
            body.new_line();

            if is_selected {
                body.add_text(&format!("    {}", cmd.usage), dim_style);
                body.new_line();
            }

            flat_idx += 1;
        }

        body.new_line();
    }
}
