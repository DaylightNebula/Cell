use anarchy::macros::Event;
use derive_more::{Deref, DerefMut};

/// ECS event wrapping a raw `winit::event::WindowEvent`. Broadcast into the `World` for every
/// window event received by the event loop, so systems can react to input, resizes, etc.
#[derive(Event, Debug, Clone, Deref, DerefMut)]
pub struct WindowEvent(pub winit::event::WindowEvent);