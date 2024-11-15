use super::{transform::*, worldline::MAX_SPEED};
use crate::shared::numerical_integration::runge_kutta_step;
use cgmath::{vec3, vec4, InnerSpace, Vector3, Vector4};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InertialFrame {
    pub position: Vector4<f64>,
    pub velocity: Vector3<f64>,
}

impl Default for InertialFrame {
    fn default() -> Self {
        Self {
            position: vec4(0.0, 0.0, 0.0, 0.0),
            velocity: vec3(0.0, 0.0, 0.0),
        }
    }
}

impl InertialFrame {
    pub fn relative_to(self, other: Self) -> Self {
        let transform = lorentz_boost(other.velocity);

        Self {
            position: transform * (self.position - other.position),
            velocity: transform_3_velocity(transform, self.velocity),
        }
    }

    pub fn predict(self, delta_time: f64) -> Self {
        Self {
            position: self.position + self.velocity.extend(1.0) * delta_time,
            ..self
        }
    }

    /// Simulates the movement of this frame with a given proper acceleration and a time step.
    ///
    /// Uses the fourth-degree [Runge-Kutta method](https://en.wikipedia.org/wiki/Runge%E2%80%93Kutta_methods),
    /// so smaller `delta_time` values are more precise.
    ///
    /// Returns the elapsed proper time during this time-step.
    pub fn step(&mut self, delta_time: f64, proper_accel: Vector3<f64>) -> f64 {
        let transform = lorentz_boost(-self.velocity);

        let old_velocity = self.velocity;

        let accel_4 = transform * proper_accel.extend(0.0);
        let velocity_derivative = |_, velocity: Vector3<f64>| {
            (1.0 - velocity.magnitude2()) * (accel_4.truncate() - velocity * accel_4.w)
        };

        self.velocity = runge_kutta_step(self.velocity, 0.0, delta_time, &velocity_derivative);
        if self.velocity.magnitude2() > MAX_SPEED * MAX_SPEED {
            self.velocity = self.velocity.normalize_to(MAX_SPEED);
        }

        self.position = runge_kutta_step(self.position.truncate(), 0.0, delta_time, |time, _| {
            runge_kutta_step(old_velocity, 0.0, time, &velocity_derivative)
        })
        .extend(self.position.w + delta_time);

        runge_kutta_step(0.0, 0.0, delta_time, |time, _| {
            1.0 / lorentz_factor(runge_kutta_step(
                old_velocity,
                0.0,
                time,
                &velocity_derivative,
            ))
        })
    }
}
