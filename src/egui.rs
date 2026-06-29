use anarchy::{Event, EventSystemMinIDTracker, Res, ResMut, macros::{Resource, system}};
use magician_vgpu::{LoadOp, PassAttachment, PassTarget, StoreOp};

use crate::{App, Frame, Graphics, Plugin, WindowEvent};

pub struct EguiPlugin;
impl Plugin for EguiPlugin {
    fn build(self, app: App) -> App {
        app.on_render_startup(setup_egui)
            .on_render_update(start_egui_frame)
            .on_render_update(end_egui_frame)
    }
}

#[derive(Resource)]
pub struct EguiCtx {
    pub context: egui::Context,
    pub state: egui_winit::State,
    pub renderer: egui_wgpu::Renderer
}

#[system]
fn setup_egui(
    graphics: Res<Graphics>
) {
    // setup egui context, state, and renderer
    let context = egui::Context::default();
    let state = egui_winit::State::new(
        context.clone(), 
        egui::ViewportId::ROOT, 
        graphics.window(), 
        Some(graphics.window().scale_factor() as f32), 
        None, None
    );
    let renderer = egui_wgpu::Renderer::new(
        graphics.device(),
        graphics.config().format,
        egui_wgpu::RendererOptions::default()
    );

    world.insert_resource(EguiCtx {
        context, state, renderer
    });
}

static WINDOW_EVENT_TRACKER: EventSystemMinIDTracker = EventSystemMinIDTracker::new();

#[system(std::i32::MIN)]
fn start_egui_frame(
    graphics: Res<Graphics>,
    ctx: ResMut<EguiCtx>,
    frame: ResMut<Frame>,
    window_events: Event<WindowEvent>
) {
    for event in window_events.read(&WINDOW_EVENT_TRACKER) {
        let _ = ctx.state.on_window_event(graphics.window(), &**event);
    }
    
    let raw_input = ctx.state.take_egui_input(graphics.window());
    ctx.context.begin_pass(raw_input);
}

#[system(std::i32::MAX)]
fn end_egui_frame(
    graphics: Res<Graphics>,
    ctx: ResMut<EguiCtx>,
    frame: ResMut<Frame>,
) {
    let full_output = ctx.context.end_pass();
    ctx.state.handle_platform_output(graphics.window(), full_output.platform_output);

    let screen_descriptor = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [graphics.window().inner_size().width, graphics.window().inner_size().height],
        pixels_per_point: graphics.window().scale_factor() as f32,
    };

    let primitives = ctx.context.tessellate(
        full_output.shapes,
        screen_descriptor.pixels_per_point,
    );

    for (id, image_delta) in &full_output.textures_delta.set {
        ctx.renderer.update_texture(graphics.device(), graphics.queue(), *id, image_delta);
    }

    let encoder = frame.encoder_mut();
    ctx.renderer.update_buffers(graphics.device(), graphics.queue(), encoder, &primitives, &screen_descriptor);

    {
        let mut pass = frame.init_pass(
            &[
                PassAttachment {
                    target: PassTarget::PassOutput,
                    load_op: LoadOp::Load,
                    store_op: StoreOp::Store
                }
            ], 
            None
        );

        ctx.renderer.render(pass.pass_mut(), &primitives, &screen_descriptor);
    }

    for id in &full_output.textures_delta.free {
        ctx.renderer.free_texture(id);
    }
}
