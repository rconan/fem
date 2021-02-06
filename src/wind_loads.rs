use super::fem_io;
use serde;
use serde::Deserialize;
use serde_pickle as pkl;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Deserialize)]
pub struct WindLoads {
    #[serde(rename = "OSS_TopEnd_6F")]
    pub oss_topend_6f: Option<Vec<Vec<f64>>>,
    #[serde(rename = "OSS_Truss_6F")]
    pub oss_truss_6f: Option<Vec<Vec<f64>>>,
    #[serde(rename = "OSS_GIR_6F")]
    pub oss_gir_6f: Option<Vec<Vec<f64>>>,
    #[serde(rename = "OSS_CRING_6F")]
    pub oss_cring_6f: Option<Vec<Vec<f64>>>,
    #[serde(rename = "OSS_Cell_lcl_6F")]
    pub oss_cell_lcl_6f: Option<Vec<Vec<f64>>>,
    #[serde(rename = "OSS_M1_lcl_6F")]
    pub oss_m1_lcl_6f: Option<Vec<Vec<f64>>>,
    #[serde(rename = "MC_M2_lcl_force_6F")]
    pub mc_m2_lcl_6f: Option<Vec<Vec<f64>>>,
    pub time: Vec<f64>,
    #[serde(default)]
    count: usize,
    #[serde(skip)]
    pub n_sample: usize,
}
impl WindLoads {
    pub fn from_pickle<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let f = File::open(path)?;
        let r = BufReader::with_capacity(1_000_000_000, f);
        let mut wind: WindLoads = pkl::from_reader(r)?;
        wind.n_sample = wind
            .oss_topend_6f
            .as_ref()
            .or(wind
                .oss_truss_6f
                .as_ref()
                .or(wind
                    .oss_gir_6f
                    .as_ref()
                    .or(wind.oss_cring_6f.as_ref().or(wind
                        .oss_cell_lcl_6f
                        .as_ref()
                        .or(wind.oss_m1_lcl_6f.as_ref().or(wind.mc_m2_lcl_6f.as_ref()))))))
            .ok_or("No wind load found")?
            .len();
        Ok(wind)
    }
    pub fn dispatch(&mut self, fem: &fem_io::Inputs) -> Option<&[f64]> {
        match fem {
            fem_io::Inputs::OSSTopEnd6F(_) => {
                if let Some(v) = &self.oss_topend_6f {
                    Some(v[self.count - 1].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSTruss6F(_) => {
                if let Some(v) = &self.oss_truss_6f {
                    Some(v[self.count - 1].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSGIR6F(_) => {
                if let Some(v) = &self.oss_gir_6f {
                    Some(v[self.count - 1].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSCRING6F(_) => {
                if let Some(v) = &self.oss_cring_6f {
                    Some(v[self.count - 1].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSCellLcl6F(_) => {
                if let Some(v) = &self.oss_cell_lcl_6f {
                    Some(v[self.count - 1].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSM1Lcl6F(_) => {
                if let Some(v) = &self.oss_m1_lcl_6f {
                    Some(v[self.count - 1].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::MCM2LclForce6F(_) => {
                if let Some(v) = &self.mc_m2_lcl_6f {
                    Some(v[self.count - 1].as_slice())
                } else {
                    None
                }
            }
            _ => None,
        }
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
