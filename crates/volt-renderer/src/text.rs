//! Text pipeline — font discovery, shaping, and rasterization via cosmic-text.
//!
//! Wraps `FontSystem` and `SwashCache` to provide glyph images for the atlas.
//! Computes cell metrics (advance width, line height) from the configured font.

use std::collections::HashMap;

use cosmic_text::{
    Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent,
    SwashImage, Weight,
};

/// Cell dimensions in pixels, derived from font metrics.
#[derive(Debug, Clone, Copy)]
pub struct CellMetrics {
    /// Width of a single cell in pixels.
    pub width: f32,
    /// Height of a single cell (line height) in pixels.
    pub height: f32,
    /// Baseline offset from top of cell.
    pub baseline: f32,
    /// Font size used.
    pub font_size: f32,
}

/// A rasterized glyph image ready for atlas insertion.
#[derive(Debug, Clone)]
pub struct GlyphImage {
    /// RGBA pixel data (4 bytes per pixel).
    pub data: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// X offset from the glyph origin.
    pub left: i32,
    /// Y offset from the baseline (positive = up).
    pub top: i32,
    /// Whether this is a color glyph (emoji).
    pub is_color: bool,
}

/// Text system wrapping cosmic-text for terminal glyph rendering.
pub struct TextSystem {
    font_system: FontSystem,
    swash_cache: SwashCache,
    metrics: CellMetrics,
    /// Cache from char → CacheKey (for the primary font at current size).
    char_cache: HashMap<char, Option<CacheKey>>,
}

impl TextSystem {
    /// Create a new text system with the given font and size.
    ///
    /// `font_family` is a CSS-like family name (e.g., "JetBrains Mono").
    /// Falls back to the system monospace font if not found.
    pub fn new(font_family: Option<&str>, font_size: f32, line_height: f32) -> Self {
        let font_system = FontSystem::new();

        let metrics = compute_cell_metrics(&font_system, font_family, font_size, line_height);

        Self {
            font_system,
            swash_cache: SwashCache::new(),
            metrics,
            char_cache: HashMap::new(),
        }
    }

    /// Get cell metrics.
    pub fn cell_metrics(&self) -> CellMetrics {
        self.metrics
    }

    /// Get the CacheKey for a character, shaping it if not cached.
    pub fn cache_key_for_char(&mut self, c: char, bold: bool, italic: bool) -> Option<CacheKey> {
        // For non-styled chars, check the simple cache
        if !bold && !italic {
            if let Some(cached) = self.char_cache.get(&c) {
                return *cached;
            }
        }

        let key = shape_single_char(
            &mut self.font_system,
            c,
            self.metrics.font_size,
            self.metrics.height,
            bold,
            italic,
        );

        if !bold && !italic {
            self.char_cache.insert(c, key);
        }

        key
    }

    /// Rasterize a glyph by its CacheKey, returning RGBA pixel data.
    pub fn rasterize(&mut self, cache_key: CacheKey) -> Option<GlyphImage> {
        let image = self
            .swash_cache
            .get_image(&mut self.font_system, cache_key)
            .as_ref()?;

        Some(swash_to_glyph_image(image))
    }

    /// Pre-populate the cache for printable ASCII (32-126).
    pub fn prepopulate_ascii(&mut self) {
        for c in ' '..='~' {
            self.cache_key_for_char(c, false, false);
        }
    }

    /// Change the font size, recomputing cell metrics and clearing caches.
    ///
    /// `font_size` and `line_height` should be in physical pixels (already scaled for Retina).
    pub fn set_font_size(&mut self, font_size: f32, line_height: f32) {
        self.metrics = compute_cell_metrics(&self.font_system, None, font_size, line_height);
        self.char_cache.clear();
        self.prepopulate_ascii();
    }

    /// Get a mutable reference to the font system (for advanced use).
    pub fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }
}

