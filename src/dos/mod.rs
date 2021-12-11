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

use crate::fem;
use core::fmt::Debug;
use dosio::{
    io::{IOError, Tags},
    DOSIOSError, Dos, IOTags, IO,
};
use log;
use nalgebra as na;
use rayon::prelude::*;
use serde_pickle as pickle;
use std::{fmt, fs::File, path::Path};

pub mod bilinear;
#[doc(inline)]
pub use bilinear::Bilinear;
pub mod exponential;
#[doc(inline)]
pub use exponential::Exponential;
pub mod second_order;
#[doc(inline)]
pub use second_order::SecondOrder;
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
    FemInputs(Tags),
    FemOutputs(Tags),
    MissingArguments(String),
    SamplingFrequency,
    MissingIO(IOError<Vec<f64>>),
}
impl fmt::Display for StateSpaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FemInputs(t) => write!(f, "no match for {:?} in FEM inputs", t),
            Self::FemOutputs(t) => write!(f, "no match for {:?} in FEM outputs", t),
            Self::MissingArguments(v) => write!(f, "argument {:?} is missing", v),
            Self::SamplingFrequency => f.write_str("sampling frequency not set"),
            Self::MissingIO(_) => f.write_str("DOS IO not found"),
        }
    }
}
impl From<IOError<Vec<f64>>> for StateSpaceError {
    fn from(source: IOError<Vec<f64>>) -> Self {
        Self::MissingIO(source)
    }
}
impl std::error::Error for StateSpaceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingIO(source) => Some(source),
            _ => None,
        }
    }
}

type Result<T> = std::result::Result<T, StateSpaceError>;
type StateSpaceIO = Vec<IO<Vec<f64>>>;

fem_macros::match_maker! {}

