use crate::context::grid::ContextDimension;
use rio_backend::config::Config;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

/// Convert an RGBA f32 color array to a hex string like "#RRGGBB".
fn color_to_hex(c: &[f32; 4]) -> String {
    let r = (c[0] * 255.0) as u8;
    let g = (c[1] * 255.0) as u8;
    let b = (c[2] * 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

#[inline]
pub fn screen(
    sugarloaf: &mut Sugarloaf,
    context_dimension: &ContextDimension,
    config: &Config,
) {
    let accent = [0.1764706, 0.6039216, 1.0, 1.0]; // blue
    let dim = [0.5, 0.5, 0.5, 1.0]; // gray for labels
    let highlight = [0.9882353, 0.7294118, 0.15686275, 1.0]; // yellow
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];

    let layout = sugarloaf.window_size();

    let mut objects = Vec::with_capacity(16);

    // Full-screen black background
    objects.push(Object::Quad(Quad {
        position: [0., 0.0],
        color: black,
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
    sugarloaf.set_rich_text_font_size(&title_rt, 28.0);

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
        position: [40., context_dimension.margin.top_y + 30.],
        lines: None,
    }));

    // --- Subtitle ---
    let subtitle_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&subtitle_rt, 14.0);

    let config_path = rio_backend::config::config_file_path();
    let content = sugarloaf.content();
    content
        .sel(subtitle_rt)
        .clear()
        .add_text(
            &format!("Edit {} to change settings", config_path.display()),
            FragmentStyle {
                color: dim,
                ..FragmentStyle::default()
            },
        )
        .build();

    objects.push(Object::RichText(RichText {
        id: subtitle_rt,
        position: [40., context_dimension.margin.top_y + 68.],
        lines: None,
    }));

    // --- Settings body ---
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 14.0);

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

    let content = sugarloaf.content();
    let body = content.sel(body_rt).clear();

    // ── Font ──
    body.add_text("FONT", category_style).new_line();

    let family_name = config
        .fonts
        .family
        .as_deref()
        .unwrap_or(&config.fonts.regular.family);
    body.add_text("  Family:      ", label_style)
        .add_text(family_name, value_style)
        .new_line();

    body.add_text("  Size:        ", label_style)
        .add_text(&format!("{:.1}", config.fonts.size), value_style)
        .new_line();

    body.add_text("  Line height: ", label_style)
        .add_text(&format!("{:.1}", config.line_height), value_style)
        .new_line();

    body.add_text("", label_style).new_line();

    // ── Window ──
    body.add_text("WINDOW", category_style).new_line();

    body.add_text("  Opacity:     ", label_style)
        .add_text(&format!("{:.2}", config.window.opacity), value_style)
        .new_line();

    let bg_image = config
        .window
        .background_image
        .as_ref()
        .map(|_| "configured")
        .unwrap_or("none");
    body.add_text("  Background:  ", label_style)
        .add_text(bg_image, value_style)
        .new_line();

    body.add_text("", label_style).new_line();

    // ── Navigation ──
    body.add_text("NAVIGATION", category_style).new_line();

    body.add_text("  Mode:        ", label_style)
        .add_text(&format!("{:?}", config.navigation.mode), value_style)
        .new_line();

    body.add_text("  Hide single: ", label_style)
        .add_text(
            if config.navigation.hide_if_single {
                "yes"
            } else {
                "no"
            },
            value_style,
        )
        .new_line();

    body.add_text("", label_style).new_line();

    // ── Colors ──
    body.add_text("COLORS", category_style).new_line();

    let bg_hex = color_to_hex(&config.colors.background.0);
    body.add_text("  Background:  ", label_style)
        .add_text(&bg_hex, value_style)
        .new_line();

    let fg_hex = color_to_hex(&config.colors.foreground);
    body.add_text("  Foreground:  ", label_style)
        .add_text(&fg_hex, value_style)
        .new_line();

    let cursor_hex = color_to_hex(&config.colors.cursor);
    body.add_text("  Cursor:      ", label_style)
        .add_text(&cursor_hex, value_style)
        .new_line();

    body.add_text("", label_style).new_line();

    // ── Shell ──
    body.add_text("SHELL", category_style).new_line();

    let program = if config.shell.program.is_empty() {
        "(default)"
    } else {
        &config.shell.program
    };
    body.add_text("  Program:     ", label_style)
        .add_text(program, value_style)
        .new_line();

    let args = if config.shell.args.is_empty() {
        String::from("(none)")
    } else {
        config.shell.args.join(" ")
    };
    body.add_text("  Args:        ", label_style)
        .add_text(&args, value_style)
        .new_line();

    let working_dir = config.working_dir.as_deref().unwrap_or("(default)");
    body.add_text("  Working dir: ", label_style)
        .add_text(working_dir, value_style)
        .new_line();

    body.add_text("", label_style).new_line();

    // ── Developer ──
    body.add_text("DEVELOPER", category_style).new_line();

    body.add_text("  Log level:   ", label_style)
        .add_text(&config.developer.log_level, value_style)
        .new_line();

    body.add_text("  Log file:    ", label_style)
        .add_text(
            if config.developer.enable_log_file {
                "enabled"
            } else {
                "disabled"
            },
            value_style,
        )
        .new_line();

    body.add_text("", label_style).new_line();
    body.add_text("", label_style).new_line();

    // ── Footer ──
    body.add_text(" Escape ", key_bg_style)
        .add_text(" close", dim_style());

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 100.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}

fn dim_style() -> FragmentStyle {
    FragmentStyle {
        color: [0.5, 0.5, 0.5, 1.0],
        ..FragmentStyle::default()
    }
}
