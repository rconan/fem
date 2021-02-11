use super::{fem_io, Pairing};
use anyhow::{Context, Result};
use serde;
use serde::Deserialize;
use serde_pickle as pkl;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

macro_rules! loads {
    ($($name:expr, $variant:ident),+) => {
        #[derive(Deserialize, Debug,Clone)]
        pub enum Loads {
            $(#[serde(rename = $name)]
              $variant(Vec<Vec<f64>>)),+
        }
        impl Loads {
            pub fn len(&self) -> usize {
                match self {
                    $(Loads::$variant(io) => io.len()),+
                }
            }
            pub fn io(&self) -> &Vec<Vec<f64>> {
                match self {
                    $(Loads::$variant(io) => io),+
                }
            }
            pub fn as_output(self) -> Box<dyn Pairing<fem_io::Inputs,Vec<f64>>> {
                match self {
                    $(Loads::$variant(io) => Box::new(Outputs::$variant(io.into_iter()))),+
                }
            }
            pub fn as_n_output(self, n: usize) -> Box<dyn Pairing<fem_io::Inputs,Vec<f64>>> {
                match self {
                    $(Loads::$variant(io) => Box::new(Outputs::$variant(io[..n].to_owned().into_iter()))),+
                }
            }
            pub fn match_io(&self, fem: &fem_io::Inputs, count: usize) -> Option<&[f64]> {
                match (fem,self) {
                    $((fem_io::Inputs::$variant(_),Loads::$variant(v)) => {
                        Some(v[count].as_slice())
                    }),+
                    _ => None
                }
            }
        }
    };
}
macro_rules! outputs {
    ($($name:expr, $variant:ident),+) => {
        pub enum Outputs {
            $($variant(std::vec::IntoIter<Vec<f64>>)),+
        }
        impl Outputs {
            pub fn len(&self) -> usize {
                match self {
                    $(Outputs::$variant(io) => io.len()),+
                }
            }
            pub fn match_io(&mut self, fem: &fem_io::Inputs) -> Option<Vec<f64>> {
                match (fem,self) {
                    $((fem_io::Inputs::$variant(_),Outputs::$variant(v)) => {
                        v.next()
                    }),+
                        _ => None
                }
            }
        }
        impl Pairing<fem_io::Inputs,Vec<f64>> for Outputs {
            fn pair(&mut self, fem: &fem_io::Inputs) -> Option<Vec<f64>> {
                match (fem,self) {
                    $((fem_io::Inputs::$variant(_),Outputs::$variant(v)) => {
                        v.next()
                    }),+
                        _ => None
                }
            }
        }
    };
}

outputs!(
    "OSS_TopEnd_6F",
    OSSTopEnd6F,
    "OSS_Truss_6F",
    OSSTruss6F,
    "OSS_GIR_6F",
    OSSGIR6F,
    "OSS_CRING_6F",
    OSSCRING6F,
    "OSS_Cell_lcl_6F",
    OSSCellLcl6F,
    "OSS_M1_lcl_6F",
    OSSM1Lcl6F,
    "MC_M2_lcl_force_6F",
    MCM2Lcl6F
);
loads!(
    "OSS_TopEnd_6F",
    OSSTopEnd6F,
    "OSS_Truss_6F",
    OSSTruss6F,
    "OSS_GIR_6F",
    OSSGIR6F,
    "OSS_CRING_6F",
    OSSCRING6F,
    "OSS_Cell_lcl_6F",
    OSSCellLcl6F,
    "OSS_M1_lcl_6F",
    OSSM1Lcl6F,
    "MC_M2_lcl_force_6F",
    MCM2Lcl6F
);

#[derive(Deserialize)]
pub struct WindLoads {
    #[serde(rename = "outputs")]
    pub loads: Vec<Option<Loads>>,
    pub time: Vec<f64>,
    #[serde(skip)]
    pub n_sample: Option<usize>,
}
pub struct WindLoadsIter {
    pub outputs: Vec<Box<dyn Pairing<fem_io::Inputs, Vec<f64>>>>,
    pub n_sample: usize,
}

impl WindLoads {
    pub fn from_pickle<P>(path: P) -> Result<WindLoads>
    where
        P: AsRef<Path> + fmt::Display + Copy,
    {
        let f = File::open(path)?;
        let r = BufReader::with_capacity(1_000_000_000, f);
        let v: serde_pickle::Value =
            serde_pickle::from_reader(r).context(format!("Cannot read {}", path))?;
        pkl::from_value(v).context(format!("Failed to load {}", path))
    }
    pub fn n_sample(self, n_sample: usize) -> Self {
        Self {
            n_sample: Some(n_sample),
            ..self
        }
    }
    pub fn as_outputs(self) -> WindLoadsIter {
        match &self.n_sample {
            Some(n) => WindLoadsIter {
                outputs: self
                    .loads
                    .iter()
                    .filter_map(|x| x.as_ref())
                    .cloned()
                    .map(|x| x.as_n_output(*n))
                    .collect(),
                n_sample: *n,
            },
            None => WindLoadsIter {
                outputs: self
                    .loads
                    .iter()
                    .filter_map(|x| x.as_ref())
                    .cloned()
                    .map(|x| x.as_output())
                    .collect(),
                n_sample: self
                    .loads
                    .iter()
                    .filter_map(|x| x.as_ref())
                    .next()
                    .map_or(0, |x| x.len()),
            },
        }
    }
}
