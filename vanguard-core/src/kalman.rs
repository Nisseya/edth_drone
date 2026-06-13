use nalgebra::{SMatrix, SVector};

type Vec4 = SVector<f64, 4>;
type Vec2 = SVector<f64, 2>;

type Mat4 = SMatrix<f64, 4, 4>;
type Mat2 = SMatrix<f64, 2, 2>;
type Mat2x4 = SMatrix<f64, 2, 4>;
type Mat4x2 = SMatrix<f64, 4, 2>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KalmanTrack {
    pub state: Vec4,
    pub covariance: Mat4,
}
