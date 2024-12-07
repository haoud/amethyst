use crate::{
    device::VulkanDevice,
    shader::{ShaderModule, ShaderType},
    swapchain::VulkanSwapchain,
};
use std::sync::Arc;
use vulkanalia::prelude::v1_3::*;

/// A pipeline object.
#[derive(Debug)]
pub struct Pipeline {
    device: Arc<VulkanDevice>,
    layout: vk::PipelineLayout,
    inner: vk::Pipeline,
}

impl Pipeline {
    /// Creates a new pipeline object. The generic parameter `T` is the type of the vertex
    /// data that will be passed to the vertex shader.
    #[must_use]
    pub fn new<T>(
        device: Arc<VulkanDevice>,
        swapchain: &VulkanSwapchain,
        info: PipelineCreateInfo,
    ) -> Self
    where
        T: VertexAttributeDescription + VertexBindingDescription,
    {
        // Create the pipeline layout.
        let layout = unsafe {
            device
                .logical()
                .create_pipeline_layout(&vk::PipelineLayoutCreateInfo::builder().build(), None)
                .expect("Failed to create pipeline layout")
        };

        // Create a pipeline shader stage create info for each shader
        let stages = info
            .shaders
            .iter()
            .map(|shader| {
                let stage = match shader.kind() {
                    ShaderType::Fragment => vk::ShaderStageFlags::FRAGMENT,
                    ShaderType::Compute => vk::ShaderStageFlags::COMPUTE,
                    ShaderType::Vertex => vk::ShaderStageFlags::VERTEX,
                };

                vk::PipelineShaderStageCreateInfo::builder()
                    .module(shader.inner())
                    .name(b"main\0")
                    .stage(stage)
                    .build()
            })
            .collect::<Vec<_>>();

        // Create the vertex input state and the vertex binding description from the vertex
        // type passed in the generic parameter and then create the vertex input state.
        let attribute_descriptions = T::attribute_descriptions();
        let binding_descriptions = T::binding_description();
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_attribute_descriptions(&attribute_descriptions)
            .vertex_binding_descriptions(&binding_descriptions);

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
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(swapchain.extent())
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
            .depth_write_enable(info.depth_write)
            .depth_test_enable(info.depth_test)
            .depth_bounds_test_enable(false)
            .depth_compare_op(vk::CompareOp::LESS)
            .stencil_test_enable(false)
            .build();

        // Create the rendering info struct, since we use dynamic rendering
        // which is not included in the base pipeline create info struct.
        let format = [swapchain.format()];
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
            .push_next(&mut rendering_info);

        let inner = unsafe {
            device
                .logical()
                .create_graphics_pipelines(vk::PipelineCache::null(), &[creat_info], None)
                .expect("Failed to create graphics pipeline")
                .0[0]
        };

        Self {
            layout,
            device,
            inner,
        }
    }

    /// Returns the pipeline layout used by the pipeline.
    #[must_use]
    pub fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }

    /// Returns the inner pipeline handle.
    #[must_use]
    pub fn inner(&self) -> vk::Pipeline {
        self.inner
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            let device = &self.device.logical();
            device.destroy_pipeline_layout(self.layout, None);
            device.destroy_pipeline(self.inner, None);
        }
    }
}

/// A struct containing the information needed to create a pipeline.
pub struct PipelineCreateInfo {
    /// A list of shaders to use for the pipeline.
    pub shaders: Vec<ShaderModule>,

    /// The front face of the pipeline. This is used to determine if a face is a front face
    /// when culling.
    pub front_face: vk::FrontFace,

    /// The fill mode to use for the pipeline.
    pub fill_mode: vk::PolygonMode,

    /// The cull mode to use for the pipeline.
    pub cull_mode: vk::CullModeFlags,

    /// The format of the depth buffer.
    pub depth_format: vk::Format,

    /// Whether or not to enable depth writing.
    pub depth_write: bool,

    /// Whether or not to enable depth testing.
    pub depth_test: bool,
}

impl Default for PipelineCreateInfo {
    fn default() -> Self {
        Self {
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            cull_mode: vk::CullModeFlags::NONE,
            fill_mode: vk::PolygonMode::FILL,
            depth_format: vk::Format::UNDEFINED,
            depth_write: false,
            depth_test: false,
            shaders: Vec::new(),
        }
    }
}

/// Specify the spacing between vertex data and and whether the data is per-vertex
/// or per-instance (instancing)
///
/// # Safety
/// This trait is unsafe because it requires the implementor to provide the exact layout of the
/// vertex data in memory. If the layout is incorrect, it can lead to undefined behavior.
pub unsafe trait VertexBindingDescription {
    fn binding_description() -> Vec<vk::VertexInputBindingDescription>;
}

/// Specify the type of the attributes passed to the vertex shader, which binding to load them
/// from and at which offset
///
/// # Safety
/// This trait is unsafe because it requires the implementor to provide the exact layout of the
/// vertex data in memory. If the layout is incorrect, it can lead to undefined behavior.
pub unsafe trait VertexAttributeDescription {
    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription>;
}

/// A struct representing a vertex with no attributes. This is useful for pipelines that don't
/// require vertex data.
#[derive(Debug, Clone, Copy)]
pub struct NoVertex;

unsafe impl VertexBindingDescription for NoVertex {
    fn binding_description() -> Vec<vk::VertexInputBindingDescription> {
        Vec::new()
    }
}

unsafe impl VertexAttributeDescription for NoVertex {
    fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        Vec::new()
    }
}
