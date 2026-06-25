use anarchy::macros::Event;
use derive_more::{Deref, DerefMut};

#[derive(Event, Debug, Clone, Deref, DerefMut)]
pub struct WindowEvent(pub winit::event::WindowEvent);