use crate::context::grid::ContextDimension;
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};

#[inline]
pub fn screen(sugarloaf: &mut Sugarloaf, context_dimension: &ContextDimension) {
    let bg = [0.06, 0.06, 0.08, 1.0];
    let accent = [0.3, 0.8, 0.9, 1.0]; // cyan
    let dim = [0.45, 0.45, 0.5, 1.0];
    let black = [0.0, 0.0, 0.0, 1.0];
    let white = [1.0, 1.0, 1.0, 1.0];
    let highlight = [0.98, 0.73, 0.16, 1.0]; // yellow for key badges
    let key_color = [0.4, 0.8, 1.0, 1.0]; // light blue for var names
    let secret_color = [0.9, 0.3, 0.3, 1.0]; // red for masked secrets

    let layout = sugarloaf.window_size();
    let mut objects = Vec::with_capacity(16);

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

    // Title
    let title_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&title_rt, 22.0);
    let content = sugarloaf.content();
    content
        .sel(title_rt)
        .clear()
        .add_text(
            "Environment Variables",
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

    // Subtitle with total count
    let subtitle_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&subtitle_rt, 13.0);
    let total_count: usize = std::env::vars().count();
    let content = sugarloaf.content();
    content
        .sel(subtitle_rt)
        .clear()
        .add_text(
            &format!("{} variables  |  Grouped by category", total_count),
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

    // Body with grouped env vars
    let body_rt = sugarloaf.create_temp_rich_text();
    sugarloaf.set_rich_text_font_size(&body_rt, 13.0);

    let key_style = FragmentStyle {
        color: key_color,
        ..FragmentStyle::default()
    };
    let val_style = FragmentStyle {
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
    let secret_style = FragmentStyle {
        color: secret_color,
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

    let grouped = crate::env_inspector::grouped_env_vars();
    for (category, vars) in &grouped {
        body.add_text(category.to_uppercase().as_str(), header_style)
            .new_line();
        for var in vars {
            let display_value = if var.is_secret {
                crate::env_inspector::mask_value(&var.value)
            } else if var.value.len() > 60 {
                format!("{}...", &var.value[..57])
            } else {
                var.value.clone()
            };

            body.add_text("  ", dim_style);
            body.add_text(&var.key, key_style);
            body.add_text("=", dim_style);
            if var.is_secret {
                body.add_text(&display_value, secret_style);
            } else {
                body.add_text(&display_value, val_style);
            }
            body.new_line();
        }
        body.new_line();
    }

    // Footer
    body.new_line();
    body.add_text(" Escape ", key_bg_style)
        .add_text(" close", footer_dim_style());

    body.build();

    objects.push(Object::RichText(RichText {
        id: body_rt,
        position: [40., context_dimension.margin.top_y + 85.],
        lines: None,
    }));

    sugarloaf.set_objects(objects);
}

fn footer_dim_style() -> FragmentStyle {
    FragmentStyle {
        color: [0.45, 0.45, 0.5, 1.0],
        ..FragmentStyle::default()
    }
}
