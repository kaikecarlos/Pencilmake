mod core;
use ash::vk;
use winit::event::{Event, WindowEvent};
use crate::core::VulkanRenderer;
use anyhow::Result;


fn main() -> Result<()> {
    let mut renderer = VulkanRenderer::new()?;
    let event_loop = renderer.window.acquire_event_loop()?;

    let graphics_queue = match renderer.main_device.queue_family(vk::QueueFlags::GRAPHICS) {
        None => panic!("No graphics queue found"),
        Some(qf) => qf.queues[0]
    };

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            },
            Event::RedrawRequested(_) => {
                // acquiring next image:
                if renderer.swapchain.image_count > 0 {
                    renderer.swapchain.current_image = (renderer.swapchain.current_image + 1) % renderer.swapchain.image_count as usize;
                } else {
                    // Handle the case where image_count is zero.
                    // For example, you could choose a default value or log an error message.
                    println!("image_count is zero")
                }
                
                if renderer.swapchain.image_available.len() > 0 {
                    let (image_index, _) = unsafe {
                        renderer.swapchain.swapchain_loader.acquire_next_image(
                            renderer.swapchain.swapchain,
                            u64::MAX,
                            renderer.swapchain.image_available[renderer.swapchain.current_image],
                            vk::Fence::null(),
                        ).unwrap()
                    };
                    // fences:
                    unsafe {
                        let fences = [renderer.swapchain.may_begin_drawing[renderer.swapchain.current_image]];
    
                        renderer.main_device.logical_device.wait_for_fences(
                            &fences,
                            true,
                            u64::MAX,
                        ).unwrap();
    
                        renderer.main_device.logical_device.reset_fences(
                            &fences,
                        ).unwrap();
                    };
    
                    // submit:
                    let semaphores_available = [renderer.swapchain.image_available[renderer.swapchain.current_image]];
                    let waiting_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
                    let semaphores_finished = [renderer.swapchain.rendering_finished[renderer.swapchain.current_image]];
                    let command_buffers = [renderer.graphics_command_buffers[image_index as usize]];
    
                    let submit_info = [
                        vk::SubmitInfo::builder()
                            .wait_semaphores(&semaphores_available)
                            .wait_dst_stage_mask(&waiting_stages)
                            .command_buffers(&command_buffers)
                            .signal_semaphores(&semaphores_finished)
                            .build()
                    ];
    
                    unsafe {
                        renderer.main_device.logical_device.queue_submit(
                            graphics_queue,
                            &submit_info,
                            renderer.swapchain.may_begin_drawing[renderer.swapchain.current_image],
                        ).unwrap();
                    };
    
                    // present:
                    let swapchains = [renderer.swapchain.swapchain];
                    let indices = [image_index];
    
                    let present_info = vk::PresentInfoKHR::builder()
                        .wait_semaphores(&semaphores_finished)
                        .swapchains(&swapchains)
                        .image_indices(&indices);
    
                    unsafe {
                        renderer.swapchain.swapchain_loader
                            .queue_present(graphics_queue, &present_info)
                            .unwrap();
                    };
                }
            },
            _ => {}
        }
    });
}