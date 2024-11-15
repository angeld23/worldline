use std::ops::{Add, AddAssign, Div, Mul};

pub fn runge_kutta_step<T>(
    initial_value: T,
    initial_time: f64,
    time_step: f64,
    mut derivative: impl FnMut(f64, T) -> T,
) -> T
where
    T: Copy + Add<Output = T> + AddAssign + Mul<f64, Output = T> + Div<f64, Output = T>,
{
    let k_1 = derivative(initial_time, initial_value);
    let k_2 = derivative(
        initial_time + time_step / 2.0,
        initial_value + k_1 * time_step / 2.0,
    );
    let k_3 = derivative(
        initial_time + time_step / 2.0,
        initial_value + k_2 * time_step / 2.0,
    );
    let k_4 = derivative(initial_time + time_step, initial_value + k_3 * time_step);

    initial_value + (k_1 + k_2 * 2.0 + k_3 * 2.0 + k_4) * (time_step / 6.0)
}

pub fn runge_kutta_evaluate<T>(
    time: f64,
    initial_value: T,
    initial_time: f64,
    mut step_size: f64,
    mut derivative: impl FnMut(f64, T) -> T,
) -> T
where
    T: Copy + Add<Output = T> + AddAssign + Mul<f64, Output = T> + Div<f64, Output = T>,
{
    let time = time.max(initial_time);

    let mut current_value = initial_value;
    let mut current_time = initial_time;

    let step_count = ((time - initial_time) / step_size) as u32 + 1;

    for i in 0..step_count {
        if i == step_count - 1 {
            step_size = (time - initial_time).rem_euclid(step_size);
        }

        current_value = runge_kutta_step(current_value, current_time, step_size, &mut derivative);
        current_time += step_size;
    }

    current_value
}
