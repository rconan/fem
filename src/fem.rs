use apache_arrow::{
    array::{Float64Array, LargeStringArray, StringArray},
    datatypes::SchemaRef,
    record_batch::{RecordBatch, RecordBatchReader},
};
use bytes::Bytes;
use matio_rs::{MatFile, MatioError};
use nalgebra as na;
use parquet::{arrow::arrow_reader::ParquetRecordBatchReaderBuilder, errors::ParquetError};
use std::{
    collections::HashMap,
    env, fmt,
    fs::File,
    io::{BufReader, Read, Write},
    path::Path,
};
use zip::{read::ZipFile, result::ZipError, ZipArchive};

pub mod fem_io;
pub mod io;
use io::{IOData, Properties, IO};

#[derive(Debug, thiserror::Error)]
pub enum FemError {
    #[error("FEM data file not found")]
    FileNotFound(#[from] std::io::Error),
    #[error("cannot read wind loads data file")]
    PickleRead(#[from] serde_pickle::Error),
    #[error("environment variable is not set")]
    EnvVar(#[from] env::VarError),
    #[error("static gain not found")]
    StaticGain,
    #[error("failed to convert {0}")]
    Convert(String),
    #[error("failed to read Parquet file")]
    Parquet(#[from] ParquetError),
    #[error("failed to read zip archive")]
    ZipReader(#[from] ZipError),
    #[error("failed to load Matlab file")]
    Matlab(#[from] MatioError),
    #[error("failed to read table column {0}")]
    ReadTableColumn(String),
    #[error("failed to find {0} in zip archive {1}")]
    ZipNotFound(String, String),
}

pub type Result<T> = std::result::Result<T, FemError>;

/* mpl fmt::Display for FEMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound(e) => write!(f, "FEM data file not found: {}", e),
            Self::PickleRead(e) => write!(f, "cannot read wind loads data file: {}", e),
            Self::EnvVar(e) => write!(f, "environment variable {} is not set", e),
            Self::StaticGain => write!(f, "Static gain not found"),
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
            Self::StaticGain => None,
        }
    }
} */

fn read<'a, T>(schema: &SchemaRef, table: &'a RecordBatch, col: &'a str) -> Result<&'a T>
where
    T: 'static,
{
    let Ok(idx) = schema.index_of(col) else {
        panic!(r#"No "csLabel" in table!"#);
    };
    table
        .column(idx)
        .as_any()
        .downcast_ref::<T>()
        .ok_or(FemError::ReadTableColumn(col.to_string()))
}

fn read_table(contents: Vec<u8>) -> Result<Vec<(String, Vec<IO>)>> {
    let parquet_reader = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(contents))?
        .with_batch_size(2048)
        .build()?;
    let schema = parquet_reader.schema();
    let mut io_map: HashMap<String, Vec<IO>> = HashMap::new();
    for maybe_table in parquet_reader {
        let Ok(table) = maybe_table else {
            panic!("Not a table!");
        };
        read::<StringArray>(&schema, &table, "csLabel")?
            .iter()
            .zip(read::<Float64Array>(&schema, &table, "index")?.iter())
            .zip(read::<Float64Array>(&schema, &table, "X")?.iter())
            .zip(read::<Float64Array>(&schema, &table, "Y")?.iter())
            .zip(read::<Float64Array>(&schema, &table, "Z")?.iter())
            .zip(read::<StringArray>(&schema, &table, "description")?.iter())
            .zip(read::<StringArray>(&schema, &table, "group")?.iter())
            .filter_map(|data| {
                if let ((((((Some(g), Some(f)), Some(e)), Some(d)), Some(c)), Some(b)), Some(a)) =
                    data
                {
                    Some((g, f, e, d, c, b, a))
                } else {
                    None
                }
            })
            .for_each(|(cs_label, index, x, y, z, description, group)| {
                let value = IO::On(IOData {
                    indices: vec![index as u32],
                    descriptions: description.to_string(),
                    properties: Properties {
                        cs_label: Some(cs_label.to_string()),
                        location: Some(vec![x, y, z]),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                io_map
                    .entry(group.to_string())
                    .or_insert(vec![])
                    .push(value)
            });
    }
    let mut sorted_map: Vec<_> = io_map.into_iter().collect();
    sorted_map.sort_by_key(|a| a.0.to_string());
    Ok(sorted_map)
}

fn read_table2(contents: Vec<u8>) -> Result<Vec<(String, Vec<IO>)>> {
    let parquet_reader = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(contents))?
        .with_batch_size(2048)
        .build()?;
    let schema = parquet_reader.schema();
    let mut io_map: HashMap<String, Vec<IO>> = HashMap::new();
    for maybe_table in parquet_reader {
        let Ok(table) = maybe_table else {
            panic!("Not a table!");
        };
        read::<LargeStringArray>(&schema, &table, "csLabel")?
            .iter()
            .zip(read::<Float64Array>(&schema, &table, "index")?.iter())
            .zip(read::<Float64Array>(&schema, &table, "X")?.iter())
            .zip(read::<Float64Array>(&schema, &table, "Y")?.iter())
            .zip(read::<Float64Array>(&schema, &table, "Z")?.iter())
            .zip(read::<LargeStringArray>(&schema, &table, "description")?.iter())
            .zip(read::<LargeStringArray>(&schema, &table, "group")?.iter())
            .filter_map(|data| {
                if let ((((((Some(g), Some(f)), Some(e)), Some(d)), Some(c)), Some(b)), Some(a)) =
                    data
                {
                    Some((g, f, e, d, c, b, a))
                } else {
                    None
                }
            })
            .for_each(|(cs_label, index, x, y, z, description, group)| {
                let value = IO::On(IOData {
                    indices: vec![index as u32],
                    descriptions: description.to_string(),
                    properties: Properties {
                        cs_label: Some(cs_label.to_string()),
                        location: Some(vec![x, y, z]),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                io_map
                    .entry(group.to_string())
                    .or_insert(vec![])
                    .push(value)
            });
    }
    let mut sorted_map: Vec<_> = io_map.into_iter().collect();
    sorted_map.sort_by_key(|a| a.0.to_string());
    Ok(sorted_map)
}

fn read_contents(mut zip_file: ZipFile) -> Result<Vec<u8>> {
    let mut contents: Vec<u8> = Vec::new();
    zip_file.read_to_end(&mut contents)?;
    Ok(contents)
}

fn read_mat(zip_file: &mut ZipArchive<BufReader<File>>, name: &str) -> Result<Vec<f64>> {
    let mat_file_name = format!("rust/{}.mat", name);
    let mut i = 1;
    let mut maybe_data = None;
    while let Ok(mat_file) = zip_file.by_name(&format!("{}/slice_{}.mat", mat_file_name, i)) {
        log::info!(
            r#"loading {} matrix slice #{} from "{}""#,
            name,
            i,
            mat_file_name
        );
        let contents = read_contents(mat_file)?;
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(contents.as_slice())?;
        file.flush()?;
        let mut data: Vec<f64> = MatFile::load(file.path())?.var(format!("slice"))?;
        maybe_data.get_or_insert(vec![]).append(&mut data);
        i += 1;
    }
    let data = if let Some(contents) = maybe_data {
        contents
    } else {
        let mat_file = zip_file.by_name(&mat_file_name)?;
        log::info!(r#"loading {} from "{}""#, name, mat_file_name);
        let contents = read_contents(mat_file)?;
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(contents.as_slice())?;
        file.flush()?;
        let data = MatFile::load(file.path())?.var(name)?;
        data
    };
    Ok(data)
}

fn read_inputs(zip_file: &mut ZipArchive<BufReader<File>>) -> Result<Vec<Option<fem_io::Inputs>>> {
    log::info!(r#"reading inputs table from "modal_state_space_model_2ndOrder_in.parquet""#);
    read_contents(zip_file.by_name("rust/modal_state_space_model_2ndOrder_in.parquet")?)
        .and_then(|contents| read_table(contents.clone()).or_else(|_| read_table2(contents)))?
        .into_iter()
        .map(|item| Some(fem_io::Inputs::try_from(item)).transpose())
        .collect()
}

fn read_outputs(
    zip_file: &mut ZipArchive<BufReader<File>>,
) -> Result<Vec<Option<fem_io::Outputs>>> {
    log::info!(r#"reading outputs table from "modal_state_space_model_2ndOrder_out.parquet""#);
    read_contents(zip_file.by_name("rust/modal_state_space_model_2ndOrder_out.parquet")?)
        .and_then(|contents| read_table(contents.clone()).or_else(|_| read_table2(contents)))?
        .into_iter()
        .map(|item| Some(fem_io::Outputs::try_from(item)).transpose())
        .collect()
}

/// GMT Finite Element Model
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct FEM {
    /// Model info
    #[cfg_attr(feature = "serde", serde(rename = "modelDescription"))]
    pub model_description: String,
    /// inputs properties
    pub inputs: Vec<Option<fem_io::Inputs>>,
    /// outputs properties
    pub outputs: Vec<Option<fem_io::Outputs>>,
    /// mode shapes eigen frequencies `[Hz]`
    #[cfg_attr(feature = "serde", serde(rename = "eigenfrequencies"))]
    pub eigen_frequencies: Vec<f64>,
    /// inputs forces to modal forces matrix `[n_modes,n_inputs]` (row wise)
    #[cfg_attr(feature = "serde", serde(rename = "inputs2ModalF"))]
    pub inputs_to_modal_forces: Vec<f64>,
    /// mode shapes to outputs nodes `[n_outputs,n_modes]` (row wise)
    #[cfg_attr(feature = "serde", serde(rename = "modalDisp2Outputs"))]
    pub modal_disp_to_outputs: Vec<f64>,
    /// mode shapes damping coefficients
    #[cfg_attr(feature = "serde", serde(rename = "proportionalDampingVec"))]
    pub proportional_damping_vec: Vec<f64>,
    #[cfg_attr(feature = "serde", serde(rename = "gainMatrix"))]
    pub static_gain: Option<Vec<f64>>,
    /// number of inputs and outputs before any model reduction
    #[cfg_attr(feature = "serde", serde(skip))]
    pub n_io: (usize, usize),
    #[cfg_attr(feature = "serde", serde(skip))]
    model: String,
}
impl FEM {
    /// Loads a FEM model, saved in a second order form, from a pickle file
    ///
    #[cfg(feature = "serde")]
    pub fn from_pickle<P: AsRef<Path>>(path: P) -> Result<FEM> {
        println!("Loading FEM from {:?}", path.as_ref());
        let file = File::open(&path)?;
        let v: serde_pickle::Value = serde_pickle::from_reader(file)?;
        let mut fem: FEM = serde_pickle::from_value(v)?;
        fem.n_io = (fem.n_inputs(), fem.n_outputs());
        fem.model = path.as_ref().to_str().unwrap().to_string();
        Ok(fem)
    }
    /// Loads a FEM model, saved in a second order form, from a zip archive file
    pub fn from_zip_archive<P: AsRef<Path>>(path: P) -> Result<FEM> {
        let path = path.as_ref();
        log::info!("Loading FEM from {path:?}");
        let file = File::open(path)?;
        let buffer = BufReader::new(file);
        let mut zip_file = zip::ZipArchive::new(buffer)?;

        let inputs = read_inputs(&mut zip_file)?;
        let outputs = read_outputs(&mut zip_file)?;
        let n_io = (
            inputs
                .iter()
                .filter_map(|x| x.as_ref())
                .fold(0usize, |a, x| a + x.len()),
            outputs
                .iter()
                .filter_map(|x| x.as_ref())
                .fold(0usize, |a, x| a + x.len()),
        );

        let inputs_to_modal_forces: Vec<f64> = read_mat(&mut zip_file, "inputs2ModalF")?;

        let modal_disp_to_outputs: Vec<f64> = read_mat(&mut zip_file, "modalDisp2Outputs")?;

        let static_gain = read_mat(&mut zip_file, "static_gain").ok();

        log::info!(r#"loading FEM properties from "modal_state_space_model_2ndOrder_mat.mat""#);
        let mat_file = zip_file.by_name("rust/modal_state_space_model_2ndOrder_mat.mat")?;
        let contents = read_contents(mat_file)?;
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(contents.as_slice())?;
        file.flush()?;
        let mat_file = MatFile::load(file.path())?;

        Ok(FEM {
            inputs,
            outputs,
            // model_description: mat_file.var("modelDescription")?,
            eigen_frequencies: mat_file.var("eigenfrequencies")?,
            inputs_to_modal_forces,
            modal_disp_to_outputs,
            proportional_damping_vec: mat_file.var("proportionalDampingVec")?,
            static_gain,
            n_io,
            model: path.to_str().unwrap().to_string(),
            ..Default::default()
        })
    }
    /// Loads a FEM model, saved in a second order form, from a zip archive file located in a directory given by the `FEM_REPO` environment variable
    ///
    /// The name of the zip file must be `"modal_state_space_model_2ndOrder.zip`
    pub fn from_env() -> Result<Self> {
        let fem_repo = env::var("FEM_REPO")?;
        let path = Path::new(&fem_repo);
        Self::from_zip_archive(path.join("modal_state_space_model_2ndOrder.zip"))
        // .or_else(|_| Self::from_pickle(&path.join("modal_state_space_model_2ndOrder.73.pkl")))
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

    /// Loads FEM static solution gain matrix
    ///
    /// The gain is loaded from a pickle file "static_reduction_model.73.pkl" located in a directory given by either the `FEM_REPO` or the `STATIC_FEM_REPO` environment variable, `STATIC_FEM_REPO` is tried first and if it failed then `FEM_REPO` is checked
    #[cfg(feature = "serde")]
    pub fn static_from_env(self) -> Result<Self> {
        let fem_repo = env::var("STATIC_FEM_REPO").or(env::var("FEM_REPO"))?;
        let path = Path::new(&fem_repo).join("static_reduction_model.73.pkl");
        // println!("Loading static gain matrix from {path:?}");
        let fem_static = Self::from_pickle(path)?;
        let static_gain = fem_static.static_gain.ok_or(FemError::StaticGain)?;
        assert_eq!(
            static_gain.len(),
            self.n_inputs() * self.n_outputs(),
            "Static gain dimensions do not mach the dynamic FEM."
        );
        Ok(Self {
            static_gain: Some(static_gain),
            ..self
        })
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
    /// Filters the inputs according to some properties matching
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
    /// Removes the inputs which properties do not match the predicate
    pub fn remove_inputs_by<F>(&mut self, id: &[usize], pred: F) -> &mut Self
    where
        F: Fn(&IOData) -> bool + Copy,
    {
        self.inputs.iter_mut().enumerate().for_each(|(k, i)| {
            if id.contains(&k) {
                if let Some(i) = i.as_mut() {
                    let io: Vec<_> = i
                        .iter()
                        .filter(|io| {
                            pred(match io {
                                IO::On(data) => data,
                                IO::Off(data) => data,
                            })
                        })
                        .cloned()
                        .collect();
                    i.set(io);
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
        /*         assert_eq!(
            matrix.nrows(),
            self.n_outputs(),
            "Matrix rows # do not match outputs #"
        ); */
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

    /// Return the static gain reduced to the turned-on inputs and outputs
    pub fn reduced_static_gain(&mut self) -> Option<na::DMatrix<f64>> {
        log::info!("computing static gain");
        let n_io = self.n_io;
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
        log::info!("computing DC dynamic gain");
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
        writeln!(f, "GMT FEM ({})", self.model)?;
        writeln!(
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
