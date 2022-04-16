use super::{DiscreteModalSolver, GetIn, GetOut, Result, Solver, SplitFem, StateSpaceError};
use crate::{fem_io, FEM};
use nalgebra as na;
use nalgebra::DMatrix;
use rayon::prelude::*;
use serde_pickle as pickle;
use std::ops::Range;
use std::{fs::File, marker::PhantomData, path::Path};

/// This structure is the state space model builder based on a builder pattern design
#[derive(Default)]
pub struct DiscreteStateSpace<T: Solver + Default> {
    sampling: Option<f64>,
    fem: Option<Box<FEM>>,
    zeta: Option<f64>,
    eigen_frequencies: Option<Vec<(usize, f64)>>,
    max_eigen_frequency: Option<f64>,
    hankel_singular_values_threshold: Option<f64>,
    n_io: Option<(usize, usize)>,
    phantom: PhantomData<T>,
    ins: Vec<Box<dyn GetIn>>,
    outs: Vec<Box<dyn GetOut>>,
    outs_transform: Vec<Option<DMatrix<f64>>>,
}
impl<T: Solver + Default> From<FEM> for DiscreteStateSpace<T> {
    /// Creates a state space model builder from a FEM structure
    fn from(fem: FEM) -> Self {
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
        let Self {
            mut outs,
            mut outs_transform,
            ..
        } = self;
        outs.push(Box::new(SplitFem::<U>::new()));
        outs_transform.push(None);
        Self {
            outs,
            outs_transform,
            ..self
        }
    }
    pub fn outs_with<U>(self, transform: DMatrix<f64>) -> Self
    where
        Vec<Option<fem_io::Outputs>>: fem_io::FemIo<U>,
        U: 'static + Send + Sync,
    {
        let Self {
            mut outs,
            mut outs_transform,
            ..
        } = self;
        outs.push(Box::new(SplitFem::<U>::new()));
        outs_transform.push(Some(transform));
        Self {
            outs,
            outs_transform,
            ..self
        }
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
                .zip(&self.outs_transform)
                .scan(0usize, |s, (x, t)| {
                    let mat = if let Some(t) = t {
                        t * x.get_out(fem).unwrap()
                    } else {
                        x.get_out(fem).unwrap()
                    };
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
                    .zip(&self.outs_transform)
                    .filter_map(|(x, t)| match (x.trim_out(fem, &m), t) {
                        (Some(x), Some(t)) => Some(t * x),
                        (x, None) => x,
                        _ => None,
                    })
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

                let psi_dcg = if let Some(n_io) = self.n_io {
                    println!(
                        "The elements of psi_dcg corresponding to 
    - OSSAzDriveTorque
    - OSSElDriveTorque
    - OSSRotDriveTorque
and
    - OSSAzEncoderAngle
    - OSSElEncoderAngle
    - OSSRotEncoderAngle
are set to zero."
                    );
                    let q = self
                        .fem
                        .as_mut()
                        .unwrap()
                        .static_gain
                        .as_ref()
                        .map(|x| DMatrix::from_row_slice(n_io.1, n_io.0, x));
                    let static_gain = self
                        .reduce2io(&q.unwrap())
                        .expect("Failed to produce FEM static gain");
                    let d = na::DMatrix::from_diagonal(&na::DVector::from_row_slice(
                        &w.iter()
                            .skip(3)
                            .take(n_modes - 3)
                            .map(|x| x.recip())
                            .map(|x| x * x)
                            .collect::<Vec<f64>>(),
                    ));
                    let dyn_static_gain = modes_2_nodes.clone().remove_columns(0, 3)
                        * d
                        * forces_2_modes.clone().remove_rows(0, 3);

                    let mut psi_dcg = static_gain - dyn_static_gain;

                    let az_torque: Option<Range<usize>> = self
                        .ins
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSAzDriveTorque>>()
                        })
                        .map(|x| x.range.clone());
                    let az_encoder = self
                        .outs
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSAzEncoderAngle>>()
                        })
                        .map(|x| x.range.clone());

                    let el_torque = self
                        .ins
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSElDriveTorque>>()
                        })
                        .map(|x| x.range.clone());
                    let el_encoder = self
                        .outs
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSElEncoderAngle>>()
                        })
                        .map(|x| x.range.clone());

                    let rot_torque = self
                        .ins
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSRotDriveTorque>>()
                        })
                        .map(|x| x.range.clone());
                    let rot_encoder = self
                        .outs
                        .iter()
                        .find_map(|x| {
                            x.as_any()
                                .downcast_ref::<SplitFem<fem_io::OSSRotEncoderAngle>>()
                        })
                        .map(|x| x.range.clone());

                    let torque_indices: Vec<_> = az_torque
                        .into_iter()
                        .chain(el_torque.into_iter())
                        .chain(rot_torque.into_iter())
                        .flat_map(|x| x.collect::<Vec<usize>>())
                        .collect();
                    let enc_indices: Vec<_> = az_encoder
                        .into_iter()
                        .chain(el_encoder.into_iter())
                        .chain(rot_encoder.into_iter())
                        .flat_map(|x| x.collect::<Vec<usize>>())
                        .collect();

                    for i in torque_indices {
                        for j in enc_indices.clone() {
                            psi_dcg[(j, i)] = 0f64;
                            // println!("({},{})",j,i);
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
                    psi_dcg,
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
