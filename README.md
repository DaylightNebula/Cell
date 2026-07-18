# Cell

> **Work in progress.** The API is still taking shape and may change without notice. Use at your own risk.

A thin wrapper that lets [`anarchy`](../anarchy) and [`shader_magician`](../shader_magician) work
together inside a `winit` + `wgpu` application. `Anarchy` provides an efficient, easy to use ECS,
and `shader_magician` simplifies `wgpu` boilerplate and lets you write shaders in Rust instead of
WGSL.

`Cell` adds an `App` builder that:
- owns an `anarchy` `World` and creates the `winit` window/event loop
- runs a primary schedule on a fixed tick rate, independent of rendering
- runs a render schedule once per frame (`RedrawRequested`), and submits the resulting `wgpu` frame
- broadcasts `winit` window events into the ECS as `WindowEvent`
- lets functionality be packaged and shared as `Plugin`s
- optionally sets up an `egui` integration for debugging and simple in-app UI

## Features

| Feature | Enables |
|---|---|
| `auto-egui` | Automatically adds `EguiPlugin` when `App::new()` is called. |
| `dbg-schedules` | Implies `auto-egui`. Adds an egui window showing the FPS of every running schedule. |

## Usage

### Minimal app (no rendering UI)

```rust
use anarchy::{ResMut, macros::{Resource, system}};
use cell::App;

#[derive(Resource)]
struct Counter(u32);

fn main() -> anyhow::Result<()> {
    App::new()
        .add_resource(Counter(0))
        .on_update(tick)
        .run()
}

#[system]
fn tick(counter: ResMut<Counter>) {
    counter.0 += 1;
    println!("tick {}", counter.0);
}
```

### App with an egui debug window

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

Run the bundled example with:

```sh
cargo run --example egui --features auto-egui
```

### Writing a plugin

```rust
use cell::{App, Plugin};

struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(self, app: App) -> App {
        app.on_startup(my_startup_system)
            .on_update(my_update_system)
    }
}
```

Plugins are deduplicated by type: calling `add_plugin` twice with the same plugin type only
builds it once.
