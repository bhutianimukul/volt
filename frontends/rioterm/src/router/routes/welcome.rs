use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(sugarloaf: &mut Sugarloaf, context_dimension: &ContextDimension) {
    // Volt brand palette
    let volt_cyan = [0.0, 0.898, 1.0, 1.0]; // #00E5FF — electric cyan
    let volt_blue = [0.153, 0.380, 1.0, 1.0]; // #2761FF — deep electric blue
    let volt_accent = [0.678, 0.847, 1.0, 1.0]; // #ADD8FF — soft highlight
    let bg_dark = [0.059, 0.063, 0.082, 1.0]; // #0F1015 — near-black
    let stripe_dim = [0.118, 0.129, 0.173, 1.0]; // #1E212C — subtle stripe

    let layout = sugarloaf.window_size();
    let w = layout.width / context_dimension.dimension.scale;
    let h = layout.height / context_dimension.dimension.scale;

    let mut objects = Vec::with_capacity(12);

    // Full-screen dark background
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: bg_dark,
        size: [w, h],
        ..Quad::default()
    }));

    // Left accent bar — single bright cyan stripe
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: volt_cyan,
        size: [3., h],
        ..Quad::default()
    }));

    // Subtle horizontal stripe behind the heading area
    objects.push(Object::Quad(Quad {
        position: [0., context_dimension.margin.top_y + 18.],
        color: stripe_dim,
        size: [w, 48.],
        ..Quad::default()
    }));

    // ── Text blocks ──────────────────────────────────────────

    let bolt_icon = sugarloaf.create_temp_rich_text();
    let heading = sugarloaf.create_temp_rich_text();
    let paragraph_action = sugarloaf.create_temp_rich_text();
    let paragraph = sugarloaf.create_temp_rich_text();
    let footer = sugarloaf.create_temp_rich_text();

    sugarloaf.set_rich_text_font_size(&bolt_icon, 36.0);
    sugarloaf.set_rich_text_font_size(&heading, 28.0);
    sugarloaf.set_rich_text_font_size(&paragraph_action, 18.0);
    sugarloaf.set_rich_text_font_size(&paragraph, 15.0);
    sugarloaf.set_rich_text_font_size(&footer, 13.0);

    let content = sugarloaf.content();

    // Lightning bolt icon
    let bolt_line = content.sel(bolt_icon);
    bolt_line
        .clear()
        .add_text(
            "\u{26A1}",
            FragmentStyle {
                color: volt_cyan,
                ..FragmentStyle::default()
            },
        )
        .build();

    // Heading
    let heading_line = content.sel(heading);
    heading_line
        .clear()
        .add_text(
            "Volt",
            FragmentStyle {
                color: volt_cyan,
                ..FragmentStyle::default()
            },
        )
        .add_text(
            " Terminal",
            FragmentStyle {
                color: [0.85, 0.87, 0.91, 1.0], // light gray
                ..FragmentStyle::default()
            },
        )
        .build();

    // Action prompt
    let action_line = content.sel(paragraph_action);
    action_line
        .clear()
        .add_text(
            "\u{25B8} press ",
            FragmentStyle {
                color: [0.5, 0.53, 0.6, 1.0], // muted
                ..FragmentStyle::default()
            },
        )
        .add_text(
            "enter",
            FragmentStyle {
                color: volt_cyan,
                ..FragmentStyle::default()
            },
        )
        .add_text(
            " to get started",
            FragmentStyle {
                color: [0.5, 0.53, 0.6, 1.0],
                ..FragmentStyle::default()
            },
        )
        .build();

    // Info paragraph
    #[cfg(target_os = "macos")]
    let shortcut = "\u{2318} + ,";

    #[cfg(not(target_os = "macos"))]
    let shortcut = "Ctrl + Shift + ,";

    let paragraph_line = content.sel(paragraph);
    paragraph_line
        .clear()
        .add_text(
            "Config  ",
            FragmentStyle {
                color: [0.5, 0.53, 0.6, 1.0],
                ..FragmentStyle::default()
            },
        )
        .add_text(
            &format!(" {} ", rio_backend::config::config_file_path().display()),
            FragmentStyle {
                background_color: Some(volt_blue),
                color: [0.95, 0.96, 0.98, 1.0],
                ..FragmentStyle::default()
            },
        )
        .new_line()
        .add_text("", FragmentStyle::default())
        .new_line()
        .add_text(
            "Settings  ",
            FragmentStyle {
                color: [0.5, 0.53, 0.6, 1.0],
                ..FragmentStyle::default()
            },
        )
        .add_text(
            &format!(" {shortcut} "),
            FragmentStyle {
                background_color: Some(volt_blue),
                color: [0.95, 0.96, 0.98, 1.0],
                ..FragmentStyle::default()
            },
        )
        .build();

    // Footer
    let footer_line = content.sel(footer);
    footer_line
        .clear()
        .add_text(
            "GPU-accelerated \u{00B7} Rust-powered",
            FragmentStyle {
                color: [0.35, 0.37, 0.42, 1.0],
                ..FragmentStyle::default()
            },
        )
        .build();

    // ── Layout positions ─────────────────────────────────────

    objects.push(Object::RichText(RichText {
        id: bolt_icon,
        position: [28., context_dimension.margin.top_y + 26.],
        lines: None,
    }));

    objects.push(Object::RichText(RichText {
        id: heading,
        position: [72., context_dimension.margin.top_y + 30.],
        lines: None,
    }));

    objects.push(Object::RichText(RichText {
        id: paragraph_action,
        position: [28., context_dimension.margin.top_y + 88.],
        lines: None,
    }));

    objects.push(Object::RichText(RichText {
        id: paragraph,
        position: [28., context_dimension.margin.top_y + 148.],
        lines: None,
    }));

    objects.push(Object::RichText(RichText {
        id: footer,
        position: [28., context_dimension.margin.top_y + 240.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}
