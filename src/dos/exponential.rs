//! This module is used to convert a continuous second order differential equation into a discretized state space model
//!
//! A continuous second order differential equation is given as $$\ddot q + 2\omega\zeta\dot q + \omega^2 q = \vec b\cdot \vec u$$
//! with the output $$\vec y=q\vec c$$
//! The ODE can be written as the state space model:
//! $$
//! \dot x = Ax + Bu
//! $$
//! $$
//! y = Cx
//! $$
//! where
//! ```math
//! x = \begin{bmatrix}
//! q \\
//! \dot q
//! \end{bmatrix},
//! A = \begin{bmatrix}
//! 0 & 1 \\
//! -\omega^2 & -2\omega\zeta
//! \end{bmatrix}
//! ,
//! B = \begin{bmatrix}
//! \vec 0 \\
//! \vec b
//! \end{bmatrix}
//! ,
//! C = \begin{bmatrix}
//! \vec c & \vec 0
//! \end{bmatrix}
//! ```
//! The continuous state space model is transformed into a discrete state space model
//! $$
//! x[k+1] = A_d x\[k\] + B_d u\[k\]
//! $$
//! $$
//! y\[k\] = C_d x\[k\]
//! $$
//! where
//! $$ A_d = \exp(A\tau),$$
//! $$ B_d = A^{-1}(A_d-I)B,$$
//! $$ C_d = C$$
//! and $`\tau`$ is the sample time.
//!
//! [$`A_d = \exp(A\tau)`$](https://www.wolframalpha.com/input/?i=Matrixexp%5B%7B%7B0%2Ct%7D%2C%7B-tx%5E2%2C-2txy%7D%7D%5D)=
//! ```math
//! A_d = \begin{bmatrix}
//! {\alpha_+\beta_- + \alpha_-\beta_+ \over 2z} & {\beta_- - \beta_+ \over 2z} \\
//! {x^2 (\beta_+ - \beta_-) \over 2z} & {\alpha_-\beta_- + \alpha_+\beta_+ \over 2z}
//! \end{bmatrix}
//! ```
//! [$`A^{-1}`$](https://www.wolframalpha.com/input/?i=inverse+%7B%7B0%2C+1%7D%2C+%7B-x%5E2%2C+-2yx%7D%7D)=
//! ```math
//! A^{-1} = \begin{bmatrix}
//! -2yx^{-1} & -x^{-2} \\
//! 1 & 0
//! \end{bmatrix}
//! ```
//! with $`x=\omega`$, $`y=\zeta`$, $`z=x^2\sqrt{y^2-1}`$, $`\alpha_-=z-xy`$, $`\alpha_+=z+xy`$, $`\beta_-=\exp(\tau\alpha_-)`$, $`\beta_+=\exp(-\tau\alpha_+)`$
//!

// https://en.wikipedia.org/wiki/Discretization
// https://www.wolframalpha.com/input/?i=inverse+%7B%7B0%2C+1%7D%2C+%7B-x%5E2%2C+-2yx%7D%7D
// https://www.wolframalpha.com/input/?i=Matrixexp%5B%7B%7B0%2Ct%7D%2C%7B-tx%5E2%2C-2txy%7D%7D%5D

use nalgebra::Matrix2;
use num_complex::Complex;
use serde::Serialize;
use std::fmt;

/// This structure is used to convert a continuous 2nd order ODE into a discrete state space model
#[derive(Debug, Serialize, Clone, Default, PartialEq)]
pub struct Exponential {
    /// Sampling time is second
    pub tau: f64,
    q: (f64, f64, f64, f64),
    m: (f64, f64),
    b: Vec<f64>,
    c: Vec<f64>,
    /// State space model output vector
    pub y: Vec<f64>,
    x: (f64, f64),
}
impl Exponential {
    pub fn n_inputs(&self) -> usize {
        self.b.len()
    }
    pub fn n_outputs(&self) -> usize {
        self.c.len()
    }
}
impl super::Solver for Exponential {
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
        let aa = Matrix2::<f64>::new(0., 1., -omega * omega, -2. * omega * zeta);
        let i = Matrix2::<f64>::identity();
        let qp = i + aa * (0.5 * tau);
        let iqm = (i - aa * (0.5 * tau)).try_inverse().unwrap();
        let q = (qp * iqm).as_slice().to_owned();
        let m = (iqm * tau.sqrt()).as_slice().to_owned();
        */
        let i = Matrix2::<f64>::identity();
        let n = continuous_cc.len();
        if omega == 0f64 {
            Self {
                tau,
                q: (1f64, tau, 0f64, 1f64),
                m: (0.5 * tau * tau, tau),
                b: continuous_bb,
                c: continuous_cc,
                y: vec![0.; n],
                x: (0f64, 0f64),
            }
        } else {
            let x = Complex { re: omega, im: 0. };
            let y = Complex { re: zeta, im: 0. };
            let ia = Matrix2::new((-2. * y / x).re, -1. / (x * x).re, 1., 0.);
            let z = (x * x * (y * y - 1.)).sqrt();
            let zmxy = z - x * y;
            let zpxy = z + x * y;
            let ezmxy = (tau * zmxy).exp();
            let ezpxy = (-tau * zpxy).exp();
            let ad = Matrix2::new(
                ((zpxy * ezmxy + zmxy * ezpxy) / (2. * z)).re,
                ((ezmxy - ezpxy) / (2. * z)).re,
                (x * x * (ezpxy - ezmxy) / (2. * z)).re,
                ((zmxy * ezmxy + zpxy * ezpxy) / (2. * z)).re,
            );
            let bd_ = ia * (ad - i); // / tau.sqrt();
            Self {
                tau,
                q: (ad[0], ad[2], ad[1], ad[3]),
                m: (bd_[2], bd_[3]),
                b: continuous_bb,
                c: continuous_cc,
                y: vec![0.; n],
                x: (0f64, 0f64),
            }
        }
    }
    /// Returns the state space model output
    fn solve(&mut self, u: &[f64]) -> &[f64] {
        let (x0, x1) = self.x;
        //let s = self.m.0 * x0 + self.m.1 * x1;
        self.y.iter_mut().zip(self.c.iter()).for_each(|(y, c)| {
            *y = c * x0;
        });
        let v = self.b.iter().zip(u).fold(0., |s, (b, u)| s + b * u);
        self.x.0 = self.q.0 * x0 + self.q.1 * x1 + self.m.0 * v;
        self.x.1 = self.q.2 * x0 + self.q.3 * x1 + self.m.1 * v;
        self.y.as_slice()
    }
}
impl fmt::Display for Exponential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "2x2 discrete state space model: {}->{} ({:.3}Hz)\n - A: {:.9?}\n - B: {:.9?}",
            self.b.len(),
            self.c.len(),
            self.tau.recip(),
            self.q,
            self.m
        )
    }
}
