use crate::constants;
use crate::context::grid::ContextDimension;
use rio_backend::config::navigation::{Navigation, NavigationMode};
use rio_backend::config::Config;
use rio_window::window::Theme;

#[inline]
pub fn padding_top_from_config(
    navigation: &Navigation,
    padding_y_top: f32,
    num_tabs: usize,
    #[allow(unused)] macos_use_unified_titlebar: bool,
) -> f32 {
    let default_padding = constants::PADDING_Y + padding_y_top;

    #[cfg(not(target_os = "macos"))]
    {
        if navigation.hide_if_single && num_tabs == 1 {
            return default_padding;
        } else if navigation.mode == NavigationMode::TopTab {
            return constants::PADDING_Y_WITH_TAB_ON_TOP + padding_y_top;
        }
    }

    #[cfg(target_os = "macos")]
    {
        if navigation.mode == NavigationMode::NativeTab {
            let additional = if macos_use_unified_titlebar {
                constants::ADDITIONAL_PADDING_Y_ON_UNIFIED_TITLEBAR
            } else {
                0.0
            };
            return additional + padding_y_top;
        }
        // Always add tab bar padding for TopTab — even with single tab
        // since we always render the tab bar now
        if navigation.mode == NavigationMode::TopTab {
            return constants::PADDING_Y + constants::PADDING_Y_BOTTOM_TABS + padding_y_top;
        }
    }

    default_padding
}

#[inline]
pub fn padding_bottom_from_config(
    navigation: &Navigation,
    padding_y_bottom: f32,
    num_tabs: usize,
    is_search_active: bool,
) -> f32 {
    let default_padding = 0.0 + padding_y_bottom;

    // Status bar (20px) is rendered together with the tab bar for
    // TopTab and BottomTab modes (hidden when hide_if_single with 1 tab).
    let is_tab_mode = navigation.mode == NavigationMode::TopTab
        || navigation.mode == NavigationMode::BottomTab;
    let tabs_hidden = navigation.hide_if_single && num_tabs == 1;
    let status_bar_h: f32 = if is_tab_mode && !tabs_hidden {
        20.0
    } else {
        0.0
    };

    if is_search_active {
        // For BottomTab: search bar replaces the tab bar + status bar entirely
        // For TopTab: search bar at bottom + status bar still visible above it
        let search_h = constants::PADDING_Y_BOTTOM_TABS;
        if navigation.mode == NavigationMode::BottomTab {
            // Tab bar and status bar are hidden; only search bar remains
            return padding_y_bottom + search_h;
        }
        return padding_y_bottom + search_h + status_bar_h;
    }

    if tabs_hidden {
        return default_padding;
    }

    if navigation.mode == NavigationMode::BottomTab {
        // Tab bar (22px) + status bar (20px) both at the bottom
        return padding_y_bottom + constants::PADDING_Y_BOTTOM_TABS + status_bar_h;
    }

    if navigation.mode == NavigationMode::TopTab {
        // Tab bar is at the top; status bar alone at the bottom
        return default_padding + status_bar_h;
    }

    default_padding
}

#[inline]
pub fn terminal_dimensions(layout: &ContextDimension) -> teletypewriter::WinsizeBuilder {
    let width = layout.width - (layout.margin.x * 2.);
    let height = (layout.height - layout.margin.top_y) - layout.margin.bottom_y;
    teletypewriter::WinsizeBuilder {
        width: width as u16,
        height: height as u16,
        cols: layout.columns as u16,
        rows: layout.lines as u16,
    }
}

#[inline]
pub fn update_colors_based_on_theme(config: &mut Config, theme_opt: Option<Theme>) {
    if let Some(theme) = theme_opt {
        if let Some(adaptive_colors) = &config.adaptive_colors {
            match theme {
                Theme::Light => {
                    if let Some(light_colors) = adaptive_colors.light {
                        config.colors = light_colors;
                    }
                }
                Theme::Dark => {
                    if let Some(darkcolors) = adaptive_colors.dark {
                        config.colors = darkcolors;
                    }
                }
            }
        }
    }
}
