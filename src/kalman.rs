use std::usize;

use nalgebra::{Matrix, RealField, SMatrix, SVector};

use crate::{
    measurement::{LinearMeasurement, Measurement},
    system::{LinearSystemNoInput, System},
};

/// Representation of Kalman filter
#[derive(Debug)]
struct Kalman<T, const N: usize, const U: usize, S> {
    /// Covariance
    pub P: SMatrix<T, N, N>,
    /// The associated system
    pub system: S,
}

impl<T: RealField + Copy, const N: usize, const U: usize, S: System<T, N, U>> Kalman<T, N, U, S> {
    pub fn predict(&mut self) {
        self.system.step();
        let F = self.system.transition();
        self.P = F * self.P * F.transpose() + self.system.covariance();
    }

    pub fn predict_with_input(&mut self, u: SVector<T, U>) {
        self.system.step_with_input(u);
        let F = self.system.transition();
        self.P = F * self.P * F.transpose() + self.system.covariance();
    }

    pub fn new(system: S) -> Self {
        Kalman {
            P: SMatrix::zeros(),
            system,
        }
    }
}

trait KalmanUpdate<T, const N: usize, const M: usize, ME: Measurement<T, N, M>> {
    fn update(&mut self, measurement: &ME);
}

impl<
        T: RealField + Copy,
        const N: usize,
        const M: usize,
        const U: usize,
        S: System<T, N, U>,
        ME: Measurement<T, N, M>,
    > KalmanUpdate<T, N, M, ME> for Kalman<T, N, U, S>
{
    fn update(&mut self, measurement: &ME) {
        let H_transpose = measurement.observation().transpose();
        // innovation
        let y = measurement.innovation(self.system.state());
        // innovation covariance
        let S = measurement.observation() * self.P * H_transpose + measurement.covariance();
        // Optimal gain
        let K = self.P * H_transpose * S.try_inverse().unwrap();
        // state update
        *self.system.state_mut() += K * y;
        // covariance update
        self.P = (SMatrix::identity() - K * measurement.observation()) * self.P;
    }
}

/// Kalman filter with a fixed shape measurement
struct Kalman1M<
    T,
    const N: usize,
    const U: usize,
    const M: usize,
    S: System<T, N, U>,
    ME: Measurement<T, N, M>,
> {
    kalman: Kalman<T, N, U, S>,
    measurement: ME,
}

impl<T: RealField + Copy, const N: usize, const M: usize>
    Kalman1M<T, N, 0, M, LinearSystemNoInput<T, N>, LinearMeasurement<T, N, M>>
{
    pub fn new_linear_no_input(
        F: SMatrix<T, N, N>,
        Q: SMatrix<T, N, N>,
        H: SMatrix<T, M, N>,
        R: SMatrix<T, M, M>,
    ) -> Self {
        Kalman1M {
            kalman: Kalman::new_linear_no_input(F, Q),
            measurement: LinearMeasurement {
                z: SMatrix::zeros(),
                H,
                R,
            },
        }
    }

    pub fn predict(&mut self) {
        self.kalman.predict();
    }

    pub fn update(&mut self, z: SVector<T, M>) {
        self.measurement.z = z;
        self.kalman.update(&self.measurement);
    }
}

type KalmanLinearNoInput<T, const N: usize> = Kalman<T, N, 0, LinearSystemNoInput<T, N>>;

