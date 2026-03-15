use crate::constants::*;
use crate::context::title::ContextTitle;
use rio_backend::config::colors::Colors;
use rio_backend::config::navigation::{Navigation, NavigationMode};
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};
use rustc_hash::FxHashMap;
use std::collections::HashMap;

/// Tab rendering constants
const TAB_MIN_WIDTH: f32 = 40.0;
const TAB_CHAR_WIDTH: f32 = 8.0;
const TAB_PADDING: f32 = 16.0;
const TAB_GAP: f32 = 2.0;

/// Calculate tab width based on label length
fn tab_width_for_label(label: &str) -> f32 {
    (TAB_PADDING + label.len() as f32 * TAB_CHAR_WIDTH).max(TAB_MIN_WIDTH)
}

pub struct ScreenNavigation {
    pub navigation: Navigation,
    pub padding_y: [f32; 2],
    color_automation: HashMap<String, HashMap<String, [f32; 4]>>,
    /// Horizontal scroll offset for tab bar (in logical pixels)
    pub tab_scroll_offset: f32,
}

impl ScreenNavigation {
    pub fn new(
        navigation: Navigation,
        color_automation: HashMap<String, HashMap<String, [f32; 4]>>,
        padding_y: [f32; 2],
    ) -> ScreenNavigation {
        ScreenNavigation {
            navigation,
            color_automation,
            padding_y,
            tab_scroll_offset: 0.0,
        }
    }

    /// Scroll the tab bar by a delta. Uses average tab width estimate.
    pub fn scroll_tabs(&mut self, delta: f32, num_tabs: usize, visible_width: f32) {
        let avg_tab = TAB_MIN_WIDTH + TAB_GAP;
        let total_width = num_tabs as f32 * avg_tab;
        let max_scroll = (total_width - visible_width).max(0.0);
        self.tab_scroll_offset = (self.tab_scroll_offset + delta).clamp(0.0, max_scroll);
    }

    /// Ensure the given tab index is visible. (Actual precise scrolling is done in render loop.)
    pub fn ensure_tab_visible(&mut self, _tab_idx: usize, _visible_width: f32) {
        // Scroll adjustment is now handled in the tab() render method
        // using actual computed tab positions
    }

