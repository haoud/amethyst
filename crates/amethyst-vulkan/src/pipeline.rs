use crate::{
    buffer::{VertexAttributeDescription, VertexBindingDescription},
    descriptor::DescriptorSetLayout,
    device::RenderDevice,
    prelude::{ImageFormat, Swapchain},
    shader::{Shader, ShaderType},
};
use bitflags::bitflags;
use std::sync::Arc;
use vulkanalia::prelude::v1_2::*;

/// A pipeline object.
pub struct Pipeline {
    device: Arc<RenderDevice>,
    descriptor_set_layouts: Vec<DescriptorSetLayout>,
    layout: vk::PipelineLayout,
    inner: vk::Pipeline,
}

impl Pipeline {
    #[must_use]
    pub fn new<T>(
        device: Arc<RenderDevice>,
        swapchain: &Swapchain,
        info: PipelineCreateInfo,
    ) -> Self
    where
        T: VertexAttributeDescription + VertexBindingDescription,
    {
        // Create the pipeline layout from the descriptor set layouts
        let layouts = info
            .descriptor_set_layouts
            .iter()
            .map(|layout| layout.inner())
            .collect::<Vec<_>>();

        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&layouts)
            .build();

        let layout = unsafe {
            device
                .logical()
                .inner()
                .create_pipeline_layout(&layout_create_info, None)
                .expect("Failed to create pipeline layout")
        };

        // Create a pipeline shader stage create info for each shader
        let stages = info
            .shaders
            .iter()
            .map(|shader| {
                let stage = match shader.kind() {
                    ShaderType::Geometry => vk::ShaderStageFlags::GEOMETRY,
                    ShaderType::Fragment => vk::ShaderStageFlags::FRAGMENT,
                    ShaderType::Compute => vk::ShaderStageFlags::COMPUTE,
                    ShaderType::Vertex => vk::ShaderStageFlags::VERTEX,
                };
                vk::PipelineShaderStageCreateInfo::builder()
                    .name(shader.entry().as_bytes())
                    .module(shader.inner())
                    .stage(stage)
                    .build()
            })
            .collect::<Vec<_>>();

        // Create the vertex input state from the vertex type passed in
        // the generic parameter.
        let attribute_descriptions = T::attribute_descriptions()
            .into_iter()
            .map(|description| vk::VertexInputAttributeDescription::from(description))
            .collect::<Vec<_>>();

        // Create the binding description from the vertex type passed in
        // the generic parameter.
        let binding_descriptions = &[vk::VertexInputBindingDescription::from(
            T::binding_description(),
        )];

        // Create the input state from the attribute and binding description
        // of the vertex type.
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&attribute_descriptions)
            .vertex_binding_descriptions(binding_descriptions);

        // Create the input assembly state
        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        // Configure the static viewport
        let viewport = vk::Viewport::builder()
            .height(swapchain.extent().height as f32)
            .width(swapchain.extent().width as f32)
            .min_depth(0.0)
            .max_depth(1.0)
            .x(0.0)
            .y(0.0)
            .build();

        // Configure the static scissor
        let scissor = vk::Rect2D::builder()
            .extent(vk::Extent2D::from(swapchain.extent()))
            .offset(vk::Offset2D { x: 0, y: 0 })
            .build();

        // Create the viewport state
        let viewports = &[viewport];
        let scissors = &[scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(viewports)
            .scissors(scissors)
            .build();

        // Configure the rasterization state
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .polygon_mode(info.fill_mode.into())
            .front_face(info.front_face.into())
            .cull_mode(info.cull_mode.into())
            .rasterizer_discard_enable(false)
            .depth_clamp_enable(false)
            .depth_bias_enable(false)
            .line_width(1.0)
            .build();

        // Configure the multisample state
        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::_1)
            .sample_shading_enable(false)
            .build();

        let attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false)
            .build();

        let attachments = &[attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .blend_constants([0.0, 0.0, 0.0, 0.0])
            .logic_op(vk::LogicOp::COPY)
            .logic_op_enable(false)
            .attachments(attachments)
            .build();

        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_bounds_test_enable(false)
            .depth_write_enable(info.depth_write)
            .depth_test_enable(info.depth_test)
            .depth_compare_op(vk::CompareOp::LESS)
            .stencil_test_enable(false)
            .build();

        // Create the rendering info struct, since we use dynamic rendering
        // which is not included in the base pipeline create info struct.
        let format = [swapchain.format().format.into()];
        let mut rendering_info = vk::PipelineRenderingCreateInfo::builder()
            .depth_attachment_format(info.depth_format.into())
            .color_attachment_formats(&format)
            .build();

        // Register all the previous structs into the pipeline create infos
        let creat_info = vk::GraphicsPipelineCreateInfo::builder()
            .input_assembly_state(&input_assembly_state)
            .rasterization_state(&rasterization_state)
            .depth_stencil_state(&depth_stencil_state)
            .vertex_input_state(&vertex_input_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .viewport_state(&viewport_state)
            .stages(&stages)
            .layout(layout)
            .push_next(&mut rendering_info)
            .build();

        let inner = unsafe {
            device
                .logical()
                .inner()
                .create_graphics_pipelines(vk::PipelineCache::null(), &[creat_info], None)
                .expect("Failed to create graphics pipeline")
                .0[0]
        };

        Self {
            descriptor_set_layouts: info.descriptor_set_layouts,
            layout,
            device,
            inner,
        }
    }

    /// Returns a reference to the descriptor set layouts used by the pipeline.
    #[must_use]
    pub(crate) fn descriptor_set_layouts(&self) -> &[DescriptorSetLayout] {
        &self.descriptor_set_layouts
    }

    /// Returns the pipeline layout used by the pipeline.
    #[must_use]
    pub(crate) fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }

    /// Returns the inner pipeline handle.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::Pipeline {
        self.inner
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .inner()
                .destroy_pipeline_layout(self.layout, None);

            self.device
                .logical()
                .inner()
                .destroy_pipeline(self.inner, None);
        }
    }
}

