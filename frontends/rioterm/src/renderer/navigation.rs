use crate::constants::*;
use crate::context::title::ContextTitle;
use rio_backend::config::colors::Colors;
use rio_backend::config::navigation::{Navigation, NavigationMode};
use rio_backend::sugarloaf::{FragmentStyle, Object, Quad, RichText, Sugarloaf};
use rustc_hash::FxHashMap;
use std::collections::HashMap;

/// Tab rendering constants
const TAB_WIDTH: f32 = 54.0; // Compact tabs with slight padding
const TAB_GAP: f32 = 3.0; // Tight gap between tabs
const TAB_INNER_HEIGHT: f32 = 18.0; // Tab pill height (smaller than bar)
const TAB_BORDER_RADIUS: f32 = 5.0; // Rounded corners
const TAB_Y_OFFSET: f32 = 2.0; // Vertical centering offset in bar

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

    /// Scroll the tab bar by a delta (positive = scroll right, negative = scroll left).
    /// Clamps to valid range based on total tab count and visible width.
    pub fn scroll_tabs(&mut self, delta: f32, num_tabs: usize, visible_width: f32) {
        let tab_width = TAB_WIDTH + TAB_GAP;
        let total_width = num_tabs as f32 * tab_width;
        let max_scroll = (total_width - visible_width).max(0.0);
        self.tab_scroll_offset = (self.tab_scroll_offset + delta).clamp(0.0, max_scroll);
    }

    /// Ensure the given tab index is visible by adjusting scroll offset.
    pub fn ensure_tab_visible(&mut self, tab_idx: usize, visible_width: f32) {
        let tab_width = TAB_WIDTH + TAB_GAP;
        let tab_start = tab_idx as f32 * tab_width;
        let tab_end = tab_start + TAB_WIDTH;

        if tab_start < self.tab_scroll_offset {
            self.tab_scroll_offset = tab_start;
        } else if tab_end > self.tab_scroll_offset + visible_width {
            self.tab_scroll_offset = tab_end - visible_width;
        }
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
                let position_y = (height / scale) - PADDING_Y_BOTTOM_TABS;
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
        if hide_if_single && len <= 1 {
            return;
        }

        let (width, _, scale) = dimensions;
        let visible_width = width / scale;

        // Ensure current tab is visible
        self.ensure_tab_visible(current, visible_width);

        // Draw tab bar background (full width)
        objects.push(Object::Quad(Quad {
            position: [0.0, position_y],
            color: colors.bar,
            size: [width, PADDING_Y_BOTTOM_TABS],
            ..Quad::default()
        }));

        let tab_step = TAB_WIDTH + TAB_GAP;
        let left_margin = 4.0; // Small left margin

        for i in 0..len {
            let tab_x =
                left_margin + i as f32 * tab_step - self.tab_scroll_offset;

            // Skip tabs that are off-screen
            if tab_x + TAB_WIDTH < 0.0 || tab_x > visible_width {
                continue;
            }

            let is_current = i == current;

            // Per-tab accent color
            let tab_accent = titles
                .get(&i)
                .map(|t| t.accent_color)
                .unwrap_or(colors.tabs_active_highlight);

            let pill_y = position_y + TAB_Y_OFFSET;

            if is_current {
                // Active tab: accent-colored rounded pill
                objects.push(Object::Quad(Quad {
                    position: [tab_x, pill_y],
                    color: tab_accent,
                    size: [TAB_WIDTH, TAB_INNER_HEIGHT],
                    border_radius: [
                        TAB_BORDER_RADIUS,
                        TAB_BORDER_RADIUS,
                        TAB_BORDER_RADIUS,
                        TAB_BORDER_RADIUS,
                    ],
                    ..Quad::default()
                }));
            } else {
                // Inactive tab: subtle rounded pill with dimmed accent border
                let inactive_bg = [
                    colors.bar[0] + 0.03,
                    colors.bar[1] + 0.03,
                    colors.bar[2] + 0.03,
                    0.8,
                ];
                objects.push(Object::Quad(Quad {
                    position: [tab_x, pill_y],
                    color: inactive_bg,
                    size: [TAB_WIDTH, TAB_INNER_HEIGHT],
                    border_radius: [
                        TAB_BORDER_RADIUS,
                        TAB_BORDER_RADIUS,
                        TAB_BORDER_RADIUS,
                        TAB_BORDER_RADIUS,
                    ],
                    border_color: [
                        tab_accent[0] * 0.4,
                        tab_accent[1] * 0.4,
                        tab_accent[2] * 0.4,
                        0.3,
                    ],
                    border_width: 1.0,
                    ..Quad::default()
                }));
            }

            // Tab label
            let label = if let Some(title) = titles.get(&i) {
                if let Some(ref custom) = title.custom_name {
                    let mut s = custom.clone();
                    if s.len() > 5 {
                        s.truncate(5);
                    }
                    s
                } else {
                    format!("{}", i + 1)
                }
            } else {
                format!("{}", i + 1)
            };

            let fg = if is_current {
                // Dark text on accent bg for readability
                [0.05, 0.05, 0.05, 1.0]
            } else {
                colors.tabs_foreground
            };

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
                position: [tab_x + 6.0, pill_y - 1.0],
                lines: None,
            }));
        }
    }
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
