use super::f32_util::AddWithEpsilon;

pub type Point<const D: usize> = [f32; D];

/// Arbitrary-dimensional bounding box.
///
/// But good luck finding a practical use for anything other than two or three dimensions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox<const D: usize> {
    min: Point<D>,
    max: Point<D>,
}

/// Alias for [BoundingBox].
pub type BBox<const D: usize> = BoundingBox<D>;
/// Alias for a [one-dimensional BoundingBox](BoundingBox<1>). (aka basically a bounding line segment)
pub type BBox1 = BoundingBox<1>;
/// Alias for a [two-dimensional BoundingBox](BoundingBox<2>).
pub type BBox2 = BoundingBox<2>;
/// Alias for a [three-dimensional BoundingBox](BoundingBox<3>).
pub type BBox3 = BoundingBox<3>;
/// Alias for a [four-dimensional BoundingBox](BoundingBox<4>).
///
/// Seriously though, why would you need this?
pub type BBox4 = BoundingBox<4>;

impl<const D: usize> std::fmt::Display for BoundingBox<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BoundingBox(\n\t{:?}\n\t{:?}\n)", self.min, self.max)
    }
}

impl<const D: usize> Default for BoundingBox<D> {
    /// Create a bounding box with both corners initialized to zero.
    fn default() -> Self {
        Self {
            min: [0.0; D],
            max: [0.0; D],
        }
    }
}

impl BoundingBox<1> {
    /// Create a one-dimensional bounding ~box~ line segment with both ~corners~ ends initialized to zero.
    pub fn d1(&self) -> Self {
        Self::default()
    }

    pub fn length(&self) -> f32 {
        self.max[0] - self.min[0]
    }
}

impl BoundingBox<2> {
    /// Create a two-dimensional bounding box with both corners initialized to zero.
    pub fn d2() -> Self {
        Self::default()
    }

    /// Width times height.
    pub fn area(&self) -> f32 {
        self.measure()
    }
}

impl BoundingBox<3> {
    /// Create a three-dimensional bounding box with both corners initialized to zero.
    pub fn d3() -> Self {
        Self::default()
    }

    /// Width times height times length.
    pub fn volume(&self) -> f32 {
        self.measure()
    }
}

impl BoundingBox<4> {
    /// Create a four-dimensional bounding box with both corners initialized to zero.
    /// Why would you need this?
    pub fn d4() -> Self {
        Self::default()
    }

    /// Width times height times length times a secret, scarier fourth thing
    pub fn hypervolume(&self) -> f32 {
        self.measure()
    }
}

impl<const D: usize> BoundingBox<D> {
    /// Create the smallest bounding box that contains all provided points.
    pub fn new(positions: impl IntoIterator<Item = impl Into<Point<D>>>) -> Self {
        let mut bounding_box = Self::default();
        bounding_box.only_fit(positions);
        bounding_box
    }

    /// Changes the bounding box to the smallest size that contains all provided points,
    /// ignoring any previous bounds.
    pub fn only_fit(&mut self, positions: impl IntoIterator<Item = impl Into<Point<D>>>) {
        let mut positions = positions.into_iter();
        let first_pos: Point<D> = match positions.next() {
            Some(first_pos) => first_pos.into(),
            None => [0.0; D],
        };
        self.min = first_pos;
        self.max = first_pos;

        for position in positions {
            self.expand_to_fit(position);
        }
    }

    /// Checks whether a point is within the bounding box.
    pub fn point_is_within(&self, position: impl Into<Point<D>>) -> bool {
        let position: Point<D> = position.into();

        for (index, value) in position.into_iter().enumerate() {
            if value < self.min[index] || value > self.max[index] {
                return false;
            }
        }

        true
    }

    /// Check whether another bounding box fits entirely within this one.
    pub fn box_is_within(&self, other_box: Self) -> bool {
        self.point_is_within(other_box.min) && self.point_is_within(other_box.max)
    }

    /// Expands the bounding box to the smallest size that contains both its previous bounds
    /// and a newly provided point.
    ///
    /// Returns whether the box changed size.
    pub fn expand_to_fit(&mut self, position: impl Into<Point<D>>) -> bool {
        let position: Point<D> = position.into();
        let is_outside = !self.point_is_within(position);
        for (index, value) in position.into_iter().enumerate() {
            self.min[index] = value.min(self.min[index]);
            self.max[index] = value.max(self.max[index]);
        }
        is_outside
    }

    /// Expands the bounding box to the smallest size that contains both its previous bounds
    /// and the bounds of a newly provided bounding box.
    ///
    /// Returns whether the box changed size.
    pub fn expand_to_fit_box(&mut self, other_box: Self) -> bool {
        let min_expanded = self.expand_to_fit(other_box.min);
        self.expand_to_fit(other_box.max) || min_expanded
    }

    /// Applies [`BoundingBox::expand_to_fit()`] on all points in an iterator.
    pub fn expand_to_fit_iter(
        &mut self,
        positions: impl Iterator<Item = impl Into<Point<D>>>,
    ) -> bool {
        let mut expanded = false;
        for position in positions {
            if self.expand_to_fit(position) {
                expanded = true;
            };
        }
        expanded
    }

