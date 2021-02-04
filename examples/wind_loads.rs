use fem::{fem_io, FEM};
use rayon::prelude::*;
use serde::Deserialize;
use serde_pickle as pkl;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;

#[derive(Deserialize)]
struct WindLoads {
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
        println!("Wind load #{}",wind.n_sample);
        Ok(wind)
    }
    pub fn dispatch(&mut self, fem: &fem_io::Inputs) -> Option<&[f64]> {
        match fem {
            fem_io::Inputs::OSSTopEnd6F(_) => {
                if let Some(v) = &self.oss_topend_6f {
                    Some(v[self.count].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSTruss6F(_) => {
                if let Some(v) = &self.oss_truss_6f {
                    Some(v[self.count].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSGIR6F(_) => {
                if let Some(v) = &self.oss_gir_6f {
                    Some(v[self.count].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSCRING6F(_) => {
                if let Some(v) = &self.oss_cring_6f {
                    Some(v[self.count].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSCellLcl6F(_) => {
                if let Some(v) = &self.oss_cell_lcl_6f {
                    Some(v[self.count].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::OSSM1Lcl6F(_) => {
                if let Some(v) = &self.oss_m1_lcl_6f {
                    Some(v[self.count].as_slice())
                } else {
                    None
                }
            }
            fem_io::Inputs::MCM2LclForce6F(_) => {
                if let Some(v) = &self.mc_m2_lcl_6f {
                    Some(v[self.count].as_slice())
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
        let c = self.count;
        if c == self.n_sample {
            None
        } else {
            self.count += 1;
            Some(c)
        }
    }
}
struct Timer {
    time: Instant,
}
impl Timer {
    pub fn tic() -> Self {
        Self {
            time: Instant::now(),
        }
    }
    pub fn toc(self) -> f64 {
        self.time.elapsed().as_secs_f64()
    }
    pub fn print_toc(self) {
        println!("... in {:3}s", self.toc());
    }
}

fn main() {
    let tic = Timer::tic();
    println!("Loading wind loads ...");
    let mut wind = WindLoads::from_pickle("examples/RefinedTelescope_80hz_from_start.pkl").unwrap();
    println!(
        "Time range: [{};{}]",
        wind.time.first().unwrap(),
        wind.time.last().unwrap()
    );
    tic.print_toc();

    let tic = Timer::tic();
    println!("Loading FEM ...");
    let mut fem = FEM::from_pkl("examples/modal_state_space_model_2ndOrder_v1.pkl").unwrap();
    tic.print_toc();
    println!("{}", fem);
    fem.keep_inputs(&[1, 2, 3, 4, 5, 6, 13])
        .keep_outputs(&[5, 24]);
    println!("{}", fem);

    let tic = Timer::tic();
    let sampling = 2000.0;
    println!("Building 2x2 state space models ...");
    let mut ss = fem.state_space(sampling);
    tic.print_toc();
    println!("# of state space models: {}", ss.len());

    let n = wind.oss_topend_6f.as_ref().unwrap().len();
    println!("# of steps: {}", n);
    let mut u = vec![0f64; 6];
    u[0] = 1.;
    println!("Running model ...");
    let tic = Timer::tic();

    let qin: Vec<&fem_io::Inputs> = fem.inputs.iter().filter_map(|x| x.as_ref()).collect();
    let y: Vec<Vec<f64>> = (0..n)
        .map(|_| {
            let u: Vec<_> = qin
                .iter()
                .flat_map(|x| wind.dispatch(x).unwrap().to_vec())
                .collect();
            wind.next().unwrap();
            ss.par_iter_mut()
                .fold(
                    || vec![0f64; fem.n_outputs()],
                    |mut a: Vec<f64>, m| {
                        a.iter_mut().zip(m.solve(&u)).for_each(|(yc, y)| {
                            *yc += y;
                        });
                        a
                    },
                )
                .reduce(
                    || vec![0f64; fem.n_outputs()],
                    |mut a: Vec<f64>, b: Vec<f64>| {
                        a.iter_mut().zip(b.iter()).for_each(|(a, b)| {
                            *a += *b;
                        });
                        a
                    },
                )
        })
        .collect();
    tic.print_toc();

    let mut f = File::create("examples/wind_loads_y.pkl").unwrap();
    pkl::to_writer(&mut f, &y, true).unwrap();
}