/// This structure is the state space model builder based on a builder pattern design
#[derive(Default)]
pub struct DiscreteStateSpace {
    sampling: Option<f64>,
    fem: Option<Box<fem::FEM>>,
    u: Option<StateSpaceIO>,
    y: Option<StateSpaceIO>,
    zeta: Option<f64>,
    eigen_frequencies: Option<Vec<(usize, f64)>>,
    max_eigen_frequency: Option<f64>,
    hankel_singular_values_threshold: Option<f64>,
}
impl From<fem::FEM> for DiscreteStateSpace {
    /// Creates a state space model builder from a FEM structure
    fn from(fem: fem::FEM) -> Self {
        Self {
            fem: Some(Box::new(fem)),
            ..Self::default()
        }
    }
}
impl DiscreteStateSpace {
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
    /// Sets the model inputs from a vector of [IO]
    pub fn inputs(self, v_u: Vec<Tags>) -> Self {
        let mut u = self.u.unwrap_or(vec![]);
        v_u.iter().for_each(|t| {
            let io: IO<Vec<f64>> = (t, Option::<Vec<f64>>::None).into();
            u.push(io);
        });

        Self { u: Some(u), ..self }
    }
    pub fn inputs_with(self, v_u: Vec<Tags>, u_mat: impl Iterator<Item = Vec<f64>>) -> Self {
        let mut u = self.u.unwrap_or(vec![]);
        v_u.iter().zip(u_mat).for_each(|(t, mat)| {
            let io: IO<Vec<f64>> = (t, Some(mat.to_owned())).into();
            u.push(io);
        });

        Self { u: Some(u), ..self }
    }
    /// Sets the model inputs based on the outputs of another component
    pub fn inputs_from(self, elements: &[&dyn IOTags]) -> Self {
        elements
            .iter()
            .fold(self, |this, element| this.inputs(element.outputs_tags()))
    }
    /// Sets the model outputs from a vector of [IO]
    pub fn outputs(self, v_y: Vec<Tags>) -> Self {
        let mut y = self.y.unwrap_or(vec![]);
        v_y.iter().for_each(|t| {
            let io: IO<Vec<f64>> = (t, Option::<Vec<f64>>::None).into();
            y.push(io);
        });
        Self { y: Some(y), ..self }
    }
    pub fn outputs_with(self, v_y: Vec<Tags>, y_mat: impl Iterator<Item = Vec<f64>>) -> Self {
        let mut y = self.y.unwrap_or(vec![]);
        v_y.iter().zip(y_mat).for_each(|(t, mat)| {
            let io: IO<Vec<f64>> = (t, Some(mat.to_owned())).into();
            y.push(io);
        });
        Self { y: Some(y), ..self }
    }
    /// Sets the model outputs based on the inputs of another component
    pub fn outputs_to(self, elements: &[&dyn IOTags]) -> Self {
        elements
            .iter()
            .fold(self, |this, element| this.outputs(element.inputs_tags()))
    }
    fn select_fem_io(fem: &mut fem::FEM, dos_inputs: &StateSpaceIO, dos_outputs: &StateSpaceIO) {
        log::debug!("\n## WHOLE FEM ##\n{}", fem);
        let inputs_idx: Vec<_> = fem
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(k, i)| {
                dos_inputs
                    .iter()
                    .find_map(|d| i.as_ref().and_then(|i| d.match_fem_inputs(i)).and(Some(k)))
            })
            .collect();
        let outputs_idx: Vec<_> = fem
            .outputs
            .iter()
            .enumerate()
            .filter_map(|(k, i)| {
                dos_outputs
                    .iter()
                    .find_map(|d| i.as_ref().and_then(|i| d.match_fem_outputs(i)).and(Some(k)))
            })
            .collect();
        fem.keep_inputs(&inputs_idx).keep_outputs(&outputs_idx);
        log::info!("\n## REDUCED FEM ##\n{}", fem);
    }
    fn io2modes(fem: &fem::FEM, dos_inputs: &StateSpaceIO) -> Result<Vec<f64>> {
        use crate::io::IO;
        let n_mode = fem.n_modes();
        let n = fem.inputs_to_modal_forces.len() / fem.n_modes();
        Ok(dos_inputs
            .iter()
            .map(|x| {
                fem.inputs
                    .iter()
                    .find_map(|y| y.as_ref().and_then(|y| x.match_fem_inputs(y)))
                    .map(|io| (io, x.into()))
                    .ok_or(StateSpaceError::FemInputs(x.into()))
            })
            .collect::<Result<Vec<(Vec<IO>, Option<Vec<f64>>)>>>()?
            .iter()
            .map(|(v, mat)| {
                (
                    v.iter()
                        .filter_map(|x| match x {
                            IO::On(io) => Some(io.indices.clone()),
                            IO::Off(_) => None,
                        })
                        .flatten()
                        .collect::<Vec<u32>>(),
                    mat,
                )
            })
            .flat_map(|(indice, mat)| {
                {
                    let n_i = indice.len();
                    let i2m = na::DMatrix::from_row_slice(
                        n_mode,
                        n_i,
                        &(fem
                            .inputs_to_modal_forces
                            .chunks(n)
                            .flat_map(|x| {
                                indice
                                    .iter()
                                    .map(|i| x[*i as usize - 1])
                                    .collect::<Vec<f64>>()
                            })
                            .collect::<Vec<f64>>()),
                    );
                    match mat {
                        Some(mat) => {
                            i2m * na::DMatrix::from_column_slice(n_i, mat.len() / n_i, &mat)
                        }
                        None => i2m,
                    }
                }
                .as_slice()
                .to_vec()
            })
            .collect())
    }
    fn modes2io(fem: &fem::FEM, dos_outputs: &StateSpaceIO) -> Result<Vec<Vec<f64>>> {
        use crate::io::IO;
        let n = fem.n_modes();
        let q: Vec<_> = fem.modal_disp_to_outputs.chunks(n).collect();
        Ok(dos_outputs
            .iter()
            .map(|x| {
                fem.outputs
                    .iter()
                    .find_map(|y| y.as_ref().and_then(|y| x.match_fem_outputs(y)))
                    .map(|io| (io, x.into()))
                    .ok_or(StateSpaceError::FemOutputs(x.into()))
            })
            .collect::<Result<Vec<(Vec<IO>, Option<Vec<f64>>)>>>()?
            .into_iter()
            .map(|(v, mat)| {
                (
                    v.into_iter()
                        .filter_map(|x| match x {
                            IO::On(io) => Some(io.indices),
                            IO::Off(_) => None,
                        })
                        .flatten()
                        .collect::<Vec<u32>>(),
                    mat,
                )
            })
            .map(|(i, mat)| {
                (
                    i.iter()
                        .flat_map(|i| q[*i as usize - 1].to_owned())
                        .collect::<Vec<f64>>(),
                    mat,
                )
            })
            .map(|(m2o, mat)| match mat {
                Some(mat) => {
                    let n_mode = fem.n_modes();
                    let n_o = m2o.len() / n_mode;
                    let mat_n_o = mat.len() / n_o;
                    (na::DMatrix::from_column_slice(mat_n_o, n_o, &mat)
                        * na::DMatrix::from_row_slice(n_o, n_mode, &m2o))
                    .as_slice()
                    .to_vec()
                }
                None => m2o,
            })
            .collect())
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
    /// Builds the state space discrete model
    pub fn build<T: Solver>(self) -> Result<DiscreteModalSolver<T>> {
        let tau = self.sampling.map_or(
            Err(StateSpaceError::MissingArguments("sampling".to_owned())),
            |x| Ok(1f64 / x),
        )?;
        let mut fem = self
            .fem
            .map_or(Err(StateSpaceError::MissingArguments("FEM".to_owned())), Ok)?;
        let dos_inputs: Vec<IO<()>> = self
            .u
            .as_ref()
            .map_or(
                Err(StateSpaceError::MissingArguments("inputs".to_owned())),
                Ok,
            )?
            .iter()
            .map(|io| {
                let t: IO<()> = io.into();
                t
            })
            .collect();
        let dos_outputs: Vec<IO<()>> = self
            .y
            .as_ref()
            .map_or(
                Err(StateSpaceError::MissingArguments("outputs".to_owned())),
                Ok,
            )?
            .iter()
            .map(|io| {
                let t: IO<()> = io.into();
                t
            })
            .collect();
        Self::select_fem_io(&mut fem, self.u.as_ref().unwrap(), self.y.as_ref().unwrap());
        let n_mode = fem.n_modes();
        let fem_io2modes = Self::io2modes(&fem, self.u.as_ref().unwrap())?;
        let forces_2_modes =
            na::DMatrix::from_column_slice(n_mode, fem_io2modes.len() / n_mode, &fem_io2modes);
        log::info!("forces 2 modes: {:?}", forces_2_modes.shape());
        let (sizes, fem_modes2io): (Vec<_>, Vec<_>) =
            Self::modes2io(&fem, self.y.as_ref().unwrap())?
                .into_iter()
                .map(|f| (f.len() / fem.n_modes(), f))
                .unzip();
        let modes_2_nodes = na::DMatrix::from_row_slice(
            sizes.iter().sum::<usize>(),
            n_mode,
            &fem_modes2io.into_iter().flatten().collect::<Vec<f64>>(),
        );
        log::info!("modes 2 nodes: {:?}", modes_2_nodes.shape());
        let mut w = fem.eigen_frequencies_to_radians();
        if let Some(eigen_frequencies) = self.eigen_frequencies {
            log::info!("Eigen values modified");
            eigen_frequencies.into_iter().for_each(|(i, v)| {
                w[i] = v.to_radians();
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
            None => fem.proportional_damping_vec,
        };
        let state_space: Vec<_> = match self.hankel_singular_values_threshold {
            Some(hsv_t) => (0..n_modes)
                .filter_map(|k| {
                    let b = forces_2_modes.row(k).clone_owned();
                    let c = modes_2_nodes.column(k);
                    let hsv =
                        Self::hankel_singular_value(w[k], zeta[k], b.as_slice(), c.as_slice());
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
            u_tags: dos_inputs,
            y: vec![0f64; modes_2_nodes.nrows()],
            y_tags: dos_outputs,
            y_sizes: sizes,
            state_space,
        })
    }
}

/// This structure represents the actual state space model of the telescope
///
/// The state space discrete model is made of several discrete 2nd order different equation solvers, all independent and solved concurrently
#[derive(Debug, Default)]
pub struct DiscreteModalSolver<T: Solver> {
    /// Model input vector
    pub u: Vec<f64>,
    u_tags: Vec<Tags>,
    /// Model output vector
    pub y: Vec<f64>,
    y_sizes: Vec<usize>,
    y_tags: Vec<Tags>,
    /// vector of state models
    pub state_space: Vec<T>,
}
impl<T: Solver> DiscreteModalSolver<T> {
    /// Returns the model outputs filled with zeros
    pub fn zeroed_outputs(&self) -> Vec<IO<Vec<f64>>> {
        self.y_tags
            .iter()
            .zip(self.y_sizes.iter())
            .map(|(t, &n)| (t, vec![0f64; n]).into())
            .collect()
    }
    /*
        /// Serializes the model using [bincode](https://docs.rs/bincode/1.3.3/bincode/)
        fn dump(&self, filename: &str) -> REs {
        let file = File::create(filename)
        }
    */
}
impl<T: Solver> From<(SecondOrder, f64)> for DiscreteModalSolver<T> {
    fn from((second_order, sampling_rate): (SecondOrder, f64)) -> Self {
        let n_in = second_order.n_u();
        let n_out = second_order.n_y();
        let n_mode = second_order.n_mode();
        let b = na::DMatrix::from_row_slice(n_mode, n_in, &second_order.b);
        let c = na::DMatrix::from_row_slice(n_out, n_mode, &second_order.c);
        let tau = sampling_rate.recip();
        let state_space: Vec<_> = (0..n_mode)
            .map(|k| {
                let b_row = b.row(k).clone_owned();
                let c_col = c.column(k);
                let w = second_order.omega[k] * 2. * std::f64::consts::PI;
                T::from_second_order(
                    tau,
                    w,
                    second_order.zeta[k],
                    b_row.as_slice().to_vec(),
                    c_col.as_slice().to_vec(),
                )
            })
            .collect();
        DiscreteModalSolver {
            u: vec![0f64; n_in],
            u_tags: second_order.u.name,
            y: vec![0f64; n_out],
            y_tags: second_order.y.name,
            y_sizes: second_order.y.size,
            state_space,
        }
    }
}
impl<T: Solver> From<(SecondOrder, f64, (usize, usize))> for DiscreteModalSolver<T> {
    fn from(
        (second_order, sampling_rate, (skip, take)): (SecondOrder, f64, (usize, usize)),
    ) -> Self {
        let n_in = second_order.n_u();
        let n_out = second_order.n_y();
        let n_mode = second_order.n_mode();
        let b = na::DMatrix::from_row_slice(n_mode, n_in, &second_order.b);
        let c = na::DMatrix::from_row_slice(n_out, n_mode, &second_order.c);
        let tau = sampling_rate.recip();
        let state_space: Vec<_> = (0..n_mode)
            .skip(skip)
            .take(take)
            .map(|k| {
                let b_row = b.row(k).clone_owned();
                let c_col = c.column(k);
                let w = second_order.omega[k] * 2. * std::f64::consts::PI;
                T::from_second_order(
                    tau,
                    w,
                    second_order.zeta[k],
                    b_row.as_slice().to_vec(),
                    c_col.as_slice().to_vec(),
                )
            })
            .collect();
        DiscreteModalSolver {
            u: vec![0f64; n_in],
            u_tags: second_order.u.name,
            y: vec![0f64; n_out],
            y_tags: second_order.y.name,
            y_sizes: second_order.y.size,
            state_space,
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
        Some(())
    }
}

impl<T: Solver> Dos for DiscreteModalSolver<T> {
    type Input = Vec<f64>;
    type Output = Vec<f64>;
    fn inputs(
        &mut self,
        data: Option<Vec<IO<Self::Input>>>,
    ) -> std::result::Result<&mut Self, DOSIOSError> {
        /*self.u = data
        .unwrap()
        .into_iter()
        .map(|x| {
            std::result::Result::<Vec<f64>, IOError<Vec<f64>>>::from(x)
                .map_err(|e| DOSIOSError::Inputs(e.into()))
        })
        .collect::<std::result::Result<Vec<Vec<f64>>, DOSIOSError>>()?
        .into_iter()
        .flatten()
        .collect();*/
        //let d = data.as_ref().unwrap();
        //self.u_tags.iter().map(|t| d[t].clone());
        self.u = data
            .map(|data| {
                self.u_tags
                    .iter()
                    .map(move |tag| {
                        std::result::Result::<Vec<f64>, IOError<Vec<f64>>>::from(data[tag].clone())
                            .map_err(|e| DOSIOSError::Inputs(e.into()))
                    })
                    .collect::<std::result::Result<Vec<Vec<f64>>, DOSIOSError>>()
            })
            .transpose()?
            .map(|x| x.into_iter().flatten().collect::<Vec<f64>>())
            .ok_or(DOSIOSError::Inputs("FEM inputs missing!".into()))?;
        Ok(self)
    }
    fn outputs(&mut self) -> Option<Vec<IO<Self::Output>>> {
        let mut pos = 0;
        self.y_tags
            .iter()
            .zip(self.y_sizes.iter())
            .map(|(t, n)| {
                let io = IO::<Vec<f64>>::from((t, self.y[pos..pos + n].to_vec()));
                pos += n;
                Some(io)
            })
            .collect()
    }
}
impl<T: Solver> IOTags for DiscreteModalSolver<T> {
    fn outputs_tags(&self) -> Vec<Tags> {
        self.y_tags.clone()
    }
    fn inputs_tags(&self) -> Vec<Tags> {
        self.u_tags.clone()
    }
}
impl<T: Solver> fmt::Display for DiscreteModalSolver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r##"
DiscreteModalSolver:
 - inputs:
 {:#?}
 - outputs:
 {:#?}
 - {:} 2x2 state space models
"##,
            self.u_tags
                .iter()
                .map(|t| t.kind())
                .collect::<Vec<String>>(),
            self.y_tags
                .iter()
                .map(|t| t.kind())
                .collect::<Vec<String>>(),
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
