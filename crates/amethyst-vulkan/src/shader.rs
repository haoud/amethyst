use crate::device::RenderDevice;
use shaderc::{CompileOptions, Compiler};
use std::{borrow::Cow, sync::Arc};
use vulkanalia::prelude::v1_2::*;

/// A shader.
pub struct Shader {
    device: Arc<RenderDevice>,
    inner: vk::ShaderModule,
    entry: String,
    kind: ShaderType,
}

impl Shader {
    /// Compiles a shader.
    ///
    /// # Warning
    /// To use this function, the Vulkan SDK must be installed and in the `PATH`
    /// environment variable. This is because the shader compiler is external to
    /// the engine.
    #[must_use]
    pub fn compile(device: Arc<RenderDevice>, info: ShaderCompileInfo) -> Self {
        let options = CompileOptions::new().expect("Failed to create shader compile options");
        let compiler = Compiler::new().expect("Failed to create shader compiler");

        // Get the shader source and language, as well as some metadata
        let (path, data) = match info.source {
            ShaderSource::Code(data) => ("(no file)", Cow::Borrowed(data)),
            ShaderSource::File(path) => {
                let data = std::fs::read_to_string(path).expect("Failed to read shader file");
                (path, Cow::Owned(data))
            }
        };

        // Compile or assemble the shader depending on the language
        let artefact = match info.language {
            ShaderSourceType::SpirV => compiler
                .assemble(&data, Some(&options))
                .expect("Failed to assemble the shader"),

            ShaderSourceType::GLSL => compiler
                .compile_into_spirv(&data, info.kind.into(), path, info.entry, Some(&options))
                .expect("Failed to compile the shader"),
        };

        let bytecode = artefact.as_binary();
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code_size(bytecode.len() * 4)
            .code(bytecode)
            .build();

        let inner = unsafe {
            device
                .logical()
                .inner()
                .create_shader_module(&create_info, None)
                .expect("Failed to create the shader module")
        };

        let entry = info.entry.to_owned() + "\0";

        Self {
            kind: info.kind,
            device,
            entry,
            inner,
        }
    }

    /// Returns the raw Vulkan handle of the shader module.
    #[must_use]
    pub(crate) fn inner(&self) -> vk::ShaderModule {
        self.inner
    }

    /// Returns the entry point name of the shader. The end of the string is
    /// guaranteed to be a null character.
    #[must_use]
    pub fn entry(&self) -> &str {
        &self.entry
    }

    /// Returns the type of the shader.
    #[must_use]
    pub fn kind(&self) -> ShaderType {
        self.kind
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.device
                .logical()
                .inner()
                .destroy_shader_module(self.inner, None);
        }
    }
}

/// Information required to compile a shader.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ShaderCompileInfo<'a> {
    /// The source language of the shader. Defaults to GLSL.
    pub language: ShaderSourceType,

    /// The source of the shader. It can either be the source code itself, or a
    /// path to a file containing the source code. By default, the source is
    /// an empty string.
    pub source: ShaderSource<'a>,

    /// The type of the shader. Defaults to vertex.
    pub kind: ShaderType,

    /// The entry point of the shader. Defaults to `main`.
    pub entry: &'a str,
}

impl Default for ShaderCompileInfo<'_> {
    fn default() -> Self {
        Self {
            language: ShaderSourceType::GLSL,
            source: ShaderSource::Code(""),
            kind: ShaderType::Vertex,
            entry: "main",
        }
    }
}

/// The source of a shader. This can either be directly the source code, or a file
/// path to the source code.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ShaderSource<'a> {
    /// The source code of the shader.
    Code(&'a str),

    /// The path to the source code of the shader.
    File(&'a str),
}

/// The source language of a shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderSourceType {
    SpirV,
    GLSL,
}

/// The type of a shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShaderType {
    Geometry,
    Fragment,
    Compute,
    Vertex,
}

impl From<shaderc::ShaderKind> for ShaderType {
    fn from(kind: shaderc::ShaderKind) -> Self {
        match kind {
            shaderc::ShaderKind::Geometry => Self::Geometry,
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
            ShaderType::Geometry => Self::Geometry,
            ShaderType::Fragment => Self::Fragment,
            ShaderType::Compute => Self::Compute,
            ShaderType::Vertex => Self::Vertex,
        }
    }
}

bitflags::bitflags! {
    /// A set of shader stages.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ShaderStages: u32 {
        /// The vertex shader stage.
        const VERTEX = vk::ShaderStageFlags::VERTEX.bits();

        /// The tessellation control shader stage.
        const TESSELLATION_EVALUATION = vk::ShaderStageFlags::TESSELLATION_EVALUATION.bits();

        /// The tessellation evaluation shader stage.
        const TESSELLATION_CONTROL = vk::ShaderStageFlags::TESSELLATION_CONTROL.bits();

        /// The geometry shader stage.
        const GEOMETRY = vk::ShaderStageFlags::GEOMETRY.bits();

        /// The fragment shader stage.
        const FRAGMENT = vk::ShaderStageFlags::FRAGMENT.bits();

        /// The compute shader stage.
        const COMPUTE = vk::ShaderStageFlags::COMPUTE.bits();

        /// FIXME: If vulkan shader stages extension will be added to this
        /// structure, a comment should be added to explain that this flags
        /// only contains the core stages included in the Vulkan 1.0 specification.
        const ALL = vk::ShaderStageFlags::ALL_GRAPHICS.bits();
    }
}

impl From<ShaderStages> for vk::ShaderStageFlags {
    fn from(flags: ShaderStages) -> Self {
        Self::from_bits_truncate(flags.bits())
    }
}
