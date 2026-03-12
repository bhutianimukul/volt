//! volt-renderer: Metal rendering pipeline, glyph atlas, and text shaping.
//!
//! Uses `objc2-metal` for direct Metal API access and `cosmic-text` for
//! font discovery, shaping (harfrust), and rasterization (swash).

pub mod atlas;
pub mod damage;
pub mod pipeline;
pub mod renderer;
pub mod shaders;
pub mod text;

pub use pipeline::MetalPipeline;
pub use renderer::Renderer;
pub use text::{CellMetrics, TextSystem};
