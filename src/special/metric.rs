use cgmath::{
    num_traits::{identities::One, Float},
    vec4, Matrix2, Matrix3, Matrix4, SquareMatrix,
};

/// A bilinear form that generalizes the dot/inner product of Euclidean space. Like the dot product, it
/// is used to define distances and angles.
///
/// A less overly-technical way to describe it is that the component with indices *ij* is equal to the
/// dot product of the *i* and *j* basis vectors. In an orthonormal Euclidean basis, the metric tensor is
/// simply the identity matrix.
///
/// # Note
///
/// A metric tensor's components **must be symmetric**, otherwise it stops making sense and you'll get some nasty
/// unexpected behavior.
pub trait MetricTensor: SquareMatrix
where
    Self::Scalar: Float,
{
    /// The metric for flat Minkowski spacetime.
    fn minkowski() -> Matrix4<f64> {
        Matrix4::from_diagonal(vec4(-1.0, -1.0, -1.0, 1.0))
    }

    /// Applies the metric on 2 vectors. This is basically just the dot product, AKA *|v||u|cos(θ)*.
    fn dot(self, v: Self::ColumnRow, u: Self::ColumnRow) -> Self::Scalar;

    /// Measures the squared length of a vector.
    fn length2(self, v: Self::ColumnRow) -> Self::Scalar {
        self.dot(v, v)
    }

    /// Measures the length of a vector.
    fn length(self, v: Self::ColumnRow) -> Self::Scalar {
        Float::sqrt(self.length2(v))
    }

    /// Creates a vector pointing in the same direction with a given length as measured by this metric.
    fn normalize_to(self, v: Self::ColumnRow, length: Self::Scalar) -> Self::ColumnRow {
        v * length / self.length(v)
    }

    /// Creates a vector pointing in the same direction with a length of 1 as measured by this metric.
    fn normalize(self, v: Self::ColumnRow) -> Self::ColumnRow {
        self.normalize_to(v, Self::Scalar::one())
    }
}

macro_rules! metric_tensor_impl {
    ($matrix:ty, $size:literal) => {
        impl MetricTensor for $matrix {
            fn dot(self, v: Self::ColumnRow, u: Self::ColumnRow) -> Self::Scalar {
                let v_components: [Self::Scalar; $size] = v.into();
                let u_components: [Self::Scalar; $size] = u.into();
                let metric_components: [[Self::Scalar; $size]; $size] = self.into();

                let mut total = 0.0;
                for (i, v_i) in v_components.into_iter().enumerate() {
                    for (j, u_j) in u_components.into_iter().enumerate() {
                        total += v_i * u_j * metric_components[i][j];
                    }
                }
                total
            }
        }
    };
}

metric_tensor_impl!(Matrix2<f64>, 2);
metric_tensor_impl!(Matrix3<f64>, 3);
metric_tensor_impl!(Matrix4<f64>, 4);
