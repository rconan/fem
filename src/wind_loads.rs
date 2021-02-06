use super::fem_io;
use anyhow::{Context, Result};
use serde;
use serde::Deserialize;
use serde_pickle as pkl;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

macro_rules! outputs {
    ($($name:expr, $variant:ident),+) => {
        #[derive(Deserialize, Debug)]
        pub enum Outputs {
            $(#[serde(rename = $name)]
              $variant(Vec<Vec<f64>>)),+
        }
        impl Outputs {
            pub fn len(&self) -> usize {
                match self {
                    $(Outputs::$variant(io) => io.len()),+
                }
            }
            pub fn io(&self) -> &Vec<Vec<f64>> {
                match self {
                    $(Outputs::$variant(io) => io),+
                }
            }
            pub fn match_fem(&self, fem: &fem_io::Inputs, count: usize) -> Option<&[f64]> {
                match (fem,self) {
                    $((fem_io::Inputs::$variant(_),Outputs::$variant(v)) => {
                        Some(v[count].as_slice())
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

#[derive(Deserialize)]
pub struct WindLoads {
    pub outputs: Vec<Option<Outputs>>,
    pub time: Vec<f64>,
    #[serde(default)]
    count: usize,
    #[serde(skip)]
    pub n_sample: usize,
}
impl WindLoads {
    pub fn from_pickle<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path> + fmt::Display + Copy,
    {
        let f = File::open(path)?;
        let r = BufReader::with_capacity(1_000_000_000, f);
        let v: serde_pickle::Value =
            serde_pickle::from_reader(r).context(format!("Cannot read {}", path))?;
        let mut wind: Self = pkl::from_value(v).context(format!("Failed to load {}", path))?;
        //       let mut wind: WindLoads = pkl::from_reader(r)?;
        wind.n_sample = wind
            .outputs
            .iter()
            .filter_map(|x| x.as_ref())
            .next()
            .map_or(0, |x| x.len());
        Ok(wind)
    }
    pub fn dispatch(&mut self, fem: &fem_io::Inputs) -> Option<&[f64]> {
        self.outputs
            .iter()
            .filter_map(|x| x.as_ref().and_then(|x| x.match_fem(&fem, self.count - 1)))
            .next()
    }
}
impl Iterator for WindLoads {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == self.n_sample {
            None
        } else {
            self.count += 1;
            Some(self.count)
        }
    }
}
