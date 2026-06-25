use std::sync::Arc;

use anarchy::EventTracker;
use winit::{application::ApplicationHandler, event_loop::ActiveEventLoop, window::{Window, WindowId}};

use crate::{App, WindowEvent};

pub(crate) struct AppWrapper {
    pub(crate) app: App
}

impl ApplicationHandler<()> for AppWrapper {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
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

            use magician_vgpu::VirtualGpu;
            use crate::{Frame, Graphics};

            let vgpu = pollster::block_on(VirtualGpu::new(window));
            self.app.world.insert_resource(Graphics(vgpu));
            self.app.world.insert_resource(Frame::default());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            State::new(window)
                                .await
                                .expect("Unable to create canvas!!!")
                        )
                        .is_ok())
                });
            }
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {
        // #[cfg(target_arch = "wasm32")]
        // {
        //     event.window.request_redraw();
        //     event.resize(
        //         event.window.inner_size().width,
        //         event.window.inner_size().height,
        //     );
        // }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: winit::event::WindowEvent
    ) {
        if let Some(event_tracker) = self.app.world.get_resource_mut::<EventTracker>() {
            event_tracker.broadcast_event(WindowEvent(event.clone()));
        }

        match &event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::Resized(size) => self.app.resize(size.width, size.height),
            winit::event::WindowEvent::RedrawRequested => {
                let _ = self.app.render();
            }
            _ => {}
        }
    }
}
