//! Top-level renderer — orchestrates atlas, pipeline, text, and damage tracking.
//!
//! Builds instance buffers from Terminal grid state and executes Metal render passes.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_metal::*;
use volt_core::Terminal;
use volt_core::cell::CellFlags;
use volt_core::color::{self, Color};

use crate::atlas::{ATLAS_SIZE, GlyphAtlas};
use crate::pipeline::{MetalPipeline, PipelineError};
use crate::shaders::{Instance, Uniforms};
use crate::text::{CellMetrics, TextSystem};

/// Terminal renderer error.
#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("pipeline error: {0}")]
    Pipeline(#[from] PipelineError),
}

/// The terminal renderer. Owns all rendering state.
pub struct Renderer {
    pipeline: MetalPipeline,
    text: TextSystem,
    atlas: GlyphAtlas,
    atlas_texture: Retained<ProtocolObject<dyn MTLTexture>>,
    /// Viewport size in pixels.
    viewport_width: f32,
    viewport_height: f32,
}

impl Renderer {
    /// Create a new renderer with the system default Metal device.
    pub fn new(
        font_family: Option<&str>,
        font_size: f32,
        line_height: f32,
    ) -> Result<Self, RendererError> {
        let pipeline = MetalPipeline::new()?;
        let mut text = TextSystem::new(font_family, font_size, line_height);
        text.prepopulate_ascii();

        let atlas = GlyphAtlas::with_default_size();
        let atlas_texture = pipeline.create_atlas_texture(ATLAS_SIZE, ATLAS_SIZE)?;

        Ok(Self {
            pipeline,
            text,
            atlas,
            atlas_texture,
            viewport_width: 800.0,
            viewport_height: 600.0,
        })
    }

    /// Create with an existing device.
    pub fn with_device(
        device: Retained<ProtocolObject<dyn MTLDevice>>,
        font_family: Option<&str>,
        font_size: f32,
        line_height: f32,
    ) -> Result<Self, RendererError> {
        let pipeline = MetalPipeline::with_device(device)?;
        let mut text = TextSystem::new(font_family, font_size, line_height);
        text.prepopulate_ascii();

        let atlas = GlyphAtlas::with_default_size();
        let atlas_texture = pipeline.create_atlas_texture(ATLAS_SIZE, ATLAS_SIZE)?;

        Ok(Self {
            pipeline,
            text,
            atlas,
            atlas_texture,
            viewport_width: 800.0,
            viewport_height: 600.0,
        })
    }

    /// Get cell metrics for the current font configuration.
    pub fn cell_metrics(&self) -> CellMetrics {
        self.text.cell_metrics()
    }

    /// Get the Metal pipeline (for CAMetalLayer setup).
    pub fn pipeline(&self) -> &MetalPipeline {
        &self.pipeline
    }

    /// Update viewport size (call on window resize).
    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    /// Change font size dynamically (for Cmd+Plus / Cmd+Minus zoom).
    ///
    /// Both `font_size` and `line_height` should be in physical pixels.
    /// Clears glyph caches and atlas, recomputes cell metrics.
    pub fn set_font_size(&mut self, font_size: f32, line_height: f32) {
        self.text.set_font_size(font_size, line_height);
        self.atlas = GlyphAtlas::with_default_size();
        if let Ok(tex) = self.pipeline.create_atlas_texture(ATLAS_SIZE, ATLAS_SIZE) {
            self.atlas_texture = tex;
        }
    }

