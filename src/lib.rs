use std::{any::TypeId, collections::LinkedList};

use anarchy::{Resource, ResourceMeta, Schedule, ScheduleID, ScheduleTile, Scheduler, System, World, macros::{Getters, GettersMut}};
use magician_vgpu::RenderFrame;
use mutual::DashSet;
use winit::event_loop::EventLoop;

pub mod egui;
pub mod gpu;
pub mod plugin;
pub mod utils;
pub(crate) mod wrapper;

pub use egui::*;
pub use gpu::*;
pub use plugin::*;
pub use utils::*;

pub type RenderScheduleIn = ();
pub type RenderScheduleOut = ();

#[derive(Getters, GettersMut)]
pub struct App {
    primary_schedule_id: ScheduleID,
    primary_schedule: Option<Schedule<(), ()>>,
    render_schedule_id: ScheduleID,
    render_schedule: Option<Schedule<RenderScheduleIn, RenderScheduleOut>>,
    added_plugins: DashSet<TypeId>,
    world: World
}

impl App {
    /// Create a new `App` instance.
    pub fn new() -> Self {
        App {
            primary_schedule_id: ScheduleID { id: "APP", tick_rate: 60, max_threads: 4 },
            primary_schedule: Some(Schedule::new_empty()),
            render_schedule_id: ScheduleID { id: "RENDER", tick_rate: 0, max_threads: 1 },
            render_schedule: Some(Schedule::new_empty()),
            added_plugins: DashSet::default(),
            world: World::new()
        }
    }

    /// Add a plugin to this `App`.  The plugins build function is called immeidately.
    /// If a plugin is added twice on accident, uniqueness is guaranteed, so the second
    /// and any future attempts will be ignored.
    pub fn add_plugin<P: Plugin + 'static>(self, plugin: P) -> Self {
        let type_id = TypeId::of::<P>();
        if self.added_plugins.contains(&type_id) { return self }
        self.added_plugins.insert(type_id);
        return plugin.build(self);
    }

    /// Add a resource to this apps world.
    pub fn add_resource<R: Resource + ResourceMeta + 'static>(self, resource: R) -> Self {
        self.world.insert_resource(resource);
        return self;
    }

    /// Add a system to run on startup of the primary schedule.
    pub fn on_startup<S>(mut self, system: S) -> Self
        where S: System<(), Result<(), Box<dyn std::error::Error>>> + 'static
    {
        let tile = ScheduleTile::new(vec![Box::new(system)]);
        self.primary_schedule.as_mut().unwrap().add_startup(tile);
        return self;
    }

    /// Add a system to run on update of the primary schedule.
    pub fn on_update<S>(mut self, system: S) -> Self
        where S: System<(), Result<(), Box<dyn std::error::Error>>> + 'static
    {
        let tile = ScheduleTile::new(vec![Box::new(system)]);
        self.primary_schedule.as_mut().unwrap().add_new(tile);
        return self;
    }

    /// Add a system to run on update of the render schedule.
    pub fn on_render_startup<S>(mut self, system: S) -> Self
        where S: System<RenderScheduleIn, Result<RenderScheduleOut, Box<dyn std::error::Error>>> + 'static
    {
        let tile = ScheduleTile::new(vec![Box::new(system)]);
        self.render_schedule.as_mut().unwrap().add_startup(tile);
        return self;
    }

    /// Add a system to run on update of the render schedule.
    pub fn on_render_update<S>(mut self, system: S) -> Self
        where S: System<RenderScheduleIn, Result<RenderScheduleOut, Box<dyn std::error::Error>>> + 'static
    {
        let tile = ScheduleTile::new(vec![Box::new(system)]);
        self.render_schedule.as_mut().unwrap().add_new(tile);
        return self;
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            env_logger::init();
        }
        #[cfg(target_arch = "wasm32")]
        {
            console_log::init_with_level(log::Level::Info).unwrap_throw();
        }

        // schedule primary schedule
        Scheduler::schedule(
            self.primary_schedule_id, 
            self.primary_schedule.take().expect("Primary schedule was taken early"), 
            self.world.clone()
        );

        let event_loop = EventLoop::with_user_event().build()?;
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::wrapper::AppWrapper;

            let mut wrapper = AppWrapper { app: self };
            event_loop.run_app(&mut wrapper)?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let app = App::new(&event_loop);
            event_loop.spawn_app(app);
        }

        Ok(())
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        let Some(mut vgpu) = self.world
            .get_resource_mut::<Graphics>() 
            else { return };
        if width <= 0 || height <= 0 { return }

        vgpu.config_mut().width = width;
        vgpu.config_mut().height = height;
        vgpu.surface().configure(vgpu.device(), vgpu.config());
     }

    pub(crate) fn render(&mut self) -> anyhow::Result<()> {
        let Some(vgpu) = self.world
            .get_resource_ref::<Graphics>() 
            else { return Ok(()) };
        
        {
            let Some(frame) = RenderFrame::begin(&vgpu)?
                else { return Ok(()) };
            self.world.get_resource_mut::<Frame>()
                .unwrap()
                .store(frame);
        }

        // setup tiles list
        let mut next_render_schedule = Schedule::new_empty();
        let prev_render_schedule = self.render_schedule
            .take()
            .expect("Render schedule has gone missing");
        let mut tiles = LinkedList::new();
        if prev_render_schedule.has_next_startup() {
            while let Some(tile) = prev_render_schedule.next_startup() {
                tiles.push_back(tile);
            }

            while let Some(item) = prev_render_schedule.next_update() {
                next_render_schedule.add_new(item.tile.clone());
            }
        } else {
            while let Some(tile) = prev_render_schedule.next_update() {
                tiles.push_back(tile);
            }
        }

        // execute previous tiles
        tiles.into_iter().for_each(|tile| {
            tile.tile.execute(&self.world, &(), self.render_schedule_id, tile.first_run);
            if !tile.dont_save { next_render_schedule.post_run_add(tile.tile.clone(), prev_render_schedule.total_runtime); }
        });
        self.render_schedule = Some(next_render_schedule);

        let frame = {
            self.world.get_resource_mut::<Frame>()
                .unwrap()
                .take()
                .expect("Frame lost")
        };

        // finalize frame and mark complete
        frame.submit(&vgpu);
        Ok(())
    }
}
