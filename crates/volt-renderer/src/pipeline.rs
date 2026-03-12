//! Metal render pipeline — device, command queue, pipeline states, and resource management.
//!
//! Owns all Metal state needed for rendering. Creates pipeline state objects
//! for solid (background/cursor) and textured (glyph) passes.

use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::ns_string;
use objc2_metal::*;

use crate::shaders;

/// Error type for pipeline operations.
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("no Metal device available")]
    NoDevice,
    #[error("failed to create command queue")]
    NoCommandQueue,
    #[error("failed to compile shaders: {0}")]
    ShaderCompilation(String),
    #[error("failed to create pipeline state: {0}")]
    PipelineCreation(String),
    #[error("failed to create texture")]
    TextureCreation,
    #[error("shader function '{0}' not found")]
    FunctionNotFound(String),
}

/// Owns all Metal pipeline state for terminal rendering.
pub struct MetalPipeline {
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    command_queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    solid_pipeline: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    text_pipeline: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    library: Retained<ProtocolObject<dyn MTLLibrary>>,
}

impl MetalPipeline {
    /// Create a new Metal pipeline with the system default device.
    pub fn new() -> Result<Self, PipelineError> {
        let device = MTLCreateSystemDefaultDevice().ok_or(PipelineError::NoDevice)?;

        let command_queue = device
            .newCommandQueue()
            .ok_or(PipelineError::NoCommandQueue)?;

        // Compile shaders
        let source = ns_string!(shaders::SHADER_SOURCE);
        let library = device
            .newLibraryWithSource_options_error(source, None)
            .map_err(|e| PipelineError::ShaderCompilation(e.to_string()))?;

        // Create solid pipeline (backgrounds, cursor)
        let solid_pipeline = create_pipeline_state(
            &device,
            &library,
            shaders::SOLID_VERTEX_FN,
            shaders::SOLID_FRAGMENT_FN,
        )?;

        // Create textured pipeline (glyphs)
        let text_pipeline = create_pipeline_state(
            &device,
            &library,
            shaders::TEXT_VERTEX_FN,
            shaders::TEXT_FRAGMENT_FN,
        )?;

        Ok(Self {
            device,
            command_queue,
            solid_pipeline,
            text_pipeline,
            library,
        })
    }

    /// Create a new Metal pipeline with an existing device.
    pub fn with_device(
        device: Retained<ProtocolObject<dyn MTLDevice>>,
    ) -> Result<Self, PipelineError> {
        let command_queue = device
            .newCommandQueue()
            .ok_or(PipelineError::NoCommandQueue)?;

        let source = ns_string!(shaders::SHADER_SOURCE);
        let library = device
            .newLibraryWithSource_options_error(source, None)
            .map_err(|e| PipelineError::ShaderCompilation(e.to_string()))?;

        let solid_pipeline = create_pipeline_state(
            &device,
            &library,
            shaders::SOLID_VERTEX_FN,
            shaders::SOLID_FRAGMENT_FN,
        )?;

        let text_pipeline = create_pipeline_state(
            &device,
            &library,
            shaders::TEXT_VERTEX_FN,
            shaders::TEXT_FRAGMENT_FN,
        )?;

        Ok(Self {
            device,
            command_queue,
            solid_pipeline,
            text_pipeline,
            library,
        })
    }

    /// Get the Metal device.
    pub fn device(&self) -> &ProtocolObject<dyn MTLDevice> {
        &self.device
    }

    /// Get the command queue.
    pub fn command_queue(&self) -> &ProtocolObject<dyn MTLCommandQueue> {
        &self.command_queue
    }

    /// Get the solid (background/cursor) pipeline state.
    pub fn solid_pipeline(&self) -> &ProtocolObject<dyn MTLRenderPipelineState> {
        &self.solid_pipeline
    }

    /// Get the textured (glyph) pipeline state.
    pub fn text_pipeline(&self) -> &ProtocolObject<dyn MTLRenderPipelineState> {
        &self.text_pipeline
    }

    /// Get the shader library.
    pub fn library(&self) -> &ProtocolObject<dyn MTLLibrary> {
        &self.library
    }

    /// Create an MTLTexture for the glyph atlas.
    pub fn create_atlas_texture(
        &self,
        width: u32,
        height: u32,
    ) -> Result<Retained<ProtocolObject<dyn MTLTexture>>, PipelineError> {
        let desc = unsafe {
            MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
                MTLPixelFormat::RGBA8Unorm,
                width as usize,
                height as usize,
                false,
            )
        };
        desc.setUsage(MTLTextureUsage::ShaderRead);

        self.device
            .newTextureWithDescriptor(&desc)
            .ok_or(PipelineError::TextureCreation)
    }

    /// Upload RGBA pixel data to a texture region.
    pub fn upload_texture(
        texture: &ProtocolObject<dyn MTLTexture>,
        pixels: &[u8],
        width: u32,
        y_offset: u32,
        height: u32,
    ) {
        let region = MTLRegion {
            origin: MTLOrigin {
                x: 0,
                y: y_offset as usize,
                z: 0,
            },
            size: MTLSize {
                width: width as usize,
                height: height as usize,
                depth: 1,
            },
        };

        let bytes_per_row = width as usize * 4;
        let row_offset = y_offset as usize * bytes_per_row;
        let data_ptr = NonNull::new(pixels[row_offset..].as_ptr() as *mut _)
            .expect("pixel data should be non-null");

        unsafe {
            texture.replaceRegion_mipmapLevel_withBytes_bytesPerRow(
                region,
                0,
                data_ptr,
                bytes_per_row,
            );
        }
    }

    /// Create a buffer for instance data.
    pub fn create_instance_buffer(
        &self,
        size: usize,
    ) -> Option<Retained<ProtocolObject<dyn MTLBuffer>>> {
        self.device
            .newBufferWithLength_options(size, MTLResourceOptions::StorageModeShared)
    }
}

/// Create a render pipeline state with the given vertex and fragment functions.
fn create_pipeline_state(
    device: &ProtocolObject<dyn MTLDevice>,
    library: &ProtocolObject<dyn MTLLibrary>,
    vertex_name: &str,
    fragment_name: &str,
) -> Result<Retained<ProtocolObject<dyn MTLRenderPipelineState>>, PipelineError> {
    let vertex_fn = library
        .newFunctionWithName(&objc2_foundation::NSString::from_str(vertex_name))
        .ok_or_else(|| PipelineError::FunctionNotFound(vertex_name.to_string()))?;

    let fragment_fn = library
        .newFunctionWithName(&objc2_foundation::NSString::from_str(fragment_name))
        .ok_or_else(|| PipelineError::FunctionNotFound(fragment_name.to_string()))?;

    let desc = MTLRenderPipelineDescriptor::new();
    desc.setVertexFunction(Some(&vertex_fn));
    desc.setFragmentFunction(Some(&fragment_fn));

    // Set pixel format to match CAMetalLayer (BGRA8Unorm is the macOS default)
    let color_attachment = unsafe { desc.colorAttachments().objectAtIndexedSubscript(0) };
    color_attachment.setPixelFormat(MTLPixelFormat::BGRA8Unorm);

    // Enable alpha blending for text and overlays
    color_attachment.setBlendingEnabled(true);
    color_attachment.setSourceRGBBlendFactor(MTLBlendFactor::SourceAlpha);
    color_attachment.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
    color_attachment.setSourceAlphaBlendFactor(MTLBlendFactor::One);
    color_attachment.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);

    device
        .newRenderPipelineStateWithDescriptor_error(&desc)
        .map_err(|e| PipelineError::PipelineCreation(e.to_string()))
}
