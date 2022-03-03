//! This module is used to build the state space model of the telescope structure
//!
//! A state space model is represented by the structure [`DiscreteModalSolver`] that is created using the builder [`DiscreteStateSpace`].
//! The transformation of the FEM continuous 2nd order differential equation into a discrete state space model is performed by the [`Exponential`] structure (for the details of the transformation see the module [`exponential`]).
//!
//! # Example
//! The following example loads a FEM model from a pickle file and converts it into a state space model setting the sampling rate and the damping coefficients and truncating the eigen frequencies. A single input and a single output are selected, the input is initialized to 0 and we assert than the output is effectively 0 after one time step.
//! ```no_run
//! use dos::{controllers::state_space::DiscreteStateSpace, io::jar, DOS};
//! use fem::FEM;
//! use std::path::Path;
//! use simple_logger::SimpleLogger;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     SimpleLogger::new().init().unwrap();
//!     let sampling_rate = 1e3; // Hz
//!     let fem_data_path = Path::new("data").join("20210225_1447_MT_mount_v202102_ASM_wind2");
//!     let fem = FEM::from_pickle(fem_data_path.join("modal_state_space_model_2ndOrder.pkl"))?;
//!     let mut fem_ss = DiscreteStateSpace::from(fem)
//!         .sampling(sampling_rate)
//!         .proportional_damping(2. / 100.)
//!         .max_eigen_frequency(75.0) // Hz
//!         .inputs(vec![jar::OSSM1Lcl6F::new()])
//!         .outputs(vec![jar::OSSM1Lcl::new()])
//!         .build()?;
//!     let y = fem_ss
//!         .inputs(vec![jar::OSSM1Lcl6F::with(vec![0f64; 42])])?
//!         .step()?
//!         .outputs();
//!     assert_eq!(
//!         Option::<Vec<f64>>::from(&y.unwrap()[0]).unwrap()
//!             .iter()
//!             .sum::<f64>(),
//!         0f64
//!     );
//!     Ok(())
//! }
//! ```

use crate::{fem, fem_io};
use core::fmt::Debug;
use log;
use nalgebra as na;
use nalgebra::DMatrix;
use rayon::prelude::*;
use serde_pickle as pickle;
use std::{
    any::{type_name, Any},
    fmt,
    fs::File,
    marker::PhantomData,
    ops::Range,
    path::Path,
};

pub mod bilinear;
#[doc(inline)]
pub use bilinear::Bilinear;
pub mod exponential;
#[doc(inline)]
pub use exponential::Exponential;
pub mod exponential_matrix;
#[doc(inline)]
pub use exponential_matrix::ExponentialMatrix;

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
}
pub trait GetOut: SetRange + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn get_out(&self, fem: &fem::FEM) -> Option<DMatrix<f64>>;
    fn trim_out(&self, fem: &fem::FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>>;
    fn fem_type(&self) -> String;
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
}