    /// Render the terminal to a drawable.
    ///
    /// Builds instance buffers from the terminal grid, uploads atlas changes,
    /// and executes solid + textured render passes.
    ///
    /// If `present_drawable` is provided, it will be scheduled for presentation
    /// on the command buffer before commit (required for CAMetalDisplayLink flow).
    pub fn draw(
        &mut self,
        terminal: &Terminal,
        render_pass_desc: &MTLRenderPassDescriptor,
        present_drawable: Option<&ProtocolObject<dyn MTLDrawable>>,
    ) {
        let cell = self.text.cell_metrics();
        let grid = terminal.grid();
        let cursor = terminal.cursor();
        let rows = terminal.size().rows as usize;
        let cols = terminal.size().cols as usize;

        // Build instance lists
        let mut bg_instances: Vec<Instance> = Vec::with_capacity(rows * cols / 4);
        let mut text_instances: Vec<Instance> = Vec::with_capacity(rows * cols);

        for row in 0..rows {
            for col in 0..cols {
                let grid_cell = grid.cell(row, col);
                if grid_cell.is_wide_spacer() {
                    continue;
                }

                let x = col as f32 * cell.width;
                let y = row as f32 * cell.height;

                // Background (only for non-default)
                if grid_cell.bg != Color::Default {
                    let w = if grid_cell.is_wide() {
                        cell.width * 2.0
                    } else {
                        cell.width
                    };
                    bg_instances.push(Instance {
                        position: [x, y],
                        size: [w, cell.height],
                        uv_rect: [0.0; 4],
                        color: color_to_rgba(grid_cell.bg),
                    });
                }

                // Text glyph
                if grid_cell.c != ' ' && grid_cell.c != '\0' {
                    let bold = grid_cell.flags.contains(CellFlags::BOLD);
                    let italic = grid_cell.flags.contains(CellFlags::ITALIC);

                    if let Some(cache_key) = self.text.cache_key_for_char(grid_cell.c, bold, italic)
                    {
                        // Ensure glyph is in atlas
                        let region = if let Some(r) = self.atlas.get(&cache_key) {
                            Some(*r)
                        } else if let Some(image) = self.text.rasterize(cache_key) {
                            self.atlas.insert(cache_key, &image)
                        } else {
                            None
                        };

                        if let Some(r) = region {
                            if r.px_width > 0 && r.px_height > 0 {
                                let gx = x + r.left as f32;
                                let gy = y + cell.baseline - r.top as f32;

                                text_instances.push(Instance {
                                    position: [gx, gy],
                                    size: [r.px_width as f32, r.px_height as f32],
                                    uv_rect: [r.u, r.v, r.u_width, r.v_height],
                                    color: color_to_rgba(grid_cell.fg),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Cursor instance
        let cursor_instance = Instance {
            position: [
                cursor.pos.col as f32 * cell.width,
                cursor.pos.row as f32 * cell.height,
            ],
            size: [cell.width, cell.height],
            uv_rect: [0.0; 4],
            color: [0.8, 0.8, 0.8, 0.7],
        };

        // Upload atlas if dirty
        if self.atlas.is_dirty() {
            let (min_y, max_y) = self.atlas.dirty_range();
            let upload_height = max_y.saturating_sub(min_y);
            if upload_height > 0 {
                MetalPipeline::upload_texture(
                    &self.atlas_texture,
                    self.atlas.pixels(),
                    self.atlas.width(),
                    min_y,
                    upload_height,
                );
            }
            self.atlas.mark_clean();
        }

        // Encode render commands
        let Some(command_buffer) = self.pipeline.command_queue().commandBuffer() else {
            return;
        };

        let Some(encoder) = command_buffer.renderCommandEncoderWithDescriptor(render_pass_desc)
        else {
            return;
        };

        let uniforms = Uniforms {
            viewport_size: [self.viewport_width, self.viewport_height],
        };
        let uniform_bytes = unsafe {
            std::slice::from_raw_parts(&uniforms as *const _ as *const u8, size_of::<Uniforms>())
        };

        // Pass 1: Backgrounds
        if !bg_instances.is_empty() {
            self.encode_instanced_draw(
                &encoder,
                self.pipeline.solid_pipeline(),
                &bg_instances,
                uniform_bytes,
                None,
            );
        }

        // Pass 2: Text glyphs
        if !text_instances.is_empty() {
            self.encode_instanced_draw(
                &encoder,
                self.pipeline.text_pipeline(),
                &text_instances,
                uniform_bytes,
                Some(&self.atlas_texture),
            );
        }

        // Pass 3: Cursor
        if terminal.cursor().visible {
            self.encode_instanced_draw(
                &encoder,
                self.pipeline.solid_pipeline(),
                &[cursor_instance],
                uniform_bytes,
                None,
            );
        }

        encoder.endEncoding();

        if let Some(drawable) = present_drawable {
            command_buffer.presentDrawable(drawable);
        }

        command_buffer.commit();
    }

    fn encode_instanced_draw(
        &self,
        encoder: &ProtocolObject<dyn MTLRenderCommandEncoder>,
        pipeline_state: &ProtocolObject<dyn MTLRenderPipelineState>,
        instances: &[Instance],
        uniform_bytes: &[u8],
        texture: Option<&ProtocolObject<dyn MTLTexture>>,
    ) {
        encoder.setRenderPipelineState(pipeline_state);

        // Set instance buffer (buffer 0)
        let instance_bytes = unsafe {
            std::slice::from_raw_parts(
                instances.as_ptr() as *const u8,
                instances.len() * size_of::<Instance>(),
            )
        };
        unsafe {
            encoder.setVertexBytes_length_atIndex(
                std::ptr::NonNull::new(instance_bytes.as_ptr() as *mut _).unwrap(),
                instance_bytes.len(),
                0,
            );

            // Set uniforms (buffer 1)
            encoder.setVertexBytes_length_atIndex(
                std::ptr::NonNull::new(uniform_bytes.as_ptr() as *mut _).unwrap(),
                uniform_bytes.len(),
                1,
            );

            // Set texture if needed
            if let Some(tex) = texture {
                encoder.setFragmentTexture_atIndex(Some(tex), 0);
            }

            // Draw instanced quads (6 vertices per quad = 2 triangles)
            encoder.drawPrimitives_vertexStart_vertexCount_instanceCount(
                MTLPrimitiveType::Triangle,
                0,
                6,
                instances.len(),
            );
        }
    }
}

/// Convert a volt-core Color to RGBA floats.
fn color_to_rgba(color: Color) -> [f32; 4] {
    match color {
        Color::Default => [1.0, 1.0, 1.0, 1.0],
        Color::Named(named) => {
            let rgb = color::default_indexed_color(named.to_index());
            [
                rgb.r as f32 / 255.0,
                rgb.g as f32 / 255.0,
                rgb.b as f32 / 255.0,
                1.0,
            ]
        }
        Color::Indexed(idx) => {
            let rgb = color::default_indexed_color(idx);
            [
                rgb.r as f32 / 255.0,
                rgb.g as f32 / 255.0,
                rgb.b as f32 / 255.0,
                1.0,
            ]
        }
        Color::Rgb(rgb) => [
            rgb.r as f32 / 255.0,
            rgb.g as f32 / 255.0,
            rgb.b as f32 / 255.0,
            1.0,
        ],
    }
}
