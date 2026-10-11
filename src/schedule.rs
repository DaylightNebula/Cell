use anarchy::Resource;
use chrono::{DateTime, TimeDelta, Utc};
use derive_more::{Deref, DerefMut};

/// Resource holding how many times per second the update graph runs, 0 runs it as fast as
/// possible. Set with [`App::with_update_rate`](crate::App::with_update_rate), and may be changed
/// while running.
#[derive(Debug, Deref, DerefMut, Resource, Clone, Copy)]
pub struct UpdateRate(pub u32);

/// Resource holding the render frame cap in frames per second, 0 renders as fast as possible.
/// Set with [`App::with_render_rate`](crate::App::with_render_rate), and may be changed while
/// running. Ignored on web, where `requestAnimationFrame` paces frames.
#[derive(Debug, Deref, DerefMut, Resource, Clone, Copy)]
pub struct RenderRate(pub u32);

/// Resource holding the seconds between the starts of the last two update passes.
#[derive(Debug, Default, Deref, DerefMut, Resource, Clone, Copy)]
pub struct UpdateDeltaTime(pub f32);

/// Resource holding the seconds between the starts of the last two render frames, including any
/// time spent waiting on the render rate cap or vsync.
#[derive(Debug, Default, Deref, DerefMut, Resource, Clone, Copy)]
pub struct RenderDeltaTime(pub f32);

/// Paces a repeating pass to a rate and measures the time between passes. Uses `chrono` rather
/// than `std::time::Instant`, which panics on web.
#[derive(Default)]
pub(crate) struct Pacer {
    /// Earliest time the next pass may start, `None` until the first capped pass.
    next: Option<DateTime<Utc>>,
    /// Start time of the previous pass.
    last: Option<DateTime<Utc>>
}

impl Pacer {
    /// Returns the deadline to wait for if a pass at `rate` per second is too early, otherwise
    /// claims the current slot and returns `None`.
    pub(crate) fn too_early(&mut self, rate: u32) -> Option<DateTime<Utc>> {
        if rate == 0 { return None }

        let now = Utc::now();
        let period = TimeDelta::nanoseconds(1_000_000_000 / rate as i64);
        match self.next {
            Some(next) if now < next => return Some(next),
            // keep cadence from the last deadline so passes don't drift, but never bank a backlog
            Some(next) => self.next = Some((next + period).max(now)),
            None => self.next = Some(now + period),
        }
        None
    }

    /// Marks the start of a pass, returning the seconds since the previous one started.
    pub(crate) fn start_pass(&mut self) -> f32 {
        let now = Utc::now();
        let previous = self.last.replace(now).unwrap_or(now);
        now.signed_duration_since(previous).to_std().map(|a| a.as_secs_f32()).unwrap_or(0.0)
    }
}

/// Logs errors returned by a graph, skipping repeats of the previous one so a system that fails
/// every pass (such as one waiting on a resource that doesn't exist yet) doesn't flood the log.
#[derive(Default)]
pub(crate) struct ErrorLog {
    last: Option<String>
}

impl ErrorLog {
    /// Report the result of running `graph`.
    pub(crate) fn report(&mut self, graph: &str, result: anyhow::Result<()>) {
        match result {
            Ok(()) => self.last = None,
            Err(err) => {
                let message = format!("{err:#}");
                if self.last.as_ref() != Some(&message) {
                    log::error!("{graph} system failed: {message}");
                    self.last = Some(message);
                }
            }
        }
    }
}
