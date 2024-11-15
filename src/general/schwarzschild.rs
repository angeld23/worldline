#[derive(Debug, Clone, Copy)]
pub struct BlackHole {
    pub mass: f64,
}

impl Default for BlackHole {
    fn default() -> Self {
        Self { mass: 1.0 }
    }
}
