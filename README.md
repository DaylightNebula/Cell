# Cell

> **Work in progress.** The API is still taking shape and may change without notice. Use at your own risk.

A thin wrapper that lets [`anarchy`](../anarchy) and [`shader_magician`](../shader_magician) work
together inside a `winit` + `wgpu` application. `Anarchy` provides an efficient, easy to use ECS,
and `shader_magician` simplifies `wgpu` boilerplate and lets you write shaders in Rust instead of
WGSL.

`Cell` adds an `App` builder that:
- owns an `anarchy` `World` and creates the `winit` window/event loop
- runs a primary update graph at a fixed tick rate (`UpdateRate`), independent of rendering and
  on its own threads natively
- runs a render graph once per frame (`RedrawRequested`), and submits the resulting `wgpu` frame
- sends `winit` window and device events into the ECS as `WindowEvent` and `DeviceEvent`
- tracks `UpdateDeltaTime` and `RenderDeltaTime`, and hands out entity ids through `spawn`
- lets functionality be packaged and shared as `Plugin`s
- optionally sets up an `egui` integration for debugging and simple in-app UI

## Features

| Feature | Enables |
|---|---|
| `auto-egui` | Automatically adds `EguiPlugin` when `App::new()` is called. |
| `dbg-schedules` | Implies `auto-egui`. Adds an egui window showing the update and render FPS. |

## Usage

### Minimal app (no rendering UI)

```rust
use anarchy::{ResMut, Resource};
use cell::App;

#[derive(Debug, Resource)]
struct Counter(u32);

fn main() -> anyhow::Result<()> {
    App::new()
        .add_resource(Counter(0))
        .on_update(tick)
        .run()
}

fn tick(mut counter: ResMut<Counter>) {
    counter.0 += 1;
    println!("tick {}", counter.0);
}
```

### App with an egui debug window

```rust
use anarchy::{Res, ResMut, Resource};
use cell::{App, EguiCtx, EguiPlugin};

#[derive(Debug, Resource)]
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

fn on_render_update(
    egui: Res<EguiCtx>,
    mut test_data: ResMut<TestData>
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

Systems are plain functions whose parameters are `anarchy` system params (`World`, `Query`,
`Res`, `ResMut`, `Event`, ...). Use the `_with` variants (`on_update_with`,
`on_render_update_with`, ...) to order them with `before`, `after` or `pin`. The egui pass is
open for every render system in the default `Normal` pin.

Plugins are deduplicated by type: calling `add_plugin` twice with the same plugin type only
builds it once.
