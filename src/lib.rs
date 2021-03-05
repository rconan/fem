use rayon::prelude::*;
use serde_pickle as pkl;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub mod io;
pub use io::{IOData, IO};

pub mod fem;
pub use fem::{fem_io, FEM};

pub mod state_space;
pub use state_space::{DiscreteApproximation, SerdeStateSpace2x2, StateSpace2x2};

pub mod bilinear;
pub use bilinear::Bilinear;
pub mod exponential;
pub use exponential::Exponential;

#[derive(Debug, Default)]
pub struct DiscreteModalSolver {
    pub u: Vec<f64>,
    pub y: Vec<f64>,
    pub state_space: Vec<Exponential>,
}
impl Iterator for DiscreteModalSolver {
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

pub fn load_io<P: AsRef<Path>>(path: P) -> Result<BTreeMap<String, Vec<IO>>, Box<dyn Error>> {
    let f = File::open(path)?;
    let r = BufReader::with_capacity(1_000_000, f);
    Ok(pkl::from_reader(r)?)
}
pub trait ToPickle {
    fn to_pickle<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>>;
}
impl ToPickle for Vec<f64> {
    fn to_pickle<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let mut f = File::create(path)?;
        pkl::to_writer(&mut f, &self, true)?;
        Ok(())
    }
}
impl ToPickle for SerdeStateSpace2x2 {
    fn to_pickle<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let mut f = File::create(path)?;
        pkl::to_writer(&mut f, &self, true)?;
        Ok(())
    }
}
impl ToPickle for Bilinear {
    fn to_pickle<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let mut f = File::create(path)?;
        pkl::to_writer(&mut f, &self, true)?;
        Ok(())
    }
}