/// A struct containing the information needed to create a pipeline.
pub struct PipelineCreateInfo {
    /// A list of descriptor set layouts to use for the pipeline. Defaults to an empty list,
    /// which means that the pipeline will not use any descriptor sets.
    pub descriptor_set_layouts: Vec<DescriptorSetLayout>,

    /// A list of shaders to use for the pipeline. Defaults to an empty list, which means that
    /// the pipeline will not use any shaders (Spoiler alert: this is not very useful)
    pub shaders: Vec<Shader>,

    /// The front face of the pipeline. This is used to determine if a face is a front face
    /// when culling. Defaults to `FrontFace::CounterClockwise`.
    pub front_face: FrontFace,

    /// The fill mode to use for the pipeline. Defaults to `FillMode::Fill`.
    pub fill_mode: FillMode,

    /// The cull mode to use for the pipeline. Defaults to `CullMode::Back`.
    pub cull_mode: CullMode,

    /// The format of the depth buffer. Defaults to `ImageFormat::D32SFLOAT`.
    pub depth_format: ImageFormat,

    /// Whether or not to enable depth writing. Defaults to `false`.
    pub depth_write: bool,

    /// Whether or not to enable depth testing. Defaults to `false`.
    pub depth_test: bool,
}

impl Default for PipelineCreateInfo {
    fn default() -> Self {
        Self {
            descriptor_set_layouts: Vec::new(),
            front_face: FrontFace::CounterClockwise,
            cull_mode: CullMode::Back,
            fill_mode: FillMode::Fill,
            depth_format: ImageFormat::D32SFLOAT,
            depth_write: false,
            depth_test: false,
            shaders: Vec::new(),
        }
    }
}

bitflags! {
    /// Pipeline stages flags. Each bit in the flags represents a pipeline stage.
    /// This is useful for synchronization between stages.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PipelineStage: u32 {
        const TOP_OF_PIPE = 1;
        const DRAW_INDIRECT = 1 << 1;
        const VERTEX_INPUT = 1 << 2;
        const VERTEX_SHADER = 1 << 3;
        const TESSELLATION_CONTROL_SHADER = 1 << 4;
        const TESSELLATION_EVALUATION_SHADER = 1 << 5;
        const GEOMETRY_SHADER = 1 << 6;
        const FRAGMENT_SHADER = 1 << 7;
        const EARLY_FRAGMENT_TESTS = 1 << 8;
        const LATE_FRAGMENT_TESTS = 1 << 9;
        const COLOR_ATTACHMENT_OUTPUT = 1 << 10;
        const COMPUTE_SHADER = 1 << 11;
        const TRANSFER = 1 << 12;
        const BOTTOM_OF_PIPE = 1 << 13;
        const HOST = 1 << 14;
        const ALL_GRAPHICS = 1 << 15;
        const ALL_COMMANDS = 1 << 16;
        const COMMAND_PREPROCESS_NV = 1 << 17;
        const CONDITIONAL_RENDERING_EXT = 1 << 18;
        const TASK_SHADER_EXT = 1 << 19;
        const MESH_SHADER_EXT = 1 << 20;
        const RAY_TRACING_SHADER_KHR = 1 << 21;
        const FRAGMENT_SHADING_RATE_ATTACHMENT_KHR = 1 << 22;
        const FRAGMENT_DENSITY_PROCESS_EXT = 1 << 23;
        const TRANSFORM_FEEDBACK_EXT = 1 << 24;
        const ACCELERATION_STRUCTURE_BUILD_KHR = 1 << 25;
    }
}

