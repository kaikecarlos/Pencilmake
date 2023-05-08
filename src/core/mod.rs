pub mod debug;
pub mod device;
pub mod window;
pub mod swapchain;
pub mod pipeline;
pub mod shader;
pub mod commandpool;
pub mod object;

use device::RendererDevice;
use window::RendererWindow;
use swapchain::RendererSwapchain;
use debug::RendererDebug;
use pipeline::RendererPipeline;
use commandpool::CommandPools;


use ash::vk;
use ash::extensions::{ext, khr};
use anyhow::Result;
use std::ffi;
use std::ptr::copy_nonoverlapping as memcpy;
use raw_window_handle::HasRawDisplayHandle;

use self::object::vertex::{Vertex};

pub struct VulkanRenderer {
    pub instance: ash::Instance,
    pub main_device: RendererDevice,
    pub window: RendererWindow,
    pub swapchain: RendererSwapchain,
    pub debug: RendererDebug,
    pub render_pass: vk::RenderPass,
    pub graphics_pipeline: RendererPipeline,
    pub command_pools: CommandPools,
    pub graphics_command_buffers: Vec<vk::CommandBuffer>,
    pub vertex_buffer: vk::Buffer,
    pub index_buffer: vk::Buffer,
    pub model_index_count: usize,
}


impl VulkanRenderer {
    fn used_layer_names() -> Vec<ffi::CString> {
        vec![
            // ffi::CString::new("VK_LAYER_KHRONOS_validation").unwrap()
        ]
    }

    fn used_extensions() -> Vec<*const i8> {
        vec![
            ext::DebugUtils::name().as_ptr(),
            khr::Surface::name().as_ptr(),
        ]
    }


    pub fn new() -> Result<Self> {
        let (event_loop, window) = RendererWindow::create_window()?;
        let raw_display_handle = window.raw_display_handle();
        window.set_title("Pencilmake");

        

        let used_layer_names = Self::used_layer_names();
        let used_layers: Vec<_> = used_layer_names.iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();
        

        println!("Used layers:");
        for layer in used_layers.iter() {
            unsafe {
                let layer_name = std::ffi::CStr::from_ptr(*layer).to_str().unwrap();
                println!("  {}", layer_name);
            }
        }
            
        let mut used_extensions = Self::used_extensions();


        let extension_names = ash_window::enumerate_required_extensions(raw_display_handle)?;
        for extension_name in extension_names.iter() {
            used_extensions.push(*extension_name);
        };

        println!("Used extensions:");
        for extension in used_extensions.iter() {
            unsafe {
                let extension_name = std::ffi::CStr::from_ptr(*extension).to_str().unwrap();
                println!("  {}", extension_name);
            }
        }
        let entry = ash::Entry::linked();
        let instance = Self::create_instance(&entry, &used_layers, &used_extensions)?;
        let window = RendererWindow::new(event_loop, window, &entry, &instance)?;
        let debug = RendererDebug::new(&entry, &instance)?;

        let main_device = match RendererDevice::new(&instance, &used_layers)? {
            None => panic!("Nenhum dispositivo foi encontrado"),
            Some(dev) => dev
        };

        let render_pass = Self::create_render_pass(&main_device, &window)?;

        let mut swapchain = RendererSwapchain::new(&instance, &main_device, &window)?;
        swapchain.create_framebuffers(&main_device, render_pass)?;

        let graphics_pipeline = RendererPipeline::new(&main_device, swapchain.extent, render_pass)?;
        let command_pools = CommandPools::new(&main_device)?;

        println!("There is {} framebuffers", swapchain.framebuffers.len());
        let graphics_command_buffers = CommandPools::create_command_buffers(&main_device, command_pools.graphics, swapchain.framebuffers.len() as u32)?;

        let (vertices, indices) = object::model::load_model("models", "duck.obj");
        let (vertex_buffer, _) = Self::create_vertex_buffer(
            &main_device, 
            &instance,
            command_pools.graphics, 
            main_device.graphics_queue,
            &vertices,
        );

        let (index_buffer, _) = Self::create_index_buffer(
            &main_device,
            &instance,
            command_pools.graphics,
            main_device.graphics_queue,
            &indices
        );

        let renderer = Self {
            instance,
            main_device,
            window,
            debug,
            swapchain,
            render_pass,
            graphics_pipeline,
            command_pools,
            graphics_command_buffers,
            vertex_buffer,
            index_buffer,
            model_index_count: indices.len()
        };

        renderer.fill_command_buffers().expect("Falha ao preencher buffers de comando");
        Ok(renderer)
    }

    fn create_instance(entry: &ash::Entry, layer_name_pts: &Vec<*const i8>, extension_name_pts: &Vec<*const i8>) -> Result<ash::Instance> {
        let app_name = std::ffi::CString::new("Pencilmake")?;
        let engine_name = std::ffi::CString::new("Pencilmake Engine")?;

        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);
        