/// Compute cell metrics by shaping a reference character.
fn compute_cell_metrics(
    font_system: &FontSystem,
    font_family: Option<&str>,
    font_size: f32,
    line_height: f32,
) -> CellMetrics {
    let mut fs = FontSystem::new();
    let metrics = Metrics::new(font_size, line_height);
    let mut buffer = Buffer::new(&mut fs, metrics);

    let attrs = match font_family {
        Some(name) => Attrs::new().family(Family::Name(name)),
        None => Attrs::new().family(Family::Monospace),
    };

    {
        let mut buffer = buffer.borrow_with(&mut fs);
        buffer.set_size(Some(f32::MAX), Some(line_height * 2.0));
        buffer.set_text("M", attrs, Shaping::Advanced);
        buffer.shape_until_scroll(true);
    }

    // Extract advance width from the first glyph
    let mut cell_width = font_size * 0.6; // fallback
    let mut baseline = line_height * 0.8;

    if let Some(run) = buffer.layout_runs().next() {
        if let Some(glyph) = run.glyphs.iter().next() {
            cell_width = glyph.w;
            baseline = run.line_y;
        }
    }

    // Drop the temporary font system, use the real one
    let _ = font_system;

    CellMetrics {
        width: cell_width.ceil(),
        height: line_height,
        baseline,
        font_size,
    }
}

/// Shape a single character and return its CacheKey.
fn shape_single_char(
    font_system: &mut FontSystem,
    c: char,
    font_size: f32,
    line_height: f32,
    bold: bool,
    italic: bool,
) -> Option<CacheKey> {
    let metrics = Metrics::new(font_size, line_height);
    let mut buffer = Buffer::new(font_system, metrics);

    let mut attrs = Attrs::new().family(Family::Monospace);
    if bold {
        attrs = attrs.weight(Weight::BOLD);
    }
    if italic {
        attrs = attrs.style(cosmic_text::Style::Italic);
    }

    let mut s = String::with_capacity(4);
    s.push(c);

    {
        let mut buffer = buffer.borrow_with(font_system);
        buffer.set_size(Some(font_size * 4.0), Some(line_height * 2.0));
        buffer.set_text(&s, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(true);
    }

    buffer
        .layout_runs()
        .next()
        .and_then(|run| run.glyphs.iter().next())
        .map(|glyph| glyph.physical((0.0, 0.0), 1.0).cache_key)
}

/// Convert a cosmic-text SwashImage to our GlyphImage format (always RGBA).
fn swash_to_glyph_image(image: &SwashImage) -> GlyphImage {
    let w = image.placement.width as usize;
    let h = image.placement.height as usize;

    let data = match image.content {
        SwashContent::Mask => {
            // Alpha-only mask → expand to RGBA (white + alpha)
            let mut rgba = Vec::with_capacity(w * h * 4);
            for &alpha in &image.data {
                rgba.extend_from_slice(&[255, 255, 255, alpha]);
            }
            rgba
        }
        SwashContent::Color => {
            // Already RGBA
            image.data.clone()
        }
        SwashContent::SubpixelMask => {
            // RGB subpixel → treat as grayscale for now (average channels)
            let mut rgba = Vec::with_capacity(w * h * 4);
            for chunk in image.data.chunks(3) {
                if chunk.len() == 3 {
                    let avg = ((chunk[0] as u16 + chunk[1] as u16 + chunk[2] as u16) / 3) as u8;
                    rgba.extend_from_slice(&[255, 255, 255, avg]);
                }
            }
            rgba
        }
    };

    GlyphImage {
        data,
        width: image.placement.width,
        height: image.placement.height,
        left: image.placement.left,
        top: image.placement.top,
        is_color: matches!(image.content, SwashContent::Color),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_system_creation() {
        let ts = TextSystem::new(None, 14.0, 20.0);
        let m = ts.cell_metrics();
        assert!(m.width > 0.0, "cell width should be positive");
        assert!(m.height > 0.0, "cell height should be positive");
    }

    #[test]
    fn shape_ascii_char() {
        let mut ts = TextSystem::new(None, 14.0, 20.0);
        let key = ts.cache_key_for_char('A', false, false);
        assert!(key.is_some(), "should shape 'A'");
    }

    #[test]
    fn rasterize_glyph() {
        let mut ts = TextSystem::new(None, 14.0, 20.0);
        let key = ts.cache_key_for_char('A', false, false).expect("shape A");
        let image = ts.rasterize(key).expect("rasterize A");
        assert!(image.width > 0);
        assert!(image.height > 0);
        assert_eq!(image.data.len(), (image.width * image.height * 4) as usize);
    }

    #[test]
    fn prepopulate_ascii() {
        let mut ts = TextSystem::new(None, 14.0, 20.0);
        ts.prepopulate_ascii();
        // All printable ASCII should be cached
        for c in ' '..='~' {
            assert!(
                ts.char_cache.contains_key(&c),
                "char '{c}' should be cached"
            );
        }
    }
}