impl From<PipelineStage> for vk::PipelineStageFlags {
    fn from(value: PipelineStage) -> Self {
        Self::from_bits_truncate(value.bits())
    }
}

/// The operation to perform on an attachment when it is loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttachmentLoadOp {
    /// Clear the attachment with the clear colors.
    Clear,
}

impl From<AttachmentLoadOp> for vk::AttachmentLoadOp {
    fn from(value: AttachmentLoadOp) -> Self {
        match value {
            AttachmentLoadOp::Clear => vk::AttachmentLoadOp::CLEAR,
        }
    }
}

/// The operation to perform on an attachment when a store operation is
/// performed.
pub enum AttachmentStoreOp {
    /// Store the data in the attachment.
    Store,

    /// Discard the data, and does not store in the attachment.
    /// TODO: Explain potential performance benefits/usage.
    Discard,
}

impl From<AttachmentStoreOp> for vk::AttachmentStoreOp {
    fn from(value: AttachmentStoreOp) -> Self {
        match value {
            AttachmentStoreOp::Discard => vk::AttachmentStoreOp::DONT_CARE,
            AttachmentStoreOp::Store => vk::AttachmentStoreOp::STORE,
        }
    }
}

/// A enum representing in which order front faces vertices are defined.
/// This is used mainly to determine if a face is a front face or a back
/// face when culling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrontFace {
    /// Front faces vertices are defined in a counter-clockwise order.
    CounterClockwise,

    /// Front faces vertices are defined in a clockwise order.
    Clockwise,
}

impl From<FrontFace> for vk::FrontFace {
    fn from(value: FrontFace) -> Self {
        match value {
            FrontFace::CounterClockwise => vk::FrontFace::COUNTER_CLOCKWISE,
            FrontFace::Clockwise => vk::FrontFace::CLOCKWISE,
        }
    }
}

/// Specifies the culling mode to use for a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CullMode {
    /// Do not cull any faces.
    None,

    /// Cull the back faces. Back face are defined as faces that are not
    /// defined in the same order as the `FrontFace` enum used when creating
    /// the pipeline.
    Back,

    /// Cull the front faces. Front face are defined as faces that are defined
    /// in the same order as the `FrontFace` enum used when creating the
    /// pipeline.
    Front,

    /// Cull both the front and back faces. All faces will be culled. Honestly,
    /// I don't know why this is an option, but it may be useful for some
    /// cases I guess.
    FrontAndBack,
}

impl From<CullMode> for vk::CullModeFlags {
    fn from(value: CullMode) -> Self {
        match value {
            CullMode::None => vk::CullModeFlags::NONE,
            CullMode::Back => vk::CullModeFlags::BACK,
            CullMode::Front => vk::CullModeFlags::FRONT,
            CullMode::FrontAndBack => vk::CullModeFlags::FRONT_AND_BACK,
        }
    }
}

/// Specifies the fill mode to use when rasterizing a pipeline.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FillMode {
    /// Fill the polygon with the fragments generated by the rasterizer.
    #[default]
    Fill,

    /// Draw only the edges of the polygon.
    Line,

    /// Draw only the vertices of the polygon.
    Point,
}

impl From<FillMode> for vk::PolygonMode {
    fn from(value: FillMode) -> Self {
        match value {
            FillMode::Point => vk::PolygonMode::POINT,
            FillMode::Fill => vk::PolygonMode::FILL,
            FillMode::Line => vk::PolygonMode::LINE,
        }
    }
}
