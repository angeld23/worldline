use cgmath::{vec2, ElementWise, Vector2};
use derive_more::*;

#[derive(Debug, Default, Clone, Copy, Add, Sub, Mul, Div, PartialEq)]
pub struct UDim {
    pub scale: f32,
    pub offset: f32,
}

impl From<f32> for UDim {
    fn from(value: f32) -> Self {
        Self::from_scale(value)
    }
}

impl From<(f32, f32)> for UDim {
    fn from(value: (f32, f32)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl UDim {
    pub fn new(scale: f32, offset: f32) -> Self {
        Self { scale, offset }
    }

    pub fn from_scale(scale: f32) -> Self {
        Self { scale, offset: 0.0 }
    }

    pub fn from_offset(offset: f32) -> Self {
        Self { scale: 0.0, offset }
    }

    pub fn lerp(self, other: Self, alpha: f32) -> Self {
        Self {
            scale: self.scale + (other.scale - self.scale) * alpha,
            offset: self.offset + (other.offset - self.offset) * alpha,
        }
    }

    pub fn absolute(self, frame: f32) -> f32 {
        self.scale * frame + self.offset
    }
}

#[derive(Debug, Default, Clone, Copy, Add, Sub, Mul, Div, PartialEq)]
pub struct UDim2 {
    pub x: UDim,
    pub y: UDim,
}

impl<T, U> From<(T, U)> for UDim2
where
    T: Into<UDim>,
    U: Into<UDim>,
{
    fn from(value: (T, U)) -> Self {
        Self {
            x: value.0.into(),
            y: value.1.into(),
        }
    }
}

impl<T> From<Vector2<T>> for UDim2
where
    T: Into<UDim>,
{
    fn from(value: Vector2<T>) -> Self {
        (value.x, value.y).into()
    }
}

impl UDim2 {
    pub fn new(x: impl Into<UDim>, y: impl Into<UDim>) -> Self {
        (x, y).into()
    }

    pub fn from_scale(x: f32, y: f32) -> Self {
        Self {
            x: UDim::from_scale(x),
            y: UDim::from_scale(y),
        }
    }

    pub fn from_offset(x: f32, y: f32) -> Self {
        Self {
            x: UDim::from_offset(x),
            y: UDim::from_offset(y),
        }
    }

    pub fn lerp(self, other: Self, alpha: f32) -> Self {
        Self {
            x: self.x.lerp(other.x, alpha),
            y: self.y.lerp(other.y, alpha),
        }
    }

    pub fn absolute(self, frame: Vector2<f32>) -> Vector2<f32> {
        vec2(self.x.absolute(frame.x), self.y.absolute(frame.y))
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScaleAxes {
    #[default]
    XY,
    XX,
    YY,
    YX,
}

impl ScaleAxes {
    pub fn effective_frame(self, frame: Vector2<f32>) -> Vector2<f32> {
        match self {
            Self::XY => frame.xy(),
            Self::XX => frame.xx(),
            Self::YY => frame.yy(),
            Self::YX => frame.yx(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuiTransform {
    pub position: UDim2,
    pub position_constraint: ScaleAxes,
    pub size: UDim2,
    pub size_constraint: ScaleAxes,
    pub anchor_point: Vector2<f32>,
}

impl Default for GuiTransform {
    fn default() -> Self {
        Self {
            position: Default::default(),
            position_constraint: Default::default(),
            size: Default::default(),
            size_constraint: Default::default(),
            anchor_point: vec2(0.0, 0.0),
        }
    }
}

impl GuiTransform {
    pub fn from_absolute(absolute_position: Vector2<f32>, absolute_size: Vector2<f32>) -> Self {
        Self {
            position: UDim2::from_offset(absolute_position.x, absolute_position.y),
            size: UDim2::from_offset(absolute_size.x, absolute_size.y),
            ..Default::default()
        }
    }

    pub fn absolute_position(self, frame: Vector2<f32>) -> Vector2<f32> {
        self.position
            .absolute(self.position_constraint.effective_frame(frame))
            - self
                .absolute_size(frame)
                .mul_element_wise(self.anchor_point)
    }

    pub fn absolute_size(self, frame: Vector2<f32>) -> Vector2<f32> {
        self.size
            .absolute(self.size_constraint.effective_frame(frame))
            .map(|v| v.abs())
    }

    /// (absolute_position, absolute_size)
    pub fn absolute(self, frame: Vector2<f32>) -> (Vector2<f32>, Vector2<f32>) {
        (self.absolute_position(frame), self.absolute_size(frame))
    }

    pub fn contained_in(
        self,
        container: Self,
        outer_frame: Vector2<f32>,
        outer_offset: Vector2<f32>,
    ) -> Self {
        let (container_position, container_size) = container.absolute(outer_frame);
        let (offset, absolute_size) = self.absolute(container_size);
        Self::from_absolute(container_position + offset + outer_offset, absolute_size)
    }
}
