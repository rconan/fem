//! This module defines a type for the representation of a set of decoupled second order ODE
//!
//! The structure [`SecondOrder`] contains the vectors of eigen coefficients and proportional damping coeff

use dosio::io::Tags;
use serde::{self, Deserialize};
use serde_pickle as pkl;
use std::{fmt, fs::File, io::BufReader, path::Path};

#[derive(Debug)]
pub enum SecondOrderError {
    FileNotFound(std::io::Error),
    PickleRead(serde_pickle::Error),
}
impl fmt::Display for SecondOrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileNotFound(e) => write!(f, "wind loads data file not found: {}", e),
            Self::PickleRead(e) => write!(f, "cannot read wind loads data file: {}", e),
        }
    }
}
impl From<std::io::Error> for SecondOrderError {
    fn from(e: std::io::Error) -> Self {
        Self::FileNotFound(e)
    }
}
impl From<serde_pickle::Error> for SecondOrderError {
    fn from(e: serde_pickle::Error) -> Self {
        Self::PickleRead(e)
    }
}
impl std::error::Error for SecondOrderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FileNotFound(source) => Some(source),
            Self::PickleRead(source) => Some(source),
        }
    }
}

#[derive(Deserialize)]
pub struct SecondOrderIO {
    pub name: Vec<Tags>,
    pub size: Vec<usize>,
}
#[derive(Deserialize)]
pub struct SecondOrder {
    pub u: SecondOrderIO,
    pub y: SecondOrderIO,
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    #[serde(rename = "omega [Hz]")]
    pub omega: Vec<f64>,
    pub zeta: Vec<f64>,
}

impl SecondOrder {
    pub fn from_pickle<P: AsRef<Path>>(path: P) -> Result<Self, SecondOrderError> {
        let f = File::open(path)?;
        let r = BufReader::with_capacity(1_000_000, f);
        let v: serde_pickle::Value = serde_pickle::from_reader(r)?;
        Ok(pkl::from_value(v)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_second_order() {
        let snd_ord = SecondOrder::from_pickle("/media/rconan/FEM/20210614_2105_ASM_topendOnly/modal_state_space_model_2ndOrder_1500Hz_noRes_postproc.pkl").unwrap();
        println!("U names: {:#?}", snd_ord.u.name);
    }
}