impl<T: RealField + Copy, const N: usize> KalmanLinearNoInput<T, N> {
    pub fn new_linear_no_input(F: SMatrix<T, N, N>, Q: SMatrix<T, N, N>) -> Self {
        Kalman::new(LinearSystemNoInput::new(F, Q, SMatrix::zeros()))
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::{Matrix1, Matrix1x2, Matrix2, SMatrix, Vector1, Vector2};

    use rand_distr::{Distribution, Normal};

    use crate::measurement::Measurement;

    use super::{Kalman, KalmanLinearNoInput, KalmanUpdate};

    // #[test]
    // fn different_measurments() {
    //     let mut kf = KalmanLinearNoInput::new_linear_no_input(
    //         Matrix2::new(1.0, 0.1, 0.0, 1.0),
    //         SMatrix::identity(),
    //     );
    //     let m1 = Measurement {
    //         z: Vector1::new(5.0),
    //         H: Matrix1x2::new(1.0, 0.0),
    //         R: SMatrix::identity(),
    //     };
    //     let m2 = Measurement {
    //         z: Vector2::new(5.0, 1.4),
    //         H: Matrix2::identity(),
    //         R: SMatrix::identity(),
    //     };
    //     kf.predict();
    //     kf.update(&m1);
    //     kf.predict();
    //     kf.update(&m2);
    // }

    // #[test]
    // fn const_velocity() {
    //     // Example usage with position and velocity states and measured position
    //     // Model assumes constant velocity

    //     // Timestep
    //     let dt = 0.1;
    //     let F = Matrix2::new(1.0, 0.1, 0.0, 1.0);
    //     let Q = Matrix2::identity();
    //     let H = Matrix1x2::new(1.0, 0.0);
    //     let R = Matrix1::identity() * 0.5;
    //     let mut kalman = Kalman::new(F, Q, H, R);

    //     // Create a random number generator (using rand_distr crate)
    //     let position_error = Normal::new(0.0, 1.0).unwrap();
    //     let mut rng = rand::thread_rng();

    //     // set initial state and covariance
    //     kalman.x = Vector2::new(position_error.sample(&mut rng), 0.0);
    //     kalman.P = Q;

    //     // Record state evolution
    //     const T: usize = 40;
    //     const VELOCITY: f64 = 1.0;
    //     let positions = (0..T).map(|t| (t as f64) * VELOCITY).collect::<Vec<f64>>();
    //     let mut measured = Vec::with_capacity(T);
    //     let mut predicted = Vec::with_capacity(T);
    //     let mut updated = Vec::with_capacity(T);
    //     let mut covariance = Vec::with_capacity(T);

    //     // iterate though time in seconds
    //     for position in &positions {
    //         // The noise corrupted measurement
    //         let measurement = position + position_error.sample(&mut rng);
    //         measured.push(measurement);
    //         // kalman predict and update
    //         kalman.predict();
    //         predicted.push(kalman.x.x);
    //         kalman.update(&Matrix1::new(measurement));
    //         updated.push(kalman.x.x);
    //         covariance.push(kalman.P.m11);
    //     }

    //     let root = SVGBackend::new("plots/kf1.svg", (640, 480)).into_drawing_area();
    //     root.fill(&WHITE);
    //     let mut chart = ChartBuilder::on(&root)
    //         .caption("KF Position Estimate 1", ("sans-serif", 30).into_font())
    //         .margin(5)
    //         .x_label_area_size(30)
    //         .y_label_area_size(30)
    //         .build_cartesian_2d(0f32..(T as f32), 0f32..(T as f32))
    //         .unwrap();

    //     chart.configure_mesh().draw().unwrap();

    //     chart
    //         .draw_series(LineSeries::new(
    //             (0..T).zip(positions).map(|(t, x)| (t as f32, x as f32)),
    //             &BLACK,
    //         ))
    //         .unwrap()
    //         .label("Real")
    //         .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLACK));

    //     chart
    //         .draw_series(LineSeries::new(
    //             (0..T).zip(measured).map(|(t, x)| (t as f32, x as f32)),
    //             &RED,
    //         ))
    //         .unwrap()
    //         .label("Measured")
    //         .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    //     chart
    //         .draw_series(LineSeries::new(
    //             (0..T).zip(predicted).map(|(t, x)| (t as f32, x as f32)),
    //             &GREEN,
    //         ))
    //         .unwrap()
    //         .label("Predicted")
    //         .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &GREEN));

    //     chart
    //         .draw_series(LineSeries::new(
    //             (0..T).zip(updated).map(|(t, x)| (t as f32, x as f32)),
    //             &BLUE,
    //         ))
    //         .unwrap()
    //         .label("Updated")
    //         .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    //     chart
    //         .configure_series_labels()
    //         .background_style(&WHITE.mix(0.8))
    //         .border_style(&BLACK)
    //         .draw()
    //         .unwrap();

    //     root.present().unwrap();
    // }
}
