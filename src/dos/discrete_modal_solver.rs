use super::{DiscreteStateSpace, Exponential, ExponentialMatrix, GetIn, GetOut, Solver};
use crate::FEM;
use nalgebra as na;
use rayon::prelude::*;
use std::fmt;

/// This structure represents the actual state space model of the telescope
///
/// The state space discrete model is made of several discrete 2nd order different equation solvers, all independent and solved concurrently
#[derive(Debug, Default)]
pub struct DiscreteModalSolver<T: Solver + Default> {
    /// Model input vector
    pub u: Vec<f64>,
    /// Model output vector
    pub y: Vec<f64>,
    pub y_sizes: Vec<usize>,
    /// vector of state models
    pub state_space: Vec<T>,
    /// Static gain correction matrix
    pub psi_dcg: Option<na::DMatrix<f64>>,
    /// Static gain correction vector
    pub psi_times_u: Vec<f64>,
    pub ins: Vec<Box<dyn GetIn>>,
    pub outs: Vec<Box<dyn GetOut>>,
}
impl<T: Solver + Default> DiscreteModalSolver<T> {
    /*
      /// Serializes the model using [bincode](https://docs.rs/bincode/1.3.3/bincode/)
      fn dump(&self, filename: &str) -> REs {
      let file = File::create(filename)
      }
    */
    /// Returns the FEM state space builer
    pub fn from_fem(fem: FEM) -> DiscreteStateSpace<T> {
        fem.into()
    }
}

impl Iterator for DiscreteModalSolver<Exponential> {
    type Item = ();
    fn next(&mut self) -> Option<Self::Item> {
        let n = self.y.len();
        //        match &self.u {
        let _u_ = &self.u;
        self.y = self
            .state_space
            .par_iter_mut()
            .fold(
                || vec![0f64; n],
                |mut a: Vec<f64>, m| {
                    a.iter_mut().zip(m.solve(_u_)).for_each(|(yc, y)| {
                        *yc += y;
                    });
                    a
                },
            )
            .reduce(
                || vec![0f64; n],
                |mut a: Vec<f64>, b: Vec<f64>| {
                    a.iter_mut().zip(b.iter()).for_each(|(a, b)| {
                        *a += *b;
                    });
                    a
                },
            );
        Some(())
    }
}

impl Iterator for DiscreteModalSolver<ExponentialMatrix> {
    type Item = ();
    fn next(&mut self) -> Option<Self::Item> {
        let n = self.y.len();
        //        match &self.u {
        let _u_ = &self.u;
        self.y = self
            .state_space
            .par_iter_mut()
            .fold(
                || vec![0f64; n],
                |mut a: Vec<f64>, m| {
                    a.iter_mut().zip(m.solve(_u_)).for_each(|(yc, y)| {
                        *yc += y;
                    });
                    a
                },
            )
            .reduce(
                || vec![0f64; n],
                |mut a: Vec<f64>, b: Vec<f64>| {
                    a.iter_mut().zip(b.iter()).for_each(|(a, b)| {
                        *a += *b;
                    });
                    a
                },
            );

        if let Some(psi_dcg) = &self.psi_dcg {
            self.y = self
                .y
                .iter_mut()
                .zip(self.psi_times_u.iter_mut())
                .map(|(v1, v2)| *v1 + *v2)
                .collect::<Vec<f64>>();

            let u_nalgebra = na::DVector::from_column_slice(&self.u);
            self.psi_times_u = (psi_dcg * u_nalgebra).as_slice().to_vec();
        }

        Some(())
    }
}
impl<T: Solver + Default> fmt::Display for DiscreteModalSolver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r##"
DiscreteModalSolver:
 - inputs:
{:}
 - outputs:
{:}
 - {:} 2x2 state space models
"##,
            self.ins
                .iter()
                .map(|x| x.fem_type())
                .collect::<Vec<String>>()
                .join("\n"),
            self.outs
                .iter()
                .map(|x| x.fem_type())
                .collect::<Vec<String>>()
                .join("\n"),
            self.state_space.len(),
        )
    }
}
