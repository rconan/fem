use super::IOTraits;
use super::IO;
use super::{DiscreteApproximation, StateSpace2x2};
use anyhow::{Context, Result};
use nalgebra as na;
use serde::Deserialize;
use serde_pickle as pkl;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Finite Element Model
#[derive(Deserialize, Debug)]
pub struct FEM {
    /// Model info
    #[serde(rename = "modelDescription")]
    pub model_description: String,
    /// inputs properties
    pub inputs: BTreeMap<String, Vec<IO>>,
    /// outputs properties
    pub outputs: BTreeMap<String, Vec<IO>>,
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
}
impl FEM {
    /// Loads a FEM model saved in a second order from in a pickle file
    pub fn from_pkl<P>(path: P) -> Result<FEM>
    where
        P: AsRef<Path> + fmt::Display + Copy,
    {
        let f = File::open(path).context(format!("File {} not found", path))?;
        let r = BufReader::with_capacity(1_000_000, f);
        Ok(pkl::from_reader(r).context(format!("Failed to load {}", path))?)
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
    /// Returns the inputs 2 modes transformation matrix for the turned-on inputs
    pub fn inputs2modes(&mut self) -> Vec<f64> {
        let indices: Vec<u32> = self
            .inputs
            .values()
            .flat_map(|v| {
                v.iter().filter_map(|x| match x {
                    IO::On(io) => Some(io.indices.clone()),
                    IO::Off(_) => None,
                })
            })
            .flatten()
            .collect();
        println!("indices: {}", indices.len());
        let n = self.inputs.n();
        println!(
            "inputs_to_modal_forces: {}",
            self.inputs_to_modal_forces.len()
        );
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
            .values()
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
            na::DMatrix::from_row_slice(self.n_modes(), self.inputs.n_on(), &self.inputs2modes());
        let modes_2_nodes =
            na::DMatrix::from_row_slice(self.outputs.n_on(), self.n_modes(), &self.modes2outputs());
        let d = na::DMatrix::from_diagonal(
            &na::DVector::from_row_slice(&self.eigen_frequencies_to_radians())
                .map(|x| 1f64 / (x * x)),
        );
        modes_2_nodes * d * forces_2_modes
    }
    /// State space
    pub fn state_space(&mut self, sampling_rate: f64) -> Vec<StateSpace2x2> {
        let tau = 1. / sampling_rate;
        let modes_2_nodes =
            na::DMatrix::from_row_slice(self.outputs.n_on(), self.n_modes(), &self.modes2outputs());
        println!("modes 2 nodes: {:?}",modes_2_nodes.shape());
        let forces_2_modes =
            na::DMatrix::from_row_slice(self.n_modes(), self.inputs.n_on(), &self.inputs2modes());
        println!("forces 2 modes: {:?}",forces_2_modes.shape());
        let w = self.eigen_frequencies_to_radians();
        let zeta = &self.proportional_damping_vec;
        (0..self.n_modes())
            .map(|k| {
                let b = forces_2_modes.row(k);
                let c = modes_2_nodes.column(k);
                StateSpace2x2::from_second_order(
                    DiscreteApproximation::Exponential(tau),
                    w[k],
                    zeta[k],
                    Some(b.clone_owned().as_slice()),
                    Some(c.as_slice()),
                )
            })
            .collect()
    }
}
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
