# Cell
A basic thin wrapper that allows `Anarchy` and `shader_magician` to work together inside a `wgpu` and `winit` app.  `Anarchy` provides an
effecient, easy to use ECS, and `shader_magician` allows simplifies the boilerplate of `wgpu` as well as provides for capabilities like
writing shaders in Rust instead of WGSL.  This project adds a basic `App` struct that builds and runs a `winit` and `wgpu` app via `Plugin`s
that can be added via the `add_plugin` function.  This also allows for adding systems to the primary or render schedules, as well as setting
up any resources that may be needed by the App.  One for thing this project adds is a easy to use Egui integration for use in debugging and 
in simple apps.  Heres an example of a simple App using `Cell`:

```rust
use anarchy::{Res, ResMut, macros::{Resource, system}};
use cell::{App, EguiCtx, EguiPlugin};

#[derive(Resource)]
pub struct TestData {
    speed: f32
}

fn main() -> anyhow::Result<()> {
    App::new()
        .add_plugin(EguiPlugin)
        .on_render_update(on_render_update)
        .add_resource(TestData { speed: 0.5 })
        .run()
}

#[system]
fn on_render_update(
    egui: Res<EguiCtx>,
    test_data: ResMut<TestData>
) {
    egui::Window::new("Debug").show(&egui.context, |ui| {
        ui.label("Hello from egui!");
        ui.add(egui::Slider::new(&mut test_data.speed, 0.0..=1.0).text("Speed"));
    });
}
```
