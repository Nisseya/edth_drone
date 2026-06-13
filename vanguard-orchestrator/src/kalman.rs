use nalgebra::{SMatrix, SVector};

type Vec4 = SVector<f64, 4>;
type Vec2 = SVector<f64, 2>;

type Mat4 = SMatrix<f64, 4, 4>;
type Mat2 = SMatrix<f64, 2, 2>;
type Mat2x4 = SMatrix<f64, 2, 4>;
type Mat4x2 = SMatrix<f64, 4, 2>;

#[derive(Clone)]
pub struct KalmanTrack {
    state: Vec4,
    covariance: Mat4,
}

impl KalmanTrack {
    pub fn new(x: f64, y: f64, vx: f64, vy: f64) -> Self {
        Self {
            state: Vec4::new(x, y, vx, vy),
            covariance: Mat4::identity() * 100.0,
        }
    }

    pub fn predict(&mut self, dt: f64) {
        let f = Mat4::new(
            1.0, 0.0, dt, 0.0, 0.0, 1.0, 0.0, dt, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        );

        let q = Mat4::identity() * 0.1;

        self.state = f * self.state;
        self.covariance = f * self.covariance * f.transpose() + q;
    }

    pub fn update(&mut self, x: f64, y: f64) {
        let z = Vec2::new(x, y);

        let h = Mat2x4::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);

        let r = Mat2::identity() * 5.0;

        let innovation = z - h * self.state;

        let s = h * self.covariance * h.transpose() + r;

        let Some(s_inv) = s.try_inverse() else {
            return;
        };

        let k: Mat4x2 = self.covariance * h.transpose() * s_inv;

        self.state += k * innovation;

        self.covariance = (Mat4::identity() - k * h) * self.covariance;
    }

    pub fn position(&self) -> (f64, f64) {
        (self.state[0], self.state[1])
    }

    pub fn velocity(&self) -> (f64, f64) {
        (self.state[2], self.state[3])
    }
}