/// This structure is the state space model builder based on a builder pattern design
#[derive(Default)]
pub struct DiscreteStateSpace<T: Solver + Default> {
    sampling: Option<f64>,
    fem: Option<Box<fem::FEM>>,
    zeta: Option<f64>,
    eigen_frequencies: Option<Vec<(usize, f64)>>,
    max_eigen_frequency: Option<f64>,
    hankel_singular_values_threshold: Option<f64>,
    n_io: Option<(usize, usize)>,
    phantom: PhantomData<T>,
    ins: Vec<Box<dyn GetIn>>,
    outs: Vec<Box<dyn GetOut>>,
}
impl<T: Solver + Default> From<fem::FEM> for DiscreteStateSpace<T> {
    /// Creates a state space model builder from a FEM structure
    fn from(fem: fem::FEM) -> Self {
        Self {
            fem: Some(Box::new(fem)),
            ..Self::default()
        }
    }
}
impl<T: Solver + Default> DiscreteStateSpace<T> {
    /// Prints information about the FEM
    pub fn fem_info(self) -> Self {
        if let Some(fem) = self.fem.as_ref() {
            println!("{}", fem);
        } else {
            println!("FEM missing!");
        }
        self
    }
    /// Set the sampling rate on Hz of the discrete state space model
    pub fn sampling(self, sampling: f64) -> Self {
        Self {
            sampling: Some(sampling),
            ..self
        }
    }
    /// Set the same proportional damping coefficients to all the modes
    pub fn proportional_damping(self, zeta: f64) -> Self {
        Self {
            zeta: Some(zeta),
            ..self
        }
    }
    ///
    pub fn use_static_gain_compensation(self, n_io: (usize, usize)) -> Self {
        Self {
            n_io: Some(n_io),
            ..self
        }
    }
    /// Overwrites some eigen frequencies in Hz
    ///
    /// Example
    /// ```rust
    /// // Setting the 1st 3 eigen values to 0
    /// fem_ss.eigen_frequencies(vec![(0,0.),(1,0.),(2,0.)])
    /// ```
    pub fn eigen_frequencies(self, eigen_frequencies: Vec<(usize, f64)>) -> Self {
        Self {
            eigen_frequencies: Some(eigen_frequencies),
            ..self
        }
    }
    /// Truncates the eigen frequencies to and including `max_eigen_frequency`
    ///
    /// The number of modes is set accordingly
    pub fn max_eigen_frequency(self, max_eigen_frequency: f64) -> Self {
        Self {
            max_eigen_frequency: Some(max_eigen_frequency),
            ..self
        }
    }
    /// Truncates the hankel singular values
    pub fn truncate_hankel_singular_values(self, hankel_singular_values_threshold: f64) -> Self {
        Self {
            hankel_singular_values_threshold: Some(hankel_singular_values_threshold),
            ..self
        }
    }
    /// Saves the eigen frequencies to a pickle data file
    pub fn dump_eigen_frequencies<P: AsRef<Path>>(self, path: P) -> Self {
        let mut file = File::create(path).unwrap();
        pickle::to_writer(
            &mut file,
            &self.fem.as_ref().unwrap().eigen_frequencies,
            true,
        )
        .unwrap();
        self
    }
    /// Sets the model input based on the input type
    pub fn ins<U>(self) -> Self
    where
        Vec<Option<fem_io::Inputs>>: fem_io::FemIo<U>,
        U: 'static + Send + Sync,
    {
        let mut ins = self.ins;
        ins.push(Box::new(SplitFem::<U>::new()));
        Self { ins, ..self }
    }
    /// Sets the model output based on the output type
    pub fn outs<U>(self) -> Self
    where
        Vec<Option<fem_io::Outputs>>: fem_io::FemIo<U>,
        U: 'static + Send + Sync,
    {
        let mut outs = self.outs;
        outs.push(Box::new(SplitFem::<U>::new()));
        Self { outs, ..self }
    }
    /// Returns the Hankel singular value for a given eigen mode
    pub fn hankel_singular_value(w: f64, z: f64, b: &[f64], c: &[f64]) -> f64 {
        let norm_x = |x: &[f64]| x.iter().map(|x| x * x).sum::<f64>().sqrt();
        0.25 * norm_x(b) * norm_x(c) / (w * z)
    }
    /// Computes the Hankel singular values
    pub fn hankel_singular_values(self) -> Result<Vec<f64>> {
        let fem = self
            .fem
            .map_or(Err(StateSpaceError::MissingArguments("FEM".to_owned())), Ok)?;
        let n_mode = fem.n_modes();
        let forces_2_modes = na::DMatrix::from_row_slice(
            n_mode,
            fem.inputs_to_modal_forces.len() / n_mode,
            &fem.inputs_to_modal_forces,
        );
        let modes_2_nodes = na::DMatrix::from_row_slice(
            fem.modal_disp_to_outputs.len() / n_mode,
            n_mode,
            &fem.modal_disp_to_outputs,
        );
        let w = fem.eigen_frequencies_to_radians();
        let zeta = match self.zeta {
            Some(zeta) => {
                log::info!("Proportional coefficients modified, new value: {:.4}", zeta);
                vec![zeta; fem.n_modes()]
            }
            None => fem.proportional_damping_vec.clone(),
        };
        Ok((0..fem.n_modes())
            .into_par_iter()
            .map(|k| {
                let b = forces_2_modes.row(k).clone_owned();
                let c = modes_2_nodes.column(k);
                Self::hankel_singular_value(w[k], zeta[k], b.as_slice(), c.as_slice())
            })
            .collect())
    }
    fn in2mode(&mut self, n_mode: usize) -> Option<DMatrix<f64>> {
        if let Some(fem) = &self.fem {
            let v: Vec<f64> = self
                .ins
                .iter_mut()
                .scan(0usize, |s, x| {
                    let mat = x.get_in(fem).unwrap();
                    let l = mat.ncols();
                    x.set_range(*s, *s + l);
                    *s += l;
                    Some(mat)
                })
                .flat_map(|x| {
                    x.column_iter()
                        .flat_map(|x| x.iter().take(n_mode).cloned().collect::<Vec<f64>>())
                        .collect::<Vec<f64>>()
                })
                .collect();
            Some(DMatrix::from_column_slice(n_mode, v.len() / n_mode, &v))
        } else {
            None
        }
    }
    fn mode2out(&mut self, n_mode: usize) -> Option<DMatrix<f64>> {
        if let Some(fem) = &self.fem {
            let v: Vec<f64> = self
                .outs
                .iter_mut()
                .scan(0usize, |s, x| {
                    let mat = x.get_out(fem).unwrap();
                    let l = mat.nrows();
                    x.set_range(*s, *s + l);
                    *s += l;
                    Some(mat)
                })
                .flat_map(|x| {
                    x.row_iter()
                        .flat_map(|x| x.iter().take(n_mode).cloned().collect::<Vec<f64>>())
                        .collect::<Vec<f64>>()
                })
                .collect();
            Some(DMatrix::from_row_slice(v.len() / n_mode, n_mode, &v))
        } else {
            None
        }
    }
    fn reduce2io(&self, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>> {
        if let Some(fem) = &self.fem {
            let m = DMatrix::from_columns(
                &self
                    .ins
                    .iter()
                    .filter_map(|x| x.trim_in(fem, matrix))
                    .flat_map(|x| x.column_iter().map(|x| x.clone_owned()).collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
            );
            Some(DMatrix::from_rows(
                &self
                    .outs
                    .iter()
                    .filter_map(|x| x.trim_out(fem, &m))
                    .flat_map(|x| x.row_iter().map(|x| x.clone_owned()).collect::<Vec<_>>())
                    .collect::<Vec<_>>(),
            ))
        } else {
            None
        }
    }
    fn properties(&self) -> Result<(Vec<f64>, usize, Vec<f64>)> {
        let fem = self
            .fem
            .as_ref()
            .map_or(Err(StateSpaceError::MissingArguments("FEM".to_owned())), Ok)?;
        let mut w = fem.eigen_frequencies_to_radians();
        if let Some(eigen_frequencies) = &self.eigen_frequencies {
            log::info!("Eigen values modified");
            eigen_frequencies.into_iter().for_each(|(i, v)| {
                w[*i] = v.to_radians();
            });
        }
        let n_modes = match self.max_eigen_frequency {
            Some(max_ef) => {
                fem.eigen_frequencies
                    .iter()
                    .fold(0, |n, ef| if ef <= &max_ef { n + 1 } else { n })
            }
            None => fem.n_modes(),
        };
        if let Some(max_ef) = self.max_eigen_frequency {
            log::info!("Eigen frequencies truncated to {:.3}Hz, hence reducing the number of modes from {} down to {}",max_ef,fem.n_modes(),n_modes)
        }
        let zeta = match self.zeta {
            Some(zeta) => {
                log::info!("Proportional coefficients modified, new value: {:.4}", zeta);
                vec![zeta; n_modes]
            }
            None => fem.proportional_damping_vec.clone(),
        };
        Ok((w, n_modes, zeta))
    }
    pub fn build(mut self) -> Result<DiscreteModalSolver<T>> {
        let tau = self.sampling.map_or(
            Err(StateSpaceError::MissingArguments("sampling".to_owned())),
            |x| Ok(1f64 / x),
        )?;

        let (w, n_modes, zeta) = self.properties()?;

        match (self.in2mode(n_modes), self.mode2out(n_modes)) {
            (Some(forces_2_modes), Some(modes_2_nodes)) => {
                log::info!("forces 2 modes: {:?}", forces_2_modes.shape());
                log::info!("modes 2 nodes: {:?}", modes_2_nodes.shape());

                let _psi_dcg = if let Some(n_io) = self.n_io {
                    println!(
                "The elements of psi_dcg corresponding to the first 14 outputs (mount encoders)
             and the first 20 inputs (mount drives) are set to zero."
		    );
                    let q = self.fem.as_mut().unwrap().reduced_static_gain(n_io);
                    let static_gain = self.reduce2io(&q.unwrap());
                    let d = na::DMatrix::from_diagonal(&na::DVector::from_row_slice(
                        &w.iter()
                            .skip(3)
                            .take(n_modes)
                            .cloned()
                            .collect::<Vec<f64>>(),
                    ))
                    .map(|x| 1f64 / (x * x));

                    let dyn_static_gain = modes_2_nodes.clone().remove_columns(0, 3)
                        * d
                        * forces_2_modes.clone().remove_rows(0, 3);
                    let mut psi_dcg = static_gain.unwrap() - dyn_static_gain;

                    let torque = self
                        .ins
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSAzDriveTorque>>()
                        })
                        .unwrap()
                        .range
                        .clone();
                    let encoder = self
                        .ins
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSAzEncoderAngle>>()
                        })
                        .unwrap()
                        .range
                        .clone();
                    for i in torque {
                        for j in encoder.clone() {
                            psi_dcg[(j, i)] = 0f64;
                        }
                    }

                    let torque = self
                        .ins
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSElDriveTorque>>()
                        })
                        .unwrap()
                        .range
                        .clone();
                    let encoder = self
                        .ins
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSElEncoderAngle>>()
                        })
                        .unwrap()
                        .range
                        .clone();
                    for i in torque {
                        for j in encoder.clone() {
                            psi_dcg[(j, i)] = 0f64;
                        }
                    }

                    let torque = self
                        .ins
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSRotDriveTorque>>()
                        })
                        .unwrap()
                        .range
                        .clone();
                    let encoder = self
                        .ins
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSRotEncoderAngle>>()
                        })
                        .unwrap()
                        .range
                        .clone();
                    for i in torque {
                        for j in encoder.clone() {
                            psi_dcg[(j, i)] = 0f64;
                        }
                    }

                    Some(psi_dcg)
                } else {
                    None
                };

                let state_space: Vec<_> = match self.hankel_singular_values_threshold {
                    Some(hsv_t) => (0..n_modes)
                        .filter_map(|k| {
                            let b = forces_2_modes.row(k).clone_owned();
                            let c = modes_2_nodes.column(k);
                            let hsv = Self::hankel_singular_value(
                                w[k],
                                zeta[k],
                                b.as_slice(),
                                c.as_slice(),
                            );
                            if hsv > hsv_t {
                                Some(T::from_second_order(
                                    tau,
                                    w[k],
                                    zeta[k],
                                    b.as_slice().to_vec(),
                                    c.as_slice().to_vec(),
                                ))
                            } else {
                                None
                            }
                        })
                        .collect(),
                    None => (0..n_modes)
                        .map(|k| {
                            let b = forces_2_modes.row(k).clone_owned();
                            let c = modes_2_nodes.column(k);
                            T::from_second_order(
                                tau,
                                w[k],
                                zeta[k],
                                b.as_slice().to_vec(),
                                c.as_slice().to_vec(),
                            )
                        })
                        .collect(),
                };
                Ok(DiscreteModalSolver {
                    u: vec![0f64; forces_2_modes.ncols()],
                    y: vec![0f64; modes_2_nodes.nrows()],
                    state_space,
                    ins: self.ins,
                    outs: self.outs,
                    ..Default::default()
                })
            }
            (Some(_), None) => Err(StateSpaceError::Matrix(
                "Failed to build modes to nodes transformation matrix".to_string(),
            )),
            (None, Some(_)) => Err(StateSpaceError::Matrix(
                "Failed to build forces to nodes transformation matrix".to_string(),
            )),
            _ => Err(StateSpaceError::Matrix(
                "Failed to build both modal transformation matrices".to_string(),
            )),
        }
    }
}

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
    psi_dcg: Option<na::DMatrix<f64>>,
    /// Static gain correction vector
    psi_times_u: Vec<f64>,
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
    pub fn from_fem(fem: fem::FEM) -> DiscreteStateSpace<T> {
        fem.into()
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
            .find_map(|x| x.as_any().downcast_ref::<SplitFem<U>>())
            .map(|io| self.y[io.range.start..io.range.end].to_vec())
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
        if let Some(io) = self
            .ins
            .iter()
            .find_map(|x| x.as_any().downcast_ref::<SplitFem<U>>())
        {
            self.u[io.range.start..io.range.end].copy_from_slice(u);
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_second_order() {
        let snd_ord = SecondOrder::from_pickle("/media/rconan/FEM/20210614_2105_ASM_topendOnly/modal_state_space_model_2ndOrder_1500Hz_noRes_postproc.pkl").unwrap();
        let _dms: DiscreteModalSolver<Exponential> = (snd_ord, 1e-3).into();
    }
}
