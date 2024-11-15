use super::{inertial_frame::InertialFrame, transform::lorentz_factor};
use cgmath::Vector3;
use derive_more::*;
use std::collections::VecDeque;

pub const PHYS_TIME_STEP: f64 = 1.0 / 240.0;
pub const EVENT_BAKE_INTERVAL: f64 = 1.0;
pub const MAX_SPEED: f64 = 0.99999999999;

/// A specific kind of worldline event, paired with information specific to that kind.
#[derive(Debug, Clone, Copy, Unwrap, IsVariant)]
pub enum WorldlineEventKind {
    /// Constant velocity.
    Inertial,
    /// Constant proper acceleration.
    Acceleration(Vector3<f64>),
}

/// A keyframe event on a worldline.
#[derive(Debug, Clone, Copy)]
pub struct WorldlineEvent {
    pub frame: InertialFrame,
    pub proper_time: f64,
    pub kind: WorldlineEventKind,
}

impl WorldlineEvent {
    pub fn get_event_at_time_offset(&self, coord_time_offset: f64, time_resolution: f64) -> Self {
        let (frame, proper_time) = match self.kind {
            WorldlineEventKind::Inertial => (
                self.frame.predict(coord_time_offset),
                self.proper_time + coord_time_offset / lorentz_factor(self.frame.velocity),
            ),
            WorldlineEventKind::Acceleration(proper_accel) => {
                // arbitrary path and proper time for an accelerating object with non-zero unaligned starting
                // velocity has no exact solution, so we've gotta do some numerical bullshit instead
                let mut frame = self.frame;
                let mut proper_time = self.proper_time;

                let step_count = (coord_time_offset / time_resolution) as u32 + 1;
                let mut step_size = time_resolution;

                for i in 0..step_count {
                    if i == step_count - 1 {
                        step_size = coord_time_offset.rem_euclid(step_size);
                    }
                    proper_time += frame.step(step_size, proper_accel);
                }

                (frame, proper_time)
            }
        };

        Self {
            frame,
            proper_time,
            kind: self.kind,
        }
    }
}

/// The path that an entity traces through spacetime. There is no notion of "now" on a worldline alone, it
/// simply represents a static path that can be modified.
#[derive(Debug, Clone)]
pub struct Worldline {
    events: VecDeque<WorldlineEvent>,
    pub time_resolution: f64,
}

impl Default for Worldline {
    fn default() -> Self {
        Self::new(InertialFrame::default())
    }
}

impl Worldline {
    pub fn new(start_frame: InertialFrame) -> Self {
        Self {
            events: [WorldlineEvent {
                frame: start_frame,
                proper_time: 0.0,
                kind: WorldlineEventKind::Inertial,
            }]
            .into(),
            time_resolution: PHYS_TIME_STEP,
        }
    }

    fn get_neighbor_event_indices(&self, coord_time: f64) -> (Option<usize>, Option<usize>) {
        if self.events.is_empty() {
            return (None, None);
        }

        if self.events[self.events.len() - 1].frame.position.w < coord_time {
            return (Some(self.events.len() - 1), None);
        }

        let after_index = self
            .events
            .partition_point(|event| event.frame.position.w < coord_time);

        (
            if after_index == 0 {
                None
            } else {
                Some(after_index - 1)
            },
            Some(after_index),
        )
    }

    pub fn get_event_at_time(&self, coord_time: f64) -> WorldlineEvent {
        let (index_before, index_after) = self.get_neighbor_event_indices(coord_time);

        match (index_before, index_after) {
            (None, None) => WorldlineEvent {
                frame: InertialFrame::default(),
                proper_time: 0.0,
                kind: WorldlineEventKind::Inertial,
            },
            (None, Some(index_after)) => {
                let fake_inertial = WorldlineEvent {
                    kind: WorldlineEventKind::Inertial,
                    ..self.events[index_after]
                };
                fake_inertial.get_event_at_time_offset(
                    coord_time - fake_inertial.frame.position.w,
                    self.time_resolution,
                )
            }
            (Some(index_before), _) => {
                let before = self.events[index_before];
                before.get_event_at_time_offset(
                    coord_time - before.frame.position.w,
                    self.time_resolution,
                )
            }
        }
    }

    pub fn insert_event(&mut self, coord_time: f64, kind: WorldlineEventKind) {
        self.bake_events(coord_time);
        let (_, index_after) = self.get_neighbor_event_indices(coord_time);

        if let Some(index_after) = index_after {
            self.events.drain(index_after..);
        }

        let mut event = self.get_event_at_time(coord_time);
        event.kind = kind;
        self.events.push_back(event);
    }

    pub fn bake_events(&mut self, coord_time: f64) {
        let (index_before, index_after) = self.get_neighbor_event_indices(coord_time);
        if index_after.is_some() {
            // time is inbetween already-defined events, nothing to bake
            return;
        }

        if let Some(index_before) = index_before {
            let event = self.events[index_before];
            if event.kind.is_inertial() {
                // no need to bake for linear motion
                return;
            }

            let multiplier = self.time_resolution / PHYS_TIME_STEP;
            let mut bake_coord_time = event.frame.position.w + EVENT_BAKE_INTERVAL * multiplier;
            while bake_coord_time < coord_time {
                self.insert_event(bake_coord_time, event.kind);
                bake_coord_time += EVENT_BAKE_INTERVAL * multiplier;
            }
        }
    }
}
