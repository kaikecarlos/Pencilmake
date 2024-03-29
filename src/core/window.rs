use ash::vk;
use ash::extensions::khr;

use winit::event_loop::EventLoop;
use winit::window::Window;

use anyhow::Result;
use raw_window_handle::HasRawDisplayHandle;
use raw_window_handle::HasRawWindowHandle;

pub struct RendererWindow {
    pub event_loop: Option<EventLoop<()>>,
    pub window: Window,
    pub surface: vk::SurfaceKHR,
    pub surface_loader: khr::Surface,
}

impl RendererWindow {
    pub fn create_window() -> Result<(EventLoop<()>, Window)> {
        let event_loop = EventLoop::new();
        let window = Window::new(&event_loop)?;

        Ok((event_loop, window))
    }

    pub fn new(
        event_loop: EventLoop<()>,
        window: Window,
        entry: &ash::Entry,
        instance: &ash::Instance
    ) -> Result<RendererWindow> {
        let raw_display_window = window.raw_display_handle();
        let raw_window_handle = window.raw_window_handle();

        let surface = unsafe {
            ash_window::create_surface(entry, instance, raw_display_window, raw_window_handle, None)?
        };

        let surface_loader = khr::Surface::new(entry, instance);

        Ok(RendererWindow {
            event_loop: Some(event_loop),
            window,
            surface,
            surface_loader
        })
    }

    pub unsafe fn cleanup(&self) {
        self.surface_loader.destroy_surface(self.surface, None);
    }

    pub fn acquire_event_loop(&mut self) -> Result<EventLoop<()>> {
        match self.event_loop.take() {
            None => anyhow::bail!("EventLoop was acquired before"),
            Some(el) => Ok(el)
        }
    }

    pub fn capabilities(
        &self,
        physical_device: vk::PhysicalDevice
    ) -> Result<vk::SurfaceCapabilitiesKHR, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_capabilities(physical_device, self.surface)
        }
    }

    pub fn formats(
        &self,
        physical_device: vk::PhysicalDevice
    ) -> Result<Vec<vk::SurfaceFormatKHR>, vk::Result> {
        unsafe {
            self.surface_loader.get_physical_device_surface_formats(physical_device, self.surface)
        }
    }
}