    /// Applies `expand_to_fit_box()` on all bounding boxes in an iterator.
    pub fn expand_to_fit_box_iter(&mut self, other_boxes: impl IntoIterator<Item = Self>) -> bool {
        let mut expanded = false;
        for other_box in other_boxes {
            if self.expand_to_fit_box(other_box) {
                expanded = true;
            }
        }
        expanded
    }

    /// The minimum corner of this bounding box's margins.
    pub const fn min(&self) -> Point<D> {
        self.min
    }

    /// The maximum corner of this bounding box's margins.
    pub const fn max(&self) -> Point<D> {
        self.max
    }

    /// The center point of this bounding box.
    pub fn center(&self) -> Point<D> {
        std::array::from_fn(|index| (self.min[index] + self.max[index]) / 2.0)
    }

    /// Retrieves the position of a specific corner of the box.
    ///
    /// The corner is chosen by specifying whether to get the maximum or minimum position for each axis.
    /// Use `true` for the maximum, and `false` for the minimum.
    ///
    /// # Example
    /// ```
    /// let cube = BoundingBox::new([[-3.0, -3.0, -3.0], [1.0, 1.0, 1.0]].into_iter());
    /// // retrieves the (+X, -Y, +Z) corner
    /// assert_eq!(cube.get_corner([true, false, true]), [1.0, -3.0, 1.0])
    /// ```
    pub fn get_corner(&self, is_max: [bool; D]) -> Point<D> {
        let mut i = 0;
        is_max.map(|is_max| {
            let value = if is_max { self.max[i] } else { self.min[i] };
            i += 1;
            value
        })
    }

    /// The size of this bounding box.
    pub fn size(&self) -> Point<D> {
        let mut i = 0;
        self.max.map(|max| {
            let value = max - self.min[i];
            i += 1;
            value
        })
    }

    /// The product of all components in this bounding box's size.
    ///
    /// This is the dimension-independant method for what is usually called *"area"* or *"volume"*.
    pub fn measure(&self) -> f32 {
        self.size()
            .into_iter()
            .fold(1.0, |product, value| product * value)
    }

    pub fn offset(&self, offset: impl Into<Point<D>>) -> Self {
        let offset = offset.into();

        let mut new_min = self.min;
        let mut new_max = self.max;
        for i in 0..D {
            new_min[i] += offset[i];
            new_max[i] += offset[i];
        }
        Self {
            min: new_min,
            max: new_max,
        }
    }

    pub fn offset_with_epsilon(&self, offset: impl Into<Point<D>>) -> Self {
        let offset = offset.into();

        let mut new_min = self.min;
        let mut new_max = self.max;
        for i in 0..D {
            new_min[i] = self.min[i].add_with_epsilon(offset[i]);
            new_max[i] = self.max[i].add_with_epsilon(offset[i]);
        }
        Self {
            min: new_min,
            max: new_max,
        }
    }

    pub fn project(&self, axis: usize) -> BoundingBox<{ D - 1 }> {
        let axis = axis.min(D - 1);

        let mut new_min = [0.0; D - 1];
        let mut new_max = [0.0; D - 1];

        let mut new_index = 0;
        for index in 0..D {
            if index != axis {
                new_min[new_index] = self.min[index];
                new_max[new_index] = self.max[index];
                new_index += 1;
            }
        }

        BoundingBox {
            min: new_min,
            max: new_max,
        }
    }

    pub fn intersection(&self, other: Self) -> Option<Self> {
        let mut new_min = [0.0; D];
        let mut new_max = [0.0; D];

        for index in 0..D {
            let min = self.min[index].max(other.min[index]);
            let max = self.max[index].min(other.max[index]);

            if max <= min {
                return None;
            } else {
                new_min[index] = min;
                new_max[index] = max;
            }
        }

        Some(Self {
            min: new_min,
            max: new_max,
        })
    }

    pub fn extend(&self, amount: impl Into<Point<D>>) -> Self {
        let amount = amount.into();

        let offset = self.offset(amount);
        Self::new([self.min, self.max, offset.min, offset.max])
    }

    pub fn extend_with_epsilon(&self, amount: impl Into<Point<D>>) -> Self {
        let amount = amount.into();

        let offset = self.offset_with_epsilon(amount);
        Self::new([self.min, self.max, offset.min, offset.max])
    }

    pub fn retract(&self, amount: impl Into<Point<D>>) -> Option<Self> {
        let amount = amount.into();

        let offset = self.offset(amount);
        self.intersection(offset)
    }

    pub fn retract_with_epsilon(&self, amount: impl Into<Point<D>>) -> Option<Self> {
        let amount = amount.into();

        let offset = self.offset_with_epsilon(amount);
        self.intersection(offset)
    }

    pub fn point_from_normalized(&self, normalized_point: impl Into<Point<D>>) -> Point<D> {
        let normalized_point = normalized_point.into();

        let size = self.size();
        let mut point = [0.0; D];
        for i in 0..D {
            point[i] = self.min[i] + normalized_point[i] * size[i];
        }

        point
    }

    pub fn point_to_normalized(&self, point: impl Into<Point<D>>) -> Point<D> {
        let point = point.into();

        let size = self.size();
        let mut normalized_point = [0.0; D];
        for i in 0..D {
            normalized_point[i] = (point[i] - self.min[i]) / size[i];
        }

        normalized_point
    }
}

macro_rules! bbox {
    ($($position:expr),*) => {
        crate::shared::bounding_box::BoundingBox::new([$($position,)*].into_iter())
    };
}

pub(crate) use bbox;
