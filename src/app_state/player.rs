use crate::{
    graphics::camera::Camera,
    shared::input::InputController,
    special::{universe::Universe, worldline::WorldlineEventKind},
};
use cgmath::{vec3, Deg, InnerSpace, One, Quaternion, Rotation, Rotation3, Zero};
use winit::keyboard::NamedKey;

#[derive(Debug, Clone, Copy)]
pub struct PlayerController {
    pub camera: Camera,
    pub rotation: Quaternion<f64>,
    pub acceleration: f64,
}

impl Default for PlayerController {
    fn default() -> Self {
        Self {
            camera: Default::default(),
            rotation: Quaternion::one(),
            acceleration: 0.25,
        }
    }
}

impl PlayerController {
    pub const ANGLE_PER_PIXEL: Deg<f64> = Deg(0.1);
    pub const ROLL_PER_SECOND: Deg<f64> = Deg(45.0);

    pub fn update(&mut self, universe: &mut Universe, input: &mut InputController, delta: f64) {
        if input.pressed(NamedKey::Tab) {
            input.force_mouse_unlock = !input.force_mouse_unlock;
        }

        let acceleration = if input.is_movement_suppressed() {
            vec3(0.0, 0.0, 0.0)
        } else {
            let mut movement_vector = vec3(0.0, 0.0, 0.0);

            if input.held("w") {
                movement_vector.z -= 1.0;
            }
            if input.held("a") {
                movement_vector.x -= 1.0;
            }
            if input.held("s") {
                movement_vector.z += 1.0;
            }
            if input.held("d") {
                movement_vector.x += 1.0;
            }
            if input.held(NamedKey::Control) {
                movement_vector.y -= 1.0;
            }
            if input.held(NamedKey::Shift) {
                movement_vector.y += 1.0;
            }

            let mouse_delta = input.mouse_delta();
            let (yaw_delta, pitch_delta) = (-mouse_delta.x as f64, -mouse_delta.y as f64);

            let mut roll_delta = 0.0;
            if input.held("q") {
                roll_delta += 1.0;
            }
            if input.held("e") {
                roll_delta -= 1.0;
            }
            roll_delta *= delta;

            self.rotation = (self.rotation
                * Quaternion::from_angle_x(Self::ANGLE_PER_PIXEL * pitch_delta)
                * Quaternion::from_angle_y(Self::ANGLE_PER_PIXEL * yaw_delta)
                * Quaternion::from_angle_z(Self::ROLL_PER_SECOND * roll_delta))
            .normalize();

            if movement_vector.is_zero() {
                vec3(0.0, 0.0, 0.0)
            } else {
                self.rotation * (movement_vector.normalize() * self.acceleration)
            }
        };

        let user_event = universe.user_event_now();

        let update_acceleration =
            if let WorldlineEventKind::Acceleration(proper_accel) = user_event.kind {
                proper_accel != acceleration
            } else {
                !acceleration.is_zero()
            };

        if update_acceleration {
            let time = universe.time;
            universe
                .get_user_entity_mut()
                .worldline
                .insert_event(time, WorldlineEventKind::Acceleration(acceleration));
        }

        self.camera = Camera {
            rotation: self.rotation.cast().unwrap(),
            vertical_fov: Deg(90.0),
            ..Default::default()
        }
    }
}
