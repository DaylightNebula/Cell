use std::{sync::Arc, time::{Duration, Instant}};

use anarchy::EventTracker;
use winit::{
    application::ApplicationHandler,
    event::StartCause,
    event_loop::{ActiveEventLoop, ControlFlow},
    window::{Window, WindowId},
};

use crate::{App, DeviceEvent, WindowEvent};

pub(crate) struct AppWrapper {
    pub(crate) app: App,
    /// Earliest time the next frame may render, `None` until the first capped frame.
    pub(crate) next_frame: Option<Instant>,
}

impl AppWrapper {
    /// Returns the deadline to wait for if the render rate cap says it's too early to render,
    /// otherwise claims the current frame slot and returns `None`.
    fn frame_too_early(&mut self) -> Option<Instant> {
        // web redraws are already paced by requestAnimationFrame, and `Instant::now` panics there
        if cfg!(target_arch = "wasm32") { return None }
        let rate = self.app.render_schedule_id.tick_rate;
        if rate == 0 { return None }

        let now = Instant::now();
        let period = Duration::from_secs(1) / rate;
        match self.next_frame {
            Some(next) if now < next => return Some(next),
            // keep cadence from the last deadline so frames don't drift, but never bank a backlog
            Some(next) => self.next_frame = Some((next + period).max(now)),
            None => self.next_frame = Some(now + period),
        }
        None
    }

    fn present_mode(&self) -> wgpu::PresentMode {
        if self.app.vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync }
    }
}

impl ApplicationHandler<()> for AppWrapper {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        // a frame was deferred by the render rate cap, its time has come
        if let StartCause::ResumeTimeReached { .. } = cause {
            event_loop.set_control_flow(ControlFlow::Wait);
            if let Some(graphics) = self.app.world.get_resource_ref::<crate::Graphics>() {
                graphics.window().request_redraw();
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::{JsCast, UnwrapThrowExt};
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we are not on web we can use pollster to
            // await the window creation
            // self.state = Some(pollster::block_on(State::new(window)).unwrap());

            use crate::{Frame, Graphics};
            use magician_vgpu::VirtualGpu;

            let mut vgpu = pollster::block_on(VirtualGpu::new(window));
            // applied when the first resize configures the surface
            vgpu.config_mut().present_mode = self.present_mode();
            self.app.world.insert_resource(Graphics(vgpu));
            self.app.world.insert_resource(Frame::default());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // wasm can't block on the GPU setup future, so spawn it and hand the
            // resources to the (shared, clonable) world once it resolves
            use crate::{Frame, Graphics};
            use magician_vgpu::VirtualGpu;

            let world = self.app.world.clone();
            let present_mode = self.present_mode();
            wasm_bindgen_futures::spawn_local(async move {
                let mut vgpu = VirtualGpu::new(window).await;
                vgpu.config_mut().present_mode = present_mode;
                world.insert_resource(Graphics(vgpu));
                world.insert_resource(Frame::default());
            });
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let Some(event_tracker) = self.app.world.get_resource_ref::<EventTracker>() {
            event_tracker.broadcast_event(DeviceEvent(event));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        {
            if let Some(event_tracker) = self.app.world.get_resource_ref::<EventTracker>() {
                event_tracker.broadcast_event(WindowEvent(event.clone()));
            }
        }

        match &event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::Resized(size) => self.app.resize(size.width, size.height),
            winit::event::WindowEvent::RedrawRequested => {
                if let Some(deadline) = self.frame_too_early() {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                    return;
                }
                let _ = self.app.render();
            }
            _ => {}
        }
    }
}
