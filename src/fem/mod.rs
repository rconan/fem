use crate::io::{IOData, IO};
use nalgebra as na;
use serde;
use serde::Deserialize;
use serde_pickle as pkl;
use std::{env, fmt, fs::File, io::BufReader, path::Path};

pub mod fem_io;

pub enum FEMError {
    FileNotFound(std::io::Error),
    PickleRead(serde_pickle::Error),
    EnvVar(env::VarError),
}
impl fmt::Display for FEMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound(e) => write!(f, "FEM data file not found: {}", e),
            Self::PickleRead(e) => write!(f, "cannot read wind loads data file: {}", e),
            Self::EnvVar(e) => write!(f, "environment variable {} is not set", e),
        }
    }
}
impl fmt::Debug for FEMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <FEMError as std::fmt::Display>::fmt(self, f)
    }
}
impl From<std::io::Error> for FEMError {
    fn from(e: std::io::Error) -> Self {
        Self::FileNotFound(e)
    }
}
impl From<serde_pickle::Error> for FEMError {
    fn from(e: serde_pickle::Error) -> Self {
        Self::PickleRead(e)
    }
}
impl From<env::VarError> for FEMError {
    fn from(e: env::VarError) -> Self {
        Self::EnvVar(e)
    }
}
impl std::error::Error for FEMError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FileNotFound(source) => Some(source),
            Self::PickleRead(source) => Some(source),
            Self::EnvVar(source) => Some(source),
        }
    }
}

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
    /// mode shapes eigen frequencies `[Hz]`
    #[serde(rename = "eigenfrequencies")]
    pub eigen_frequencies: Vec<f64>,
    /// inputs forces to modal forces matrix `[n_modes,n_inputs]` (row wise)
    #[serde(rename = "inputs2ModalF")]
    pub inputs_to_modal_forces: Vec<f64>,
    /// mode shapes to outputs nodes `[n_outputs,n_modes]` (row wise)
    #[serde(rename = "modalDisp2Outputs")]
    pub modal_disp_to_outputs: Vec<f64>,
    /// mode shapes damping coefficients
    #[serde(rename = "proportionalDampingVec")]
    pub proportional_damping_vec: Vec<f64>,
    #[serde(rename = "gainMatrix")]
    pub static_gain: Option<Vec<f64>>,
}
impl FEM {
    /// Loads a FEM model saved in a second order from a pickle file
    pub fn from_pickle<P: AsRef<Path>>(path: P) -> Result<FEM, FEMError> {
        let f = File::open(path)?;
        let r = BufReader::with_capacity(1_000_000, f);
        let v: serde_pickle::Value = serde_pickle::from_reader(r)?;
        Ok(pkl::from_value(v)?)
    }
    /// Loads a FEM model saved in a second order from a pickle file "modal_state_space_model_2ndOrder.73.pkl" located in a directory given by the `FEM)REPO` environment variable
    pub fn from_env() -> Result<Self, FEMError> {
        let fem_repo = env::var("FEM_REPO")?;
        let path = Path::new(&fem_repo).join("modal_state_space_model_2ndOrder.73.pkl");
        println!("Loading FEM from {path:?}");
        Self::from_pickle(path)
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
    /// Gets the number of inputs
    pub fn n_inputs(&self) -> usize {
        self.inputs
            .iter()
            .filter_map(|x| x.as_ref())
            .fold(0usize, |a, x| a + x.len())
    }
    /// Gets the number of outputs
    pub fn n_outputs(&self) -> usize {
        self.outputs
            .iter()
            .filter_map(|x| x.as_ref())
            .fold(0usize, |a, x| a + x.len())
    }

    /// Loads FEM static solution gain matrix from a pickle file "static_reduction_model.73.pkl" located in a directory given by the `FEM_REPO` environment variable
    pub fn static_from_env(self) -> Self {
        let fem_repo = env::var("FEM_REPO").unwrap();
        println!("Loading static gain matrix from static_reduction_model.73.pkl...");
        let fem_static =
            Self::from_pickle(Path::new(&fem_repo).join("static_reduction_model.73.pkl")).unwrap();
        Self {
            static_gain: fem_static.static_gain,
            ..self
        }
    }

    /// Selects the inputs according to their natural ordering
    pub fn keep_inputs(&mut self, id: &[usize]) -> &mut Self {
        self.inputs.iter_mut().enumerate().for_each(|(k, i)| {
            if !id.contains(&k) {
                *i = None
            }
        });
        self
    }
    /// Selects the inputs according to their natural ordering and some properties matching
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
    /// Selects the outputs according to their natural ordering
    pub fn keep_outputs(&mut self, id: &[usize]) -> &mut Self {
        self.outputs.iter_mut().enumerate().for_each(|(k, i)| {
            if !id.contains(&k) {
                *i = None
            }
        });
        self
    }
    /// Selects the outputs according to their natural ordering and some properties matching
    pub fn keep_outputs_by<F>(&mut self, id: &[usize], pred: F) -> &mut Self
    where
        F: Fn(&IOData) -> bool + Copy,
    {
        self.outputs.iter_mut().enumerate().for_each(|(k, i)| {
            if !id.contains(&k) {
                *i = None
            } else {
                if let Some(i) = i.as_mut() {
                    i.iter_mut().for_each(|io| {
                        *io = io.clone().switch_off();
                        *io = io.clone().switch_on_by(pred);
                    })
                }
            }
        });
        self
    }
    /// Filters the outputs according to some properties matching
    pub fn filter_inputs_by<F>(&mut self, id: &[usize], pred: F) -> &mut Self
    where
        F: Fn(&IOData) -> bool + Copy,
    {
        self.inputs.iter_mut().enumerate().for_each(|(k, i)| {
            if id.contains(&k) {
                if let Some(i) = i.as_mut() {
                    i.iter_mut().for_each(|io| {
                        *io = io.clone().switch_off();
                        *io = io.clone().switch_on_by(pred);
                    })
                }
            }
        });
        self
    }
    /// Filters the outputs according to some properties matching
    pub fn filter_outputs_by<F>(&mut self, id: &[usize], pred: F) -> &mut Self
    where
        F: Fn(&IOData) -> bool + Copy,
    {
        self.outputs.iter_mut().enumerate().for_each(|(k, i)| {
            if id.contains(&k) {
                if let Some(i) = i.as_mut() {
                    i.iter_mut().for_each(|io| {
                        *io = io.clone().switch_off();
                        *io = io.clone().switch_on_by(pred);
                    })
                }
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
    /// Returns the inputs 2 modes transformation matrix for a given input
    pub fn input2modes(&self, id: usize) -> Option<Vec<f64>> {
        self.inputs[id].as_ref().map(|input| {
            let indices: Vec<u32> = input
                .iter()
                .filter_map(|x| match x {
                    IO::On(io) => Some(io.indices.clone()),
                    IO::Off(_) => None,
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
        })
    }
    pub fn trim2input(&self, id: usize, matrix: &na::DMatrix<f64>) -> Option<na::DMatrix<f64>> {
        /*assert_eq!(
            matrix.ncols(),
            self.n_inputs(),
            "Matrix columns # do not match inputs #"
        );*/
        self.inputs[id].as_ref().map(|input| {
            let indices: Vec<u32> = input
                .iter()
                .filter_map(|x| match x {
                    IO::On(io) => Some(io.indices.clone()),
                    IO::Off(_) => None,
                })
                .flatten()
                .collect();
            na::DMatrix::from_columns(
                &indices
                    .iter()
                    .map(|&i| matrix.column(i as usize - 1))
                    .collect::<Vec<_>>(),
            )
        })
    }
    /// Returns the inputs 2 modes transformation matrix for an input type
    pub fn in2modes<U>(&self) -> Option<Vec<f64>>
    where
        Vec<Option<fem_io::Inputs>>: fem_io::FemIo<U>,
    {
        <Vec<Option<fem_io::Inputs>> as fem_io::FemIo<U>>::position(&self.inputs)
            .and_then(|id| self.input2modes(id))
    }
    pub fn trim2in<U>(&self, matrix: &na::DMatrix<f64>) -> Option<na::DMatrix<f64>>
    where
        Vec<Option<fem_io::Inputs>>: fem_io::FemIo<U>,
    {
        <Vec<Option<fem_io::Inputs>> as fem_io::FemIo<U>>::position(&self.inputs)
            .and_then(|id| self.trim2input(id, matrix))
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
    /// Returns the modes 2 outputs transformation matrix for a given output
    pub fn modes2output(&self, id: usize) -> Option<Vec<f64>> {
        let q: Vec<_> = self.modal_disp_to_outputs.chunks(self.n_modes()).collect();
        self.outputs[id].as_ref().map(|output| {
            output
                .iter()
                .filter_map(|x| match x {
                    IO::On(io) => Some(io.indices.clone()),
                    IO::Off(_) => None,
                })
                .flatten()
                .flat_map(|i| q[i as usize - 1])
                .cloned()
                .collect()
        })
    }
    pub fn trim2output(&self, id: usize, matrix: &na::DMatrix<f64>) -> Option<na::DMatrix<f64>> {
        assert_eq!(
            matrix.nrows(),
            self.n_outputs(),
            "Matrix rows # do not match outputs #"
        );
        //let q: Vec<_> = matrix.chunks(self.n_modes()).collect();
        self.outputs[id].as_ref().map(|output| {
            na::DMatrix::from_rows(
                &output
                    .iter()
                    .filter_map(|x| match x {
                        IO::On(io) => Some(io.indices.clone()),
                        IO::Off(_) => None,
                    })
                    .flatten()
                    .map(|i| matrix.row(i as usize - 1))
                    .collect::<Vec<_>>(),
            )
        })
    }
    /// Returns the modes 2 outputs transformation matrix for an output type
    pub fn modes2out<U>(&self) -> Option<Vec<f64>>
    where
        Vec<Option<fem_io::Outputs>>: fem_io::FemIo<U>,
    {
        <Vec<Option<fem_io::Outputs>> as fem_io::FemIo<U>>::position(&self.outputs)
            .and_then(|id| self.modes2output(id))
    }
    pub fn trim2out<U>(&self, matrix: &na::DMatrix<f64>) -> Option<na::DMatrix<f64>>
    where
        Vec<Option<fem_io::Outputs>>: fem_io::FemIo<U>,
    {
        <Vec<Option<fem_io::Outputs>> as fem_io::FemIo<U>>::position(&self.outputs)
            .and_then(|id| self.trim2output(id, matrix))
    }
    /// Return the static gain reduced to the turned-on inputs and outputs
    pub fn reduced_static_gain(&mut self, n_io: (usize, usize)) -> Option<na::DMatrix<f64>> {
        let n_reduced_io = (self.n_inputs(), self.n_outputs());
        self.static_gain
            .as_ref()
            .map(|gain| {
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
                let n = n_io.0;
                let reduced_inputs_gain: Vec<f64> = gain
                    .chunks(n)
                    .flat_map(|x| {
                        indices
                            .iter()
                            .map(|i| x[*i as usize - 1])
                            .collect::<Vec<f64>>()
                    })
                    .collect();
                let n = n_reduced_io.0;
                let q: Vec<_> = reduced_inputs_gain.chunks(n).collect();
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
                    .collect::<Vec<f64>>()
            })
            .map(|new_gain| na::DMatrix::from_row_slice(n_reduced_io.1, n_reduced_io.0, &new_gain))
    }
    /// Returns the FEM static gain for the turned-on inputs and outputs
    pub fn static_gain(&mut self) -> na::DMatrix<f64> {
        let forces_2_modes =
            na::DMatrix::from_row_slice(self.n_modes(), self.n_inputs(), &self.inputs2modes());
        let modes_2_nodes =
            na::DMatrix::from_row_slice(self.n_outputs(), self.n_modes(), &self.modes2outputs());
        let d = na::DMatrix::from_diagonal(
            &na::DVector::from_row_slice(&self.eigen_frequencies_to_radians())
                .map(|x| 1f64 / (x * x))
                .remove_rows(0, 3),
        );

        // println!("{ }",d.fixed_slice::<3,3>(0,0)); <- Just checking if unstable modes were removed
        modes_2_nodes.remove_columns(0, 3) * d * forces_2_modes.remove_rows(0, 3)
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
        if let Some(_) = &self.static_gain {
            write!(
                f,
                "INPUTS:\n{}\n{:>29}: [{:5}]\n OUTPUTS:\n{}\n{:>29}: [{:5}]",
                ins,
                "Total",
                self.n_inputs(),
                outs,
                "Total",
                self.n_outputs()
            )
        } else {
            let min_damping = self
                .proportional_damping_vec
                .iter()
                .cloned()
                .fold(std::f64::INFINITY, f64::min);
            let max_damping = self
                .proportional_damping_vec
                .iter()
                .cloned()
                .fold(std::f64::NEG_INFINITY, f64::max);
            write!(
            f,
            "  - # of modes: {}\n  - first 5 eigen frequencies: {:9.3?}\n  - last 5 eigen frequencies: {:9.3?}\n  - damping coefficients [min;max]: [{:.4};{:.4}] \nINPUTS:\n{}\n{:>29}: [{:5}]\n OUTPUTS:\n{}\n{:>29}: [{:5}]",
            self.n_modes(),
            &self.eigen_frequencies[..5],
            &self.eigen_frequencies[self.n_modes()-5..],
            min_damping, max_damping,
            ins,
            "Total",
            self.n_inputs(),
            outs,
            "Total",
            self.n_outputs()
        )
        }
    }
}
