use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(sugarloaf: &mut Sugarloaf, context_dimension: &ContextDimension) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.98, 0.73, 0.16, 1.0]; // yellow/gold (Volt brand)
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0]; // yellow for key badges
    let key_color = [0.4, 0.8, 1.0, 1.0]; // light blue for shortcut keys

    let layout = sugarloaf.window_size();
    let mut objects = Vec::with_capacity(16);

    // Background
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: bg,
        size: [
            layout.width / context_dimension.dimension.scale,
            layout.height / context_dimension.dimension.scale,
        ],
        ..Quad::default()
    }));

    // Accent bar on the left
    objects.push(Object::Quad(Quad {
        position: [0., 30.0],
        color: accent,
        size: [4., layout.height / context_dimension.dimension.scale],
        ..Quad::default()
    }));

    // Title
    let title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&title_rt, 22.0);
    let content = sugarloaf.content();
    content
        .sel(title_rt)
        .clear()
        .add_text(
            "Volt Keyboard Shortcuts",
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
    let content = sugarloaf.content();
    content
        .sel(subtitle_rt)
        .clear()
        .add_text(
            "All available keyboard shortcuts and actions",
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

    // Body with all shortcuts
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 13.0);

    let key_style = FragmentStyle {
        color: key_color,
        ..FragmentStyle::default()
    };
    let desc_style = FragmentStyle {
        color: white,
        ..FragmentStyle::default()
    };
    let header_style = FragmentStyle {
        color: accent,
        ..FragmentStyle::default()
    };
    let dim_style = FragmentStyle {
        color: dim,
        ..FragmentStyle::default()
    };
    let key_bg_style = FragmentStyle {
        background_color: Some(highlight),
        color: black,
        ..FragmentStyle::default()
    };

    let content = sugarloaf.content();
    let body = content.sel(body_rt);
    body.clear();

    // --- TABS ---
    body.add_text("TABS", header_style).new_line();
    body.add_text("  Cmd+T", key_style).add_text(" ............. ", dim_style).add_text("New tab", desc_style).new_line();
    body.add_text("  Cmd+W", key_style).add_text(" ............. ", dim_style).add_text("Close tab/split", desc_style).new_line();
    body.add_text("  Cmd+1-9", key_style).add_text(" ........... ", dim_style).add_text("Jump to tab N", desc_style).new_line();
    body.add_text("  Cmd+Shift+]", key_style).add_text(" ....... ", dim_style).add_text("Next tab", desc_style).new_line();
    body.add_text("  Cmd+Shift+[", key_style).add_text(" ....... ", dim_style).add_text("Previous tab", desc_style).new_line();
    body.add_text("  Cmd+Shift+R", key_style).add_text(" ....... ", dim_style).add_text("Rename tab", desc_style).new_line();
    body.add_text("  Double-click", key_style).add_text(" ...... ", dim_style).add_text("Rename tab (mouse)", desc_style).new_line();
    body.add_text("  Click tab", key_style).add_text(" ......... ", dim_style).add_text("Switch to tab", desc_style).new_line();

    // --- SPLITS ---
    body.new_line().add_text("SPLITS", header_style).new_line();
    body.add_text("  Cmd+D", key_style).add_text(" ............. ", dim_style).add_text("Split right", desc_style).new_line();
    body.add_text("  Cmd+Shift+Enter", key_style).add_text(" .. ", dim_style).add_text("Zoom pane", desc_style).new_line();
    body.add_text("  Cmd+Shift+B", key_style).add_text(" ....... ", dim_style).add_text("Broadcast to all panes", desc_style).new_line();
    body.add_text("  Ctrl+Cmd+Arrow", key_style).add_text(" .... ", dim_style).add_text("Resize pane", desc_style).new_line();
    body.add_text("  Drag divider", key_style).add_text(" ...... ", dim_style).add_text("Resize pane (mouse)", desc_style).new_line();

    // --- NAVIGATION ---
    body.new_line().add_text("NAVIGATION", header_style).new_line();
    body.add_text("  Cmd+Up", key_style).add_text(" ............ ", dim_style).add_text("Previous command block", desc_style).new_line();
    body.add_text("  Cmd+Down", key_style).add_text(" .......... ", dim_style).add_text("Next command block", desc_style).new_line();
    body.add_text("  Cmd+K", key_style).add_text(" ............. ", dim_style).add_text("Clear scrollback", desc_style).new_line();
    body.add_text("  Cmd+L", key_style).add_text(" ............. ", dim_style).add_text("Clear screen", desc_style).new_line();

    // --- FEATURES ---
    body.new_line().add_text("FEATURES", header_style).new_line();
    body.add_text("  Cmd+,", key_style).add_text(" ............. ", dim_style).add_text("Settings viewer", desc_style).new_line();
    body.add_text("  Cmd+?", key_style).add_text(" ............. ", dim_style).add_text("This help screen", desc_style).new_line();
    body.add_text("  Cmd+Shift+H", key_style).add_text(" ....... ", dim_style).add_text("Session history", desc_style).new_line();
    body.add_text("  Cmd+Shift+E", key_style).add_text(" ....... ", dim_style).add_text("Environment viewer", desc_style).new_line();
    body.add_text("  Cmd+Shift+K", key_style).add_text(" ....... ", dim_style).add_text("Bookmarks", desc_style).new_line();
    body.add_text("  Cmd+=/-/0", key_style).add_text(" ......... ", dim_style).add_text("Zoom in/out/reset", desc_style).new_line();
    body.add_text("  Cmd+Enter", key_style).add_text(" ......... ", dim_style).add_text("Toggle fullscreen", desc_style).new_line();
    body.add_text("  Cmd+N", key_style).add_text(" ............. ", dim_style).add_text("New window", desc_style).new_line();

    // --- SAFETY ---
    body.new_line().add_text("SAFETY", header_style).new_line();
    body.add_text("  Auto-detect", key_style).add_text(" ....... ", dim_style).add_text("Warns on rm -rf, git push -f, etc.", desc_style).new_line();
    body.add_text("  !command", key_style).add_text(" .......... ", dim_style).add_text("Bypass safety check", desc_style).new_line();

    // Footer
    body.new_line().new_line();
    body.add_text(" Escape ", key_bg_style)
        .add_text(" close", dim_style);

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 85.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}
