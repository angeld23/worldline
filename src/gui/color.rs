use derive_more::*;

#[derive(Debug, Clone, Copy, From, Into, Add, Sub, Mul, Div, PartialEq)]
pub struct GuiColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for GuiColor {
    fn default() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

impl From<(f32, f32, f32)> for GuiColor {
    fn from(value: (f32, f32, f32)) -> Self {
        Self {
            r: value.0,
            g: value.1,
            b: value.1,
            a: 1.0,
        }
    }
}

impl From<GuiColor> for [f32; 4] {
    fn from(value: GuiColor) -> Self {
        [value.r, value.g, value.b, value.a]
    }
}

impl GuiColor {
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const DARK_BLUE: Self = Self::rgb(0.0, 0.0, 0.666);
    pub const DARK_GREEN: Self = Self::rgb(0.0, 0.666, 0.0);
    pub const DARK_AQUA: Self = Self::rgb(0.0, 0.666, 0.666);
    pub const DARK_RED: Self = Self::rgb(0.666, 0.0, 0.0);
    pub const DARK_PURPLE: Self = Self::rgb(0.666, 0.0, 0.666);
    pub const GOLD: Self = Self::rgb(1.0, 0.666, 0.0);
    pub const GRAY: Self = Self::rgb(0.666, 0.666, 0.666);
    pub const DARK_GRAY: Self = Self::rgb(0.333, 0.333, 0.333);
    pub const BLUE: Self = Self::rgb(0.333, 0.333, 1.0);
    pub const GREEN: Self = Self::rgb(0.333, 1.0, 0.333);
    pub const AQUA: Self = Self::rgb(0.333, 1.0, 1.0);
    pub const RED: Self = Self::rgb(1.0, 0.333, 0.333);
    pub const LIGHT_PURPLE: Self = Self::rgb(1.0, 0.333, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.333);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);

    pub const INVISIBLE: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn with_red(mut self, r: f32) -> Self {
        self.r = r;
        self
    }

    pub const fn with_green(mut self, g: f32) -> Self {
        self.g = g;
        self
    }

    pub const fn with_blue(mut self, b: f32) -> Self {
        self.b = b;
        self
    }

    pub const fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }

    pub fn shadow(self) -> Self {
        self.mul_color(0.125)
    }

    pub fn is_visible(self) -> bool {
        self.a > (1.0 / 255.0) / 2.0
    }

    pub fn mul_color(self, scalar: f32) -> Self {
        Self {
            r: self.r * scalar,
            g: self.g * scalar,
            b: self.b * scalar,
            a: self.a,
        }
    }
}
