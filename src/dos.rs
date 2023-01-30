//! This module is used to build the state space model of the telescope structure
//!
//! A state space model is represented by the structure [DiscreteModalSolver] that is created using the builder [`DiscreteStateSpace`].
//! The transformation of the FEM continuous 2nd order differential equation
//! into a discrete state space model is performed by the [Exponential] structure
//! (for the details of the transformation see the `exponential` module ).
//!
//! # Example
//! The following example loads a FEM model and converts it into a state space model
//! setting the sampling rate and the damping coefficients and truncating the eigen frequencies.
//! A single input and a single output are selected.
//! ```no_run
//! use gmt_fem::{FEM,
//!               dos::{DiscreteStateSpace, DiscreteModalSolver, Exponential},
//!               fem_io::{OSSM1Lcl6F, OSSM1Lcl}};
//!
//! # fn main() -> anyhow::Result<()> {
//!     let sampling_rate = 1e3; // Hz
//!     let fem = FEM::from_env()?;
//!     let mut fem_ss: DiscreteModalSolver<Exponential> = DiscreteStateSpace::from(fem)
//!         .sampling(sampling_rate)
//!         .proportional_damping(2. / 100.)
//!         .max_eigen_frequency(75.0) // Hz
//!         .ins::<OSSM1Lcl6F>()
//!         .outs::<OSSM1Lcl>()
//!         .build()?;
//! # Ok::<(), anyhow::Error>(())
//! # }
//! ```

use crate::{fem, fem_io};
use nalgebra::DMatrix;
use std::{
    any::{type_name, Any},
    fmt,
    fmt::Debug,
    marker::PhantomData,
    ops::Range,
};

mod bilinear;
pub use bilinear::Bilinear;
mod exponential;
pub use exponential::Exponential;
mod exponential_matrix;
pub use exponential_matrix::ExponentialMatrix;
mod discrete_state_space;
pub use discrete_state_space::DiscreteStateSpace;
mod discrete_modal_solver;
pub use discrete_modal_solver::DiscreteModalSolver;

#[cfg(feature = "full")]
pub mod actors_interface;

pub trait Solver {
    fn from_second_order(
        tau: f64,
        omega: f64,
        zeta: f64,
        continuous_bb: Vec<f64>,
        continuous_cc: Vec<f64>,
    ) -> Self;
    fn solve(&mut self, u: &[f64]) -> &[f64];
}

#[derive(Debug)]
pub enum StateSpaceError {
    MissingArguments(String),
    SamplingFrequency,
    Matrix(String),
}
impl fmt::Display for StateSpaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingArguments(v) => write!(f, "argument {:?} is missing", v),
            Self::SamplingFrequency => f.write_str("sampling frequency not set"),
            Self::Matrix(msg) => write!(f, "{}", msg),
        }
    }
}
impl std::error::Error for StateSpaceError {}
type Result<T> = std::result::Result<T, StateSpaceError>;

pub struct SplitFem<U> {
    range: Range<usize>,
    io: PhantomData<U>,
}

impl<U> SplitFem<U> {
    fn new() -> Self {
        Self {
            range: Range::default(),
            io: PhantomData,
        }
    }
    pub fn fem_type(&self) -> String {
        type_name::<U>().to_string()
    }
}
impl<U> Debug for SplitFem<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("SplitFem<{}>", self.fem_type()))
            .field("range", &self.range)
            .finish()
    }
}
impl<U> Default for SplitFem<U> {
    fn default() -> Self {
        Self::new()
    }
}
pub trait SetRange {
    fn set_range(&mut self, start: usize, end: usize);
}
impl<U> SetRange for SplitFem<U> {
    fn set_range(&mut self, start: usize, end: usize) {
        self.range = Range { start, end };
    }
}
pub trait GetIn: SetRange + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn get_in(&self, fem: &fem::FEM) -> Option<DMatrix<f64>>;
    fn trim_in(&self, fem: &fem::FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>>;
    fn fem_type(&self) -> String;
    fn range(&self) -> Range<usize>;
}
impl<U: 'static + Send + Sync> GetIn for SplitFem<U>
where
    Vec<Option<fem_io::Inputs>>: fem_io::FemIo<U>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn get_in(&self, fem: &fem::FEM) -> Option<DMatrix<f64>> {
        fem.in2modes::<U>()
            .as_ref()
            .map(|x| DMatrix::from_row_slice(fem.n_modes(), x.len() / fem.n_modes(), x))
    }
    fn trim_in(&self, fem: &fem::FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>> {
        fem.trim2in::<U>(matrix)
    }
    fn fem_type(&self) -> String {
        self.fem_type()
    }

    fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}
pub trait GetOut: SetRange + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn get_out(&self, fem: &fem::FEM) -> Option<DMatrix<f64>>;
    fn trim_out(&self, fem: &fem::FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>>;
    fn fem_type(&self) -> String;
    fn range(&self) -> Range<usize>;
}
impl<U: 'static + Send + Sync> GetOut for SplitFem<U>
where
    Vec<Option<fem_io::Outputs>>: fem_io::FemIo<U>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn get_out(&self, fem: &fem::FEM) -> Option<DMatrix<f64>> {
        fem.modes2out::<U>()
            .as_ref()
            .map(|x| DMatrix::from_row_slice(x.len() / fem.n_modes(), fem.n_modes(), x))
    }
    fn trim_out(&self, fem: &fem::FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>> {
        fem.trim2out::<U>(matrix)
    }
    fn fem_type(&self) -> String {
        self.fem_type()
    }

    fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

pub trait Get<U> {
    fn get(&self) -> Option<Vec<f64>>;
}
impl<T: Solver + Default, U: 'static> Get<U> for DiscreteModalSolver<T>
where
    Vec<Option<fem_io::Outputs>>: fem_io::FemIo<U>,
{
    fn get(&self) -> Option<Vec<f64>> {
        self.outs
            .iter()
            .find(|&x| x.as_any().is::<SplitFem<U>>())
            .map(|io| self.y[io.range()].to_vec())
    }
}
pub trait Set<U> {
    fn set(&mut self, u: &[f64]);
}
impl<T: Solver + Default, U: 'static> Set<U> for DiscreteModalSolver<T>
where
    Vec<Option<fem_io::Inputs>>: fem_io::FemIo<U>,
{
    fn set(&mut self, u: &[f64]) {
        if let Some(io) = self.ins.iter().find(|&x| x.as_any().is::<SplitFem<U>>()) {
            self.u[io.range()].copy_from_slice(u);
        }
    }
}
