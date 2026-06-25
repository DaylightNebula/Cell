use std::ops::{Deref, DerefMut};

use anarchy::macros::Resource;
use derive_more::{Deref, DerefMut};
use magician_vgpu::{RenderFrame, VirtualGpu};

#[derive(Resource, Deref, DerefMut)]
pub struct Graphics(pub(crate) VirtualGpu);


#[derive(Resource, Default)]
pub struct Frame(Option<RenderFrame>);

impl Frame {
    pub fn store(&mut self, frame: RenderFrame) {
        self.0 = Some(frame);
    }

    pub fn take(&mut self) -> Option<RenderFrame> {
        self.0.take()
    }
}

impl Deref for Frame {
    type Target = RenderFrame;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("Empty frame")
    }
}

impl DerefMut for Frame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("Empty frame")
    }
}