    #[inline]
    pub fn build_objects(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        dimensions: (f32, f32, f32),
        colors: &Colors,
        context_manager: &crate::context::ContextManager<rio_backend::event::EventProxy>,
        is_search_active: bool,
        objects: &mut Vec<Object>,
    ) {
        // When search is active then BottomTab should not be rendered
        if is_search_active && self.navigation.mode == NavigationMode::BottomTab {
            return;
        }

        let current = context_manager.current_index();
        let len = context_manager.len();

        let titles = &context_manager.titles.titles;

        match self.navigation.mode {
            #[cfg(target_os = "macos")]
            NavigationMode::NativeTab => {}
            NavigationMode::Bookmark => self.bookmark(
                objects,
                titles,
                colors,
                len,
                current,
                self.navigation.hide_if_single,
                dimensions,
            ),
            NavigationMode::TopTab => {
                let position_y = 0.0;
                self.tab(
                    sugarloaf,
                    objects,
                    titles,
                    colors,
                    len,
                    current,
                    position_y,
                    self.navigation.hide_if_single,
                    dimensions,
                );
            }
            NavigationMode::BottomTab => {
                let (_, height, scale) = dimensions;
                // Stack above the status bar (20px) at the bottom
                let position_y =
                    (height / scale) - PADDING_Y_BOTTOM_TABS - STATUS_BAR_HEIGHT;
                self.tab(
                    sugarloaf,
                    objects,
                    titles,
                    colors,
                    len,
                    current,
                    position_y,
                    self.navigation.hide_if_single,
                    dimensions,
                );
            }
            // Minimal simply does not do anything
            NavigationMode::Plain => {}
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn bookmark(
        &mut self,
        objects: &mut Vec<Object>,
        titles: &FxHashMap<usize, ContextTitle>,
        colors: &Colors,
        len: usize,
        current: usize,
        hide_if_single: bool,
        dimensions: (f32, f32, f32),
    ) {
        if hide_if_single && len <= 1 {
            return;
        }

        let (width, _, scale) = dimensions;

        let mut initial_position = (width / scale) - PADDING_X_COLLAPSED_TABS;
        let position_modifier = 20.;
        for i in (0..len).rev() {
            let mut size = INACTIVE_TAB_WIDTH_SIZE;
            // Use per-tab accent color if available, otherwise fall back to config colors
            let mut color = titles
                .get(&i)
                .map(|t| t.accent_color)
                .unwrap_or(colors.tabs);
            if i == current {
                size = ACTIVE_TAB_WIDTH_SIZE;
            }

            if let Some(title) = titles.get(&i) {
                if !self.color_automation.is_empty() {
                    if let Some(extra) = &title.extra {
                        if let Some(color_overwrite) = get_color_overwrite(
                            &self.color_automation,
                            &extra.program,
                            &extra.path,
                        ) {
                            color = *color_overwrite;
                        }
                    }
                }
            }

            let renderable = Quad {
                position: [initial_position, 0.0],
                color,
                size: [15.0, size],
                ..Quad::default()
            };
            initial_position -= position_modifier;
            objects.push(Object::Quad(renderable));
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn tab(
        &mut self,
        sugarloaf: &mut Sugarloaf,
        objects: &mut Vec<Object>,
        titles: &FxHashMap<usize, ContextTitle>,
        colors: &Colors,
        len: usize,
        current: usize,
        position_y: f32,
        hide_if_single: bool,
        dimensions: (f32, f32, f32),
    ) {
        let (width, height, scale) = dimensions;
        let visible_width = width / scale;
        let tabs_hidden = false; // Always show navbar

        // ALWAYS draw tab bar background and buttons
        objects.push(Object::Quad(Quad {
            position: [0.0, position_y],
            color: colors.bar,
            size: [width, PADDING_Y_BOTTOM_TABS],
            ..Quad::default()
        }));

        // Tab pills only when not hidden
        if !tabs_hidden {
        self.ensure_tab_visible(current, visible_width);

        let left_margin = 4.0;

        // First pass: compute labels and positions
        let mut tab_layouts: Vec<(f32, f32, String)> = Vec::with_capacity(len); // (x, width, label)
        let mut x_cursor = left_margin;
        for i in 0..len {
            let label = if let Some(title) = titles.get(&i) {
                if let Some(ref custom) = title.custom_name {
                    let mut s = custom.clone();
                    if s.len() > 20 {
                        s.truncate(18);
                        s.push_str("..");
                    }
                    s
                } else {
                    format!("{}", i + 1)
                }
            } else {
                format!("{}", i + 1)
            };
            let w = tab_width_for_label(&label);
            tab_layouts.push((x_cursor, w, label));
            x_cursor += w + TAB_GAP;
        }

        // Auto-scroll to keep current tab visible
        if let Some((cur_x, cur_w, _)) = tab_layouts.get(current) {
            let tab_end = cur_x + cur_w;
            if *cur_x < self.tab_scroll_offset {
                self.tab_scroll_offset = *cur_x;
            } else if tab_end > self.tab_scroll_offset + visible_width {
                self.tab_scroll_offset = tab_end - visible_width;
            }
        }

        // Second pass: render
        for (i, (base_x, w, label)) in tab_layouts.iter().enumerate() {
            let tab_x = base_x - self.tab_scroll_offset;

            // Skip off-screen tabs
            if tab_x + w < 0.0 || tab_x > visible_width {
                continue;
            }

            let is_current = i == current;

            let tab_accent = titles
                .get(&i)
                .map(|t| t.accent_color)
                .unwrap_or(colors.tabs_active_highlight);

            let tab_color = if is_current {
                tab_accent
            } else {
                [
                    tab_accent[0] * 0.65,
                    tab_accent[1] * 0.65,
                    tab_accent[2] * 0.65,
                    1.0,
                ]
            };

            objects.push(Object::Quad(Quad {
                position: [tab_x, position_y],
                color: tab_color,
                size: [*w, PADDING_Y_BOTTOM_TABS],
                ..Quad::default()
            }));

            if is_current {
                let indicator_y = position_y + PADDING_Y_BOTTOM_TABS - 2.5;
                objects.push(Object::Quad(Quad {
                    position: [tab_x, indicator_y],
                    color: [1.0, 1.0, 1.0, 0.9],
                    size: [*w, 2.5],
                    ..Quad::default()
                }));
            }

            // White text on colored background for all tabs
            let fg = [1.0, 1.0, 1.0, if is_current { 1.0 } else { 0.85 }];

            let tab_rt = sugarloaf.create_temp_rich_text();
            sugarloaf.set_rich_text_font_size(&tab_rt, 12.);
            let content = sugarloaf.content();

            content
                .sel(tab_rt)
                .clear()
                .new_line()
                .add_text(
                    &label,
                    FragmentStyle {
                        color: fg,
                        ..FragmentStyle::default()
                    },
                )
                .build();

            objects.push(Object::RichText(RichText {
                id: tab_rt,
                position: [tab_x + 6.0, position_y],
                lines: None,
            }));
        } // end if !tabs_hidden — tab pills only

        // Top bar buttons — ALWAYS rendered (even with single tab)
        let top_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&top_rt, 11.);
        let top_dim = FragmentStyle { color: [0.5, 0.5, 0.55, 0.8], ..FragmentStyle::default() };
        let top_link = FragmentStyle { color: [0.6, 0.75, 0.9, 1.0], ..FragmentStyle::default() };

        sugarloaf.content().sel(top_rt).clear().new_line()
            .add_text("Help", top_link)
            .add_text("  ", top_dim)
            .add_text("Settings", top_link)
            .add_text("  ", top_dim)
            .build();

        let top_text_w = 110.0_f32;
        objects.push(Object::RichText(RichText {
            id: top_rt,
            position: [visible_width - top_text_w, position_y + 1.0],
            lines: None,
        }));

        // --- Bottom status bar — ALWAYS rendered, even with single tab ---
        let sb_h = 22.0_f32;
        let sb_y = (height / scale) - sb_h;

        // Status bar background — dark blue-tinted like VS Code
        objects.push(Object::Quad(Quad {
            position: [0.0, sb_y],
            color: [0.0, 0.05, 0.15, 1.0],
            size: [width, sb_h],
            ..Quad::default()
        }));

        // All items rendered as a single rich text for clean spacing
        let sb_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&sb_rt, 11.);

        let sb_text = FragmentStyle { color: [0.75, 0.8, 0.85, 1.0], ..FragmentStyle::default() };
        let sb_accent = FragmentStyle { color: [0.4, 0.7, 1.0, 1.0], ..FragmentStyle::default() };
        let sb_sep = FragmentStyle { color: [0.3, 0.35, 0.4, 0.6], ..FragmentStyle::default() };
        let sb_green = FragmentStyle { color: [0.3, 0.85, 0.5, 1.0], ..FragmentStyle::default() };
        let sb_gold = FragmentStyle { color: [0.95, 0.75, 0.2, 1.0], ..FragmentStyle::default() };
        let sb_purple = FragmentStyle { color: [0.7, 0.5, 0.95, 1.0], ..FragmentStyle::default() };

        let content = sugarloaf.content();
        let sb = content.sel(sb_rt);
        sb.clear().new_line();

        // Left side: clickable items with subtle styling
        sb.add_text("  AI ", sb_purple);
        sb.add_text("|", sb_sep);
        sb.add_text(" History ", sb_text);
        sb.add_text("|", sb_sep);
        sb.add_text(" Env ", sb_text);
        sb.add_text("|", sb_sep);
        sb.add_text(" Bookmarks ", sb_text);
        sb.add_text("|", sb_sep);
        sb.add_text(" Connect ", sb_accent);
        sb.add_text("|", sb_sep);
        sb.add_text(" Cmds ", sb_gold);
        sb.add_text("|", sb_sep);
        sb.add_text(" Layout ", sb_text);

        sb.build();

        objects.push(Object::RichText(RichText {
            id: sb_rt,
            position: [0.0, sb_y + 1.0],
            lines: None,
        }));

        // tmux — right side, styled text (no pill)
        let tmux_rt = sugarloaf.create_temp_rich_text();
        sugarloaf.set_rich_text_font_size(&tmux_rt, 11.);
        sugarloaf.content().sel(tmux_rt).clear().new_line()
            .add_text("tmux", FragmentStyle { color: [0.4, 0.85, 0.55, 1.0], ..FragmentStyle::default() }).build();
        objects.push(Object::RichText(RichText {
            id: tmux_rt,
            position: [visible_width - 40.0, sb_y + 1.0],
            lines: None,
        }));
    }
  }
}

/// Button positions for click detection (must match rendering)
pub const NAV_BTN_SIZE: f32 = 22.0;
pub const NAV_BTN_GAP: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavButton {
    Help,
    Settings,
    AiAssistant,
    TmuxConnect,
    History,
    EnvViewer,
    Bookmarks,
    Connections,
    SlashCommands,
    Layouts,
}

/// Check if a click (in logical pixels) hit a top bar button.
pub fn nav_button_at_position(x: f32, visible_width: f32) -> Option<NavButton> {
    // Text layout: "Help  Settings  " right-aligned, ~110px wide
    // "Help" at offset 0..28, "Settings" at offset 42..96 from text start
    let text_start = visible_width - 110.0;
    let relative_x = x - text_start;

    if relative_x >= 0.0 && relative_x < 30.0 {
        return Some(NavButton::Help);
    }
    if relative_x >= 38.0 && relative_x < 100.0 {
        return Some(NavButton::Settings);
    }
    None
}

/// Status bar height
pub const STATUS_BAR_HEIGHT: f32 = 22.0;

/// Check if a click (in logical pixels) hit a bottom status bar item.
/// Text: "  AI | History | Env | Bookmarks | Connect | Cmds | Layout"
/// Uses char width ~6.5px at font size 11 (monospace estimate).
pub fn status_button_at_position(x: f32, y: f32, win_height: f32, visible_width: f32) -> Option<NavButton> {
    let status_y = win_height - STATUS_BAR_HEIGHT;
    if y < status_y || y > win_height {
        return None;
    }

    // Char positions (0-indexed) with ~6.5px per char:
    //   "  AI " chars 0-4       → x 0..32
    //   "| History " chars 5-14 → x 32..97
    //   "| Env " chars 15-20   → x 97..136
    //   "| Bookmarks " 21-32   → x 136..214
    //   "| Connect " 33-42     → x 214..279
    //   "| Cmds " 43-49        → x 279..325
    //   "| Layout " 50-58      → x 325..383
    let cw = 6.5_f32;

    // Use generous zones — click anywhere in the range maps to that item
    if x < 5.0 * cw { return Some(NavButton::AiAssistant); }     // 0..32
    if x < 15.0 * cw { return Some(NavButton::History); }         // 32..97
    if x < 21.0 * cw { return Some(NavButton::EnvViewer); }       // 97..136
    if x < 33.0 * cw { return Some(NavButton::Bookmarks); }       // 136..214
    if x < 43.0 * cw { return Some(NavButton::Connections); }     // 214..279
    if x < 50.0 * cw { return Some(NavButton::SlashCommands); }   // 279..325
    if x < 59.0 * cw { return Some(NavButton::Layouts); }         // 325..383

    // tmux text on right (~40px from right edge)
    let tmux_x = visible_width - 40.0;
    if x >= tmux_x {
        return Some(NavButton::TmuxConnect);
    }

    None
}

#[inline]
fn get_color_overwrite<'a>(
    color_automation: &'a HashMap<String, HashMap<String, [f32; 4]>>,
    program: &str,
    path: &str,
) -> Option<&'a [f32; 4]> {
    color_automation
        .get(program)
        .and_then(|m| m.get(path).or_else(|| m.get("")))
        .or_else(|| color_automation.get("").and_then(|m| m.get(path)))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::renderer::navigation::get_color_overwrite;

