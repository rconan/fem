//! This module is used to convert a continuous second order differential equation into a discretized state space model
//!
//! December 9, 2021
//!

use nalgebra::{Matrix3, RowVector3, Vector3};
use num_complex::Complex;
use serde::Serialize;
use std::fmt;

const Z_CPLX: Complex<f64> = Complex { re: 0., im: 0. };

/// This structure is used to convert a continuous 2nd order ODE into a discrete state space model
#[derive(Debug, Serialize, Clone, Default)]
pub struct ExponentialMatrix {
    /// Sampling time is second
    pub tau: f64,
    phi: (f64, f64, f64, f64),
    gamma: (f64, f64),
    b: Vec<f64>,
    c: Vec<f64>,
    /// State space model output vector
    pub y: Vec<f64>,
    x: (f64, f64),
}
impl ExponentialMatrix {
    pub fn n_inputs(&self) -> usize {
        self.b.len()
    }
    pub fn n_outputs(&self) -> usize {
        self.c.len()
    }
}
impl super::Solver for ExponentialMatrix {
    /// Creates a discrete state space model from a 2nd order ODE
    ///
    /// Creates a new structure from the sampling time $`\tau`$, the eigen frequency $`\omega`$ in radians, the damping coefficient $`\zeta`$ and the vectors $`b`$ and $`c`$ that converts a input vector to a modal coefficient and a model coefficient to an output vector, respectively
    fn from_second_order(
        tau: f64,
        omega: f64,
        zeta: f64,
        continuous_bb: Vec<f64>,
        continuous_cc: Vec<f64>,
    ) -> Self {
        /*

        */

        let exp_3by3m = if omega != 0. {
            // Complex pole of the 2nd order model
            let lambda_cplx = Complex {
                re: -omega * zeta,
                im: omega * (1. - (zeta * zeta)).sqrt(),
            };

            // Some of the diagonalization matrix elements
            let v11 = Complex {
                re: 1. / ((1. + omega.powi(4)).sqrt()),
                im: 0.,
            };
            let v31 = (omega * omega) * v11;
            let v12 = 1. / (1. + lambda_cplx.norm_sqr()).sqrt();
            let v13 = 1. / (1. + lambda_cplx.conj().norm_sqr()).sqrt();

            // Matrix of eigenvectors
            let v = Matrix3::from_columns(&[
                Vector3::new(v11, Z_CPLX, v31),
                Vector3::new(Complex { re: v12, im: 0. }, lambda_cplx * v12, Z_CPLX),
                Vector3::new(
                    Complex { re: v13, im: 0. },
                    lambda_cplx.conj() * v13,
                    Z_CPLX,
                ),
            ]);

            let k_row2 = (v12 * (lambda_cplx.conj() - lambda_cplx)).inv();
            let k_row3 = (v13 * (lambda_cplx.conj() - lambda_cplx)).inv();
            let inv_v = Matrix3::from_rows(&[
                RowVector3::new(Z_CPLX, Z_CPLX, v31.inv()),
                RowVector3::new(
                    lambda_cplx.conj() * k_row2,
                    Complex { re: -1., im: 0. } * k_row2,
                    -lambda_cplx.conj().unscale(omega * omega) * k_row2,
                ),
                RowVector3::new(
                    -lambda_cplx * k_row3,
                    Complex { re: 1., im: 0. } * k_row3,
                    lambda_cplx.unscale(omega * omega) * k_row3,
                ),
            ]);

            let diag_exp = Matrix3::from_columns(&[
                Vector3::new(Complex { re: 1., im: 0. }, Z_CPLX, Z_CPLX),
                Vector3::new(Z_CPLX, lambda_cplx.scale(tau).exp(), Z_CPLX),
                Vector3::new(Z_CPLX, Z_CPLX, lambda_cplx.conj().scale(tau).exp()),
            ]);

            v * diag_exp * inv_v
        } else {
            Matrix3::from_columns(&[
                Vector3::new(Complex { re: 1., im: 0. }, Z_CPLX, Z_CPLX),
                Vector3::new(
                    Complex { re: tau, im: 0. },
                    Complex { re: 1., im: 0. },
                    Z_CPLX,
                ),
                Vector3::new(
                    Complex {
                        re: 0.5 * tau * tau,
                        im: 0.,
                    },
                    Complex { re: tau, im: 0. },
                    Complex { re: 1., im: 0. },
                ),
            ])
        };

        let n = continuous_cc.len();
        Self {
            tau,
            phi: (
                exp_3by3m[0].re,
                exp_3by3m[3].re,
                exp_3by3m[1].re,
                exp_3by3m[4].re,
            ),
            gamma: (exp_3by3m[6].re, exp_3by3m[7].re),
            b: continuous_bb,
            c: continuous_cc,
            y: vec![0.; n],
            x: (0f64, 0f64),
        }
    }
    /// Returns the state space model output
    fn solve(&mut self, u: &[f64]) -> &[f64] {

        /* Implementation based on the standard state-space model realization:
        let (x0, x1) = self.x;
        self.y.iter_mut().zip(self.c.iter()).for_each(|(y, c)| {
            *y = c * x0;
        });
        let v = self.b.iter().zip(u).fold(0., |s, (b, u)| s + b * u);
        self.x.0 = self.phi.0 * x0 + self.phi.1 * x1 + self.gamma.0 * v;
        self.x.1 = self.phi.2 * x0 + self.phi.3 * x1 + self.gamma.1 * v;
        self.y.as_slice()
        */

        // Alternative realization to cope with extra delay due to the bootstrap process
        // State update
        let (x0, x1): (f64, f64) = self.x;
        let v = self.b.iter().zip(u).fold(0., |s, (b, u)| s + b * u);
        self.x.0 = self.phi.0 * x0 + self.phi.1 * x1 + self.gamma.0 * v;
        self.x.1 = self.phi.2 * x0 + self.phi.3 * x1 + self.gamma.1 * v;
        // Output update
        self.y.iter_mut().zip(self.c.iter()).for_each(|(y, c)| {
            *y = c * self.x.0;
        });
        
        self.y.as_slice()
    }

}
impl fmt::Display for ExponentialMatrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "2x2 discrete state space model: {}->{} ({:.3}Hz)\n - A: {:.9?}\n - B: {:.9?}",
            self.b.len(),
            self.c.len(),
            self.tau.recip(),
            self.phi,
            self.gamma
        )
    }
}
