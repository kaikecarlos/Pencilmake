use ash::vk;

use crate::core::device::RendererDevice;
use crate::core::shader::Shader;

use std::ffi;

use anyhow::Result;

use super::object::vertex::Vertex;
pub struct RendererPipeline {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
}

impl RendererPipeline {
    pub fn new(
        device: &RendererDevice,
        extent: vk::Extent2D,
        render_pass: vk::RenderPass
    ) -> Result<RendererPipeline> {
        let vert = Shader::from_code_vert(
            &device.logical_device,
            vk_shader_macros::include_glsl!("./src/shaders/default.vert")
        )?;
        let frag = Shader::from_code_frag(
            &device.logical_device,
            vk_shader_macros::include_glsl!("./src/shaders/default.frag")
        )?;

        let entry_point = ffi::CString::new("main").unwrap();

        let shader_stages = [
            vert.shader_stage(&entry_point),
            frag.shader_stage(&entry_point),
        ];

        println!("Etapa dos shaders finalizado.");

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder();

        println!("Informações de vertice obtidas");

        let (pipeline_layout, pipeline) = Self::create_graphics_pipeline(
            &device.logical_device,
            render_pass,
            extent,
            vertex_input_info,
            &shader_stages
        ).expect("Falha em criar a pipeline de gráficos");

        println!("Pipeline criada!");
        unsafe {
            vert.cleanup(&device.logical_device);
            frag.cleanup(&device.logical_device);
        }

        Ok(RendererPipeline {
            pipeline,
            pipeline_layout,
        })
    }

    fn create_graphics_pipeline(
        device: &ash::Device,
        render_pass: vk::RenderPass,
        extent: vk::Extent2D,
        vertex_input_info: vk::PipelineVertexInputStateCreateInfoBuilder,
        shader_stages: &[vk::PipelineShaderStageCreateInfo]
    ) -> Result<(vk::PipelineLayout, vk::Pipeline)> {
        // vertex:
        let binding_descriptions = [Vertex::get_binding_description()];
        let attribute_descriptions = Vertex::get_attribute_descriptions();
        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&binding_descriptions)
            .vertex_attribute_descriptions(&attribute_descriptions)
            .build();
        // input:

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

        // viewport:

        let viewports = [
            vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 0.0,
            }
        ];

        let scissors = [
            vk::Rect2D {
                offset: vk::Offset2D {
                    x: 0,
                    y: 0,
                },
                extent,
            }
        ];

        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        // rasterizer:

        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .cull_mode(vk::CullModeFlags::NONE)
            .polygon_mode(vk::PolygonMode::FILL);

        // multisampler:

        let multisampler_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        // color blend:

        let color_blend_attachments = [
            vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                )
                .build()
        ];

        let color_blend_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .attachments(&color_blend_attachments);

        // pipeline:

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder();
        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&pipeline_layout_info, None)?
        };

        println!("Pipeline layout criada!");
        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(shader_stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampler_info)
            .color_blend_state(&color_blend_info)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0)
            .build();
        
        println!("Pipeline info criada!");
        let pipeline = unsafe {
            device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ).unwrap()
        }[0];
        println!("Created pipeline {:?}", pipeline);
        Ok((pipeline_layout, pipeline))
    }

    pub unsafe fn cleanup(&self, device: &ash::Device) {
        device.destroy_pipeline(self.pipeline, None);
        device.destroy_pipeline_layout(self.pipeline_layout, None);
    }
}