    #[test]
    fn test_get_color_overwrite() {
        let program = "nvim";
        let path = "/home/";

        let program_and_path = [0.0, 0.0, 0.0, 0.0];
        let program_only = [1.1, 1.1, 1.1, 1.1];
        let path_only = [2.2, 2.2, 2.2, 2.2];
        let neither = [3.3, 3.3, 3.3, 3.3];

        let color_automation = HashMap::from([
            (
                program.to_owned(),
                HashMap::from([
                    (path.to_owned(), program_and_path),
                    (String::new(), program_only),
                ]),
            ),
            (
                String::new(),
                HashMap::from([(path.to_owned(), path_only), (String::new(), neither)]),
            ),
        ]);

        let program_and_path_result =
            get_color_overwrite(&color_automation, program, path)
                .expect("it to return a color");

        assert_eq!(&program_and_path, program_and_path_result);

        let program_only_result = get_color_overwrite(&color_automation, program, "")
            .expect("it to return a color");

        assert_eq!(&program_only, program_only_result);

        let path_only_result = get_color_overwrite(&color_automation, "", path)
            .expect("it to return a color");

        assert_eq!(&path_only, path_only_result);

        let neither_result =
            get_color_overwrite(&color_automation, "", "").expect("it to return a color");

        assert_eq!(&neither, neither_result);
    }
}
