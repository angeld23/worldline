use cgmath::{InnerSpace, Matrix, Matrix3, Matrix4, SquareMatrix, Vector3, Vector4, Zero};

/// Calculates the Lorentz/gamma (time dilation/length contraction) factor for a given 3-velocity.
///
/// The Lorentz factor for a 4-velocity is stored in the time (`w`) component.
pub fn lorentz_factor(velocity: Vector3<f64>) -> f64 {
    1.0 / (1.0 - velocity.magnitude2()).sqrt()
}

/// Calculates a transformation matrix to boost into the reference frame of a given 3-velocity.
///
/// A spacetime vector in a stationary basis will be transformed into the same vector in the moving frame's basis.
///
/// To get the inverse of the boost, simply negate the 3-velocity.
pub fn lorentz_boost(velocity: Vector3<f64>) -> Matrix4<f64> {
    let gamma = lorentz_factor(velocity);
    let speed2 = velocity.magnitude2();

    if speed2.is_zero() || speed2.next_down().is_zero() {
        return Matrix4::identity();
    }

    let velocity_matrix = Matrix3::from_cols(velocity, Vector3::zero(), Vector3::zero());
    let space_matrix = Matrix3::identity()
        + (gamma - 1.0) * velocity_matrix * velocity_matrix.transpose() / speed2;
    Matrix4::from_cols(
        space_matrix.x.extend(-gamma * velocity.x),
        space_matrix.y.extend(-gamma * velocity.y),
        space_matrix.z.extend(-gamma * velocity.z),
        velocity.extend(-1.0) * -gamma,
    )
}

/// Converts a 3-velocity into its corresponding 4-velocity.
pub fn velocity_3_to_4(velocity: Vector3<f64>) -> Vector4<f64> {
    let gamma = lorentz_factor(velocity);
    velocity.extend(1.0) * gamma
}

/// Converts a 4-velocity into its corresponding 3-velocity.
pub fn velocity_4_to_3(velocity: Vector4<f64>) -> Vector3<f64> {
    velocity.truncate() / velocity.w
}

/// Applies a 4-dimensional transformation matrix on a 3-velocity.
///
/// Shorthand for
/// ```
/// velocity_4_to_3(transform * velocity_3_to_4(velocity))
/// ```
pub fn transform_3_velocity(transform: Matrix4<f64>, velocity: Vector3<f64>) -> Vector3<f64> {
    velocity_4_to_3(transform * velocity_3_to_4(velocity))
}

/// Performs relativistic 3-velocity addition, which never results in a speed faster than light.
pub fn add_velocities(velocity_gun: Vector3<f64>, velocity_bullet: Vector3<f64>) -> Vector3<f64> {
    transform_3_velocity(lorentz_boost(-velocity_gun), velocity_bullet)
}

pub fn const_accel_proper_time(proper_accel: f64, rest_time: f64) -> f64 {
    ((1.0 + (proper_accel * rest_time).powi(2)).sqrt() + proper_accel * rest_time).ln()
        / proper_accel
}

pub fn const_accel_displacement(proper_accel: f64, rest_time: f64) -> f64 {
    ((1.0 + (proper_accel * rest_time).powi(2)).sqrt() - 1.0) / proper_accel
}

/// Converts a 3-velocity to its corresponding proper velocity (displacement per moving-clock-second).
pub fn velocity_3_to_proper(velocity: Vector3<f64>) -> Vector3<f64> {
    velocity * lorentz_factor(velocity)
}

/// Converts a proper velocity (displacement per moving-clock-second) to its corresponding 3-velocity.
pub fn velocity_proper_to_3(proper_velocity: Vector3<f64>) -> Vector3<f64> {
    proper_velocity.normalize_to(1.0 / (1.0 + 1.0 / proper_velocity.magnitude2()).sqrt())
}

/// Converts a 4-velocity to its corresponding proper velocity (displacement per moving-clock-second).
pub fn velocity_4_to_proper(velocity: Vector4<f64>) -> Vector3<f64> {
    velocity_3_to_proper(velocity_4_to_3(velocity))
}

/// Converts a proper velocity (displacement per moving-clock-second) to its corresponding 4-velocity.
pub fn velocity_proper_to_4(proper_velocity: Vector3<f64>) -> Vector4<f64> {
    velocity_3_to_4(velocity_proper_to_3(proper_velocity))
}
