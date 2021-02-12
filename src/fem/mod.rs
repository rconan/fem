use super::{Exponential, IOData, IO, wind_loads};
use anyhow::{Context, Result};
use nalgebra as na;
use rayon::prelude::*;
use serde;
use serde::Deserialize;
use serde_pickle as pkl;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub mod fem_io;

/// Finite Element Model
#[derive(Deserialize, Debug, Clone)]
pub struct FEM {
    /// Model info
    #[serde(rename = "modelDescription")]
    pub model_description: String,
    /// inputs properties
    pub inputs: Vec<Option<fem_io::Inputs>>,
    /// outputs properties
    pub outputs: Vec<Option<fem_io::Outputs>>,
    /// mode shapes eigen frequencies [Hz]
    #[serde(rename = "eigenfrequencies")]
    pub eigen_frequencies: Vec<f64>,
    /// inputs forces to modal forces matrix [n_modes,n_inputs] (row wise)
    #[serde(rename = "inputs2ModalF")]
    inputs_to_modal_forces: Vec<f64>,
    /// mode shapes to outputs nodes [n_outputs,n_modes] (row wise)
    #[serde(rename = "modalDisp2Outputs")]
    modal_disp_to_outputs: Vec<f64>,
    /// mode shapes damping coefficients
    #[serde(rename = "proportionalDampingVec")]
    pub proportional_damping_vec: Vec<f64>,
    #[serde(skip, default)]
    pub state_space: Option<Vec<Exponential>>,
    #[serde(skip, default)]
    pub u: Option<Vec<f64>>,
}
impl FEM {
    /// Loads a FEM model saved in a second order from in a pickle file
    pub fn from_pickle<P>(path: P) -> Result<FEM>
    where
        P: AsRef<Path> + fmt::Display + Copy,
    {
        let f = File::open(path).context(format!("File {} not found", path))?;
        let r = BufReader::with_capacity(1_000_000, f);
        let v: serde_pickle::Value =
            serde_pickle::from_reader(r).context(format!("Cannot read {}", path))?;
        Ok(pkl::from_value(v).context(format!("Failed to load {}", path))?)
    }
    /// Gets the number of modes
    pub fn n_modes(&self) -> usize {
        self.eigen_frequencies.len()
    }
    /// Converts FEM eigen frequencies from Hz to radians
    pub fn eigen_frequencies_to_radians(&self) -> Vec<f64> {
        self.eigen_frequencies
            .iter()
            .map(|x| 2.0 * std::f64::consts::PI * x)
            .collect()
    }
    pub fn n_inputs(&self) -> usize {
        self.inputs
            .iter()
            .filter_map(|x| x.as_ref())
            .fold(0usize, |a, x| a + x.len())
    }
    pub fn n_outputs(&self) -> usize {
        self.outputs
            .iter()
            .filter_map(|x| x.as_ref())
            .fold(0usize, |a, x| a + x.len())
    }
    pub fn keep_inputs(&mut self, id: &[usize]) -> &mut Self {
        self.inputs.iter_mut().enumerate().for_each(|(k, i)| {
            if !id.contains(&k) {
                *i = None
            }
        });
        self
    }
    pub fn keep_inputs_by<F>(&mut self, id: &[usize], pred: F) -> &mut Self
    where
        F: Fn(&IOData) -> bool + Copy,
    {
        self.inputs.iter_mut().enumerate().for_each(|(k, i)| {
            if !id.contains(&k) {
                *i = None
            } else {
                i.as_mut().map(|i| {
                    i.iter_mut().for_each(|io| {
                        *io = io.clone().switch_off();
                        *io = io.clone().switch_on_by(pred);
                    })
                });
            }
        });
        self
    }
    pub fn keep_outputs(&mut self, id: &[usize]) -> &mut Self {
        self.outputs.iter_mut().enumerate().for_each(|(k, i)| {
            if !id.contains(&k) {
                *i = None
            }
        });
        self
    }
    /// Returns the inputs 2 modes transformation matrix for the turned-on inputs
    pub fn inputs2modes(&mut self) -> Vec<f64> {
        let indices: Vec<u32> = self
            .inputs
            .iter()
            .filter_map(|x| x.as_ref())
            .flat_map(|v| {
                v.iter().filter_map(|x| match x {
                    IO::On(io) => Some(io.indices.clone()),
                    IO::Off(_) => None,
                })
            })
            .flatten()
            .collect();
        let n = self.inputs_to_modal_forces.len() / self.n_modes();
        self.inputs_to_modal_forces
            .chunks(n)
            .flat_map(|x| {
                indices
                    .iter()
                    .map(|i| x[*i as usize - 1])
                    .collect::<Vec<f64>>()
            })
            .collect()
    }
    /// Returns the modes 2 outputs transformation matrix for the turned-on outputs
    pub fn modes2outputs(&mut self) -> Vec<f64> {
        let n = self.n_modes();
        let q: Vec<_> = self.modal_disp_to_outputs.chunks(n).collect();
        self.outputs
            .iter()
            .filter_map(|x| x.as_ref())
            .flat_map(|v| {
                v.iter().filter_map(|x| match x {
                    IO::On(io) => Some(io.indices.clone()),
                    IO::Off(_) => None,
                })
            })
            .flatten()
            .flat_map(|i| q[i as usize - 1])
            .cloned()
            .collect()
    }
    /// Returns the FEM static gain for the turned-on inputs and outputs
    pub fn static_gain(&mut self) -> na::DMatrix<f64> {
        let forces_2_modes =
            na::DMatrix::from_row_slice(self.n_modes(), self.n_inputs(), &self.inputs2modes());
        let modes_2_nodes =
            na::DMatrix::from_row_slice(self.n_outputs(), self.n_modes(), &self.modes2outputs());
        let d = na::DMatrix::from_diagonal(
            &na::DVector::from_row_slice(&self.eigen_frequencies_to_radians())
                .map(|x| 1f64 / (x * x)),
        );
        modes_2_nodes * d * forces_2_modes
    }
    /// build state space
    pub fn build(&mut self, sampling_rate: f64) -> &mut Self {
        let tau = 1. / sampling_rate;
        let modes_2_nodes =
            na::DMatrix::from_row_slice(self.n_outputs(), self.n_modes(), &self.modes2outputs());
        //println!("modes 2 nodes: {:?}", modes_2_nodes.shape());
        let forces_2_modes =
            na::DMatrix::from_row_slice(self.n_modes(), self.n_inputs(), &self.inputs2modes());
        //println!("forces 2 modes: {:?}", forces_2_modes.shape());
        let w = self.eigen_frequencies_to_radians();
        let zeta = &self.proportional_damping_vec;
        /*
        (0..self.n_modes())
            .map(|k| {
                let b = forces_2_modes.row(k);
                let c = modes_2_nodes.column(k);
                StateSpace2x2::from_second_order(
                    DiscreteApproximation::BiLinear(tau),
                    w[k],
                    zeta[k],
                    Some(b.clone_owned().as_slice()),
                    Some(c.as_slice()),
                )
            })
            .collect()
        */
        self.state_space = Some(
            (0..self.n_modes())
                .map(|k| {
                    let b = forces_2_modes.row(k);
                    let c = modes_2_nodes.column(k);
                    Exponential::from_second_order(
                        tau,
                        w[k],
                        zeta[k],
                        b.clone_owned().as_slice().to_vec(),
                        c.as_slice().to_vec(),
                    )
                })
                .collect(),
        );
        self
    }
}
impl fmt::Display for FEM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ins = self
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(k, x)| x.as_ref().and_then(|x| Some((k, x))))
            .map(|(k, x)| format!(" #{:02} {}", k, x))
            .collect::<Vec<String>>()
            .join("\n");
        let outs = self
            .outputs
            .iter()
            .enumerate()
            .filter_map(|(k, x)| x.as_ref().and_then(|x| Some((k, x))))
            .map(|(k, x)| format!(" #{:02} {}", k, x))
            .collect::<Vec<String>>()
            .join("\n");
        write!(
            f,
            " INPUTS:\n{}\n{:>29}: [{:5}]\n OUTPUTS:\n{}\n{:>29}: [{:5}]",
            ins,
            "Total",
            self.n_inputs(),
            outs,
            "Total",
            self.n_outputs()
        )
    }
}
impl Iterator for FEM {
    type Item = Vec<f64>;
    fn next(&mut self) -> Option<Self::Item> {
        let n = self.n_outputs();
        match &self.u {
            Some(u) => self.state_space.as_mut().and_then(|ss| {
                Some(
                    ss.par_iter_mut()
                        .fold(
                            || vec![0f64; n],
                            |mut a: Vec<f64>, m| {
                                a.iter_mut().zip(m.solve(&u)).for_each(|(yc, y)| {
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
                        ),
                )
            }),
            None => None,
        }
    }
}

/*
impl fmt::Display for FEM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let a = format!(" - number of inputs: {}", self.inputs.n());
        let b = format!(" - number of outputs: {}", self.outputs.n());
        let c = format!(" - number of modes: {}", self.n_modes());
        let d = format!(
            " - eigen frequencies range: [{:.3},{:.3}]",
            self.eigen_frequencies.first().unwrap(),
            self.eigen_frequencies.last().unwrap()
        );
        let e = format!(
            " - proportional damping: {:6}",
            self.proportional_damping_vec.first().unwrap()
        );
        write!(
            f,
            "FEM:\n{}\n{}\n{}\n{}\n{}\n - inputs{:#?}\n - outputs{:#?}",
            a,
            b,
            c,
            d,
            e,
            self.inputs
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v.len()))
                .collect::<Vec<String>>(),
            self.outputs
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v.len()))
                .collect::<Vec<String>>()
        )
    }
}
*/
