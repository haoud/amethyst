use crate::device::VulkanDevice;
use std::sync::Arc;
use vulkanalia::prelude::v1_3::*;

/// A shader module. Shader modules are just a thin wrapper around the shader bytecode
/// and the functions defined in it. The compilation and linking of the SPIR-V bytecode
/// to machine code for execution by the GPU doesn't happen until the graphics pipeline
/// is created, and is encapsulated in the `Pipeline` struct. When a pipeline is created,
/// the shader modules used to create it are linked together and compiled into a single
/// program that can be executed by the GPU: therefore, the shader modules themselves
/// are not needed after the pipeline creation and can be safely dropped at that point.
#[derive(Debug)]
pub struct ShaderModule {
    device: Arc<VulkanDevice>,
    inner: vk::ShaderModule,
}

impl ShaderModule {
    /// Compiles the given GLSL code into a shader module.
    ///
    /// # Panics
    /// This method panics if the shader compilation fails.
    #[must_use]
    pub fn compile_glsl(device: Arc<VulkanDevice>, kind: ShaderType, code: String) -> Self {
        let options = shaderc::CompileOptions::new().unwrap();
        let compiler = shaderc::Compiler::new().expect("Failed to create shader compiler");
        let provenance = "(no provenance)";

        let artefact = compiler
            .compile_into_spirv(&code, kind.into(), provenance, "main", Some(&options))
            .expect("Failed to compile the shader");

        let bytecode = artefact.as_binary();
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code_size(bytecode.len() * 4)
            .code(bytecode)
            .build();

        let inner = unsafe {
            device
                .logical()
                .create_shader_module(&create_info, None)
                .expect("Failed to create the shader module")
        };

        Self { device, inner }
    }

    /// Returns the raw Vulkan handle of the shader module.
    #[must_use]
    pub fn inner(&self) -> vk::ShaderModule {
        self.inner
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .destroy_shader_module(self.inner, None);
        }
    }
}

/// The type of a shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

impl From<shaderc::ShaderKind> for ShaderType {
    fn from(kind: shaderc::ShaderKind) -> Self {
        match kind {
            shaderc::ShaderKind::Fragment => Self::Fragment,
            shaderc::ShaderKind::Compute => Self::Compute,
            shaderc::ShaderKind::Vertex => Self::Vertex,
            _ => panic!("Unsupported shader type"),
        }
    }
}

impl From<ShaderType> for shaderc::ShaderKind {
    fn from(kind: ShaderType) -> Self {
        match kind {
            ShaderType::Fragment => Self::Fragment,
            ShaderType::Compute => Self::Compute,
            ShaderType::Vertex => Self::Vertex,
        }
    }
}