        let instance_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(layer_name_pts)
            .enabled_extension_names(extension_name_pts);

        let instance = unsafe {
            entry.create_instance(&instance_info, None)?
        };
        Ok(instance)
    }

    fn create_render_pass(device: &RendererDevice, window: &RendererWindow) -> Result<vk::RenderPass> {
        let formats = window.formats(device.physical_device)?;
        let format = formats.first().unwrap();

        let attachments = [
            vk::AttachmentDescription::builder()
                .format(format.format)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .initial_layout(vk::ImageLayout::UNDEFINED)
                .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .samples(vk::SampleCountFlags::TYPE_1)
                .build()
        ];

        let color_attachment_references = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];

        let subpasses = [
            vk::SubpassDescription::builder()
                .color_attachments(&color_attachment_references)
                .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                .build()
        ];

        let subpass_dependencies = [
            vk::SubpassDependency::builder()
                .src_subpass(vk::SUBPASS_EXTERNAL)
                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_subpass(0)
                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .build()
        ];

        let render_pass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&subpass_dependencies);

        let render_pass = unsafe {
            device.logical_device.create_render_pass(&render_pass_info, None)?
        };

        Ok(render_pass)
    }
    
    fn get_mem_properties(device: &RendererDevice, instance: &ash::Instance) -> vk::PhysicalDeviceMemoryProperties {
        unsafe {
            instance
                .get_physical_device_memory_properties(device.physical_device)
        }
    }

    fn find_memory_type(
        requirements: vk::MemoryRequirements,
        mem_properties: vk::PhysicalDeviceMemoryProperties,
        required_properties: vk::MemoryPropertyFlags,
    ) -> u32 {
        for i in 0..mem_properties.memory_type_count {
            if requirements.memory_type_bits & (1 << i) != 0
                && mem_properties.memory_types[i as usize]
                    .property_flags
                    .contains(required_properties)
            {
                return i;
            }
        }
        panic!("Failed to find suitable memory type.")
    }

    fn create_buffer(
        instance: &ash::Instance,
        device: &RendererDevice,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        mem_properties: vk::MemoryPropertyFlags,
    ) -> (vk::Buffer, vk::DeviceMemory, vk::DeviceSize) {
        let buffer = {
            let buffer_info = vk::BufferCreateInfo::builder()
                .size(size)
                .usage(usage)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build();
            unsafe { device.logical_device.create_buffer(&buffer_info, None).unwrap() }
        };

        let mem_requirements = unsafe { device.logical_device.get_buffer_memory_requirements(buffer) };
        let memory = {
            let mem_type = Self::find_memory_type(
                mem_requirements,
                Self::get_mem_properties(device, instance),
                mem_properties,
            );

            let alloc_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(mem_requirements.size)
                .memory_type_index(mem_type)
                .build();
            unsafe { device.logical_device.allocate_memory(&alloc_info, None).unwrap() }
        };

        unsafe { device.logical_device.bind_buffer_memory(buffer, memory, 0).unwrap() };

        (buffer, memory, mem_requirements.size)

    }

    fn create_device_local_buffer_with_data<A, T: Copy>(
        instance: &ash::Instance,
        device: &RendererDevice,
        command_pool: vk::CommandPool,
        transfer_queue: vk::Queue,
        usage: vk::BufferUsageFlags,
        data: &[T],
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let l_device = &device.logical_device;
        let size = (data.len() * std::mem::size_of::<T>()) as vk::DeviceSize;
        let (staging_buffer, staging_memory, staging_mem_size) = Self::create_buffer(
            instance,
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        );

        unsafe {
            let data_ptr = l_device
                .map_memory(staging_memory, 0, size, vk::MemoryMapFlags::empty())
                .unwrap();
            let mut align = ash::util::Align::new(data_ptr, std::mem::align_of::<A>() as _, staging_mem_size);
            align.copy_from_slice(data);
            l_device.unmap_memory(staging_memory);
        };

        let (buffer, memory, _) = Self::create_buffer(
            instance,
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_DST | usage,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        Self::copy_buffer(
            device,
            command_pool,
            transfer_queue,
            staging_buffer,
            buffer,
            size,
        );

        unsafe {
            l_device.destroy_buffer(staging_buffer, None);
            l_device.free_memory(staging_memory, None);
        };

        (buffer, memory)
    }
    fn copy_buffer(
        device: &RendererDevice,
        command_pool: vk::CommandPool,
        transfer_queue: vk::Queue,
        src: vk::Buffer,
        dst: vk::Buffer,
        size: vk::DeviceSize,
    ) {
        Self::execute_one_time_commands(device, command_pool, transfer_queue, |buffer| {
            let region = vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size,
            };
            let regions = [region];

            unsafe { device.logical_device.cmd_copy_buffer(buffer, src, dst, &regions) };
        });
    }

    fn execute_one_time_commands<F: FnOnce(vk::CommandBuffer)>(
        device: &RendererDevice,
        command_pool: vk::CommandPool,
        queue: vk::Queue,
        executor: F,
    ) {
        let command_buffer = {
            let alloc_info = vk::CommandBufferAllocateInfo::builder()
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_pool(command_pool)
                .command_buffer_count(1)
                .build();

            unsafe { device.logical_device.allocate_command_buffers(&alloc_info).unwrap()[0] }
        };
        let command_buffers = [command_buffer];

        // Begin recording
        {
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                .build();
            unsafe {
                device
                    .logical_device.begin_command_buffer(command_buffer, &begin_info)
                    .unwrap()
            };
        }

        // Execute user function
        executor(command_buffer);

        // End recording
        unsafe { device.logical_device.end_command_buffer(command_buffer).unwrap() };

        // Submit and wait
        {
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&command_buffers)
                .build();
            let submit_infos = [submit_info];
            unsafe {
                device.logical_device.
                    queue_submit(queue, &submit_infos, vk::Fence::null())
                    .unwrap();
                device.logical_device.queue_wait_idle(queue).unwrap();
            };
        }

        // Free
        unsafe { device.logical_device.free_command_buffers(command_pool, &command_buffers) };
    }



    fn create_vertex_buffer(
        device: &RendererDevice,
        instance: &ash::Instance,
        command_pool: vk::CommandPool,
        transfer_queue: vk::Queue,
        vertices: &[Vertex],
    ) -> (vk::Buffer, vk::DeviceMemory) {
        Self::create_device_local_buffer_with_data::<u32, _>(
            instance,
            device,
            command_pool,
            transfer_queue,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vertices,
        )
    }

    fn create_index_buffer(
        device: &RendererDevice,
        instance: &ash::Instance,
        command_pool: vk::CommandPool,
        transfer_queue: vk::Queue,
        indices: &[u32],
    ) -> (vk::Buffer, vk::DeviceMemory) {
        Self::create_device_local_buffer_with_data::<u16, _>(
            instance,
            device,
            command_pool,
            transfer_queue,
            vk::BufferUsageFlags::INDEX_BUFFER,
            indices,
        )
    }

    fn get_memory_type_index(
        device: &RendererDevice,
        instance: &ash::Instance,
        properties: vk::MemoryPropertyFlags,
        requirements: vk::MemoryRequirements
    ) -> Result<u32> {
        let memory = unsafe {
            instance.get_physical_device_memory_properties(device.physical_device)
        };
        (0..memory.memory_type_count)
            .find(|i| {
                let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
                let memory_type = memory.memory_types[*i as usize];
                suitable && memory_type.property_flags.contains(properties)
            })
            .ok_or_else(|| panic!("Failed to find suitable memory type."))
    }

    fn fill_command_buffers(&self) -> Result<()> {
        for (i, &command_buffer) in self.graphics_command_buffers.iter().enumerate() {
            let begin_info = vk::CommandBufferBeginInfo::builder();

            unsafe {
                self.main_device.logical_device.begin_command_buffer(command_buffer, &begin_info).expect("Falha ao iniciar o buffer de comandos")
            };

            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    }
                },
            ];
            println!("Render pass: {:?}", self.render_pass);
            println!("Framebuffer: {:?}", self.swapchain.framebuffers[i]);
            println!("Render area: {:?}", vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: self.swapchain.extent,
                        });
            
            let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                .render_pass(self.render_pass)
                .framebuffer(self.swapchain.framebuffers[i])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain.extent,
                })
                .clear_values(&clear_values);

            println!("Initiate render pass");

            unsafe {
                self.main_device.logical_device.cmd_begin_render_pass(
                    command_buffer,
                    &render_pass_begin_info,
                    vk::SubpassContents::INLINE,
                );

                self.main_device.logical_device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.graphics_pipeline.pipeline,
                );
                
                self.main_device.logical_device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer], &[0]);
                self.main_device.logical_device.cmd_bind_index_buffer(command_buffer, self.index_buffer, 0, vk::IndexType::UINT32);

                self.main_device.logical_device.cmd_draw(command_buffer, self.model_index_count as _, 1, 0, 0);

                self.main_device.logical_device.cmd_end_render_pass(command_buffer);

                self.main_device.logical_device.end_command_buffer(command_buffer)?;
            };
        }

        Ok(())
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            self.main_device.logical_device.device_wait_idle().unwrap();
            self.command_pools.cleanup(&self.main_device);
            self.graphics_pipeline.cleanup(&self.main_device.logical_device);
            self.main_device.logical_device.destroy_render_pass(self.render_pass, None);
            self.main_device.logical_device.destroy_buffer(self.index_buffer, None);
            self.main_device.logical_device.destroy_buffer(self.vertex_buffer, None);
            self.debug.cleanup();
            self.swapchain.cleanup(&self.main_device);
            self.main_device.logical_device.destroy_buffer(self.vertex_buffer, None);
            self.window.cleanup();
            self.main_device.cleanup();
            self.instance.destroy_instance(None);
        }
    }
}
