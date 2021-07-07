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
impl SecondOrderIO {
    pub fn position(&self, tag: &Tags) -> Option<usize> {
        self.name.iter().position(|x| *x == *tag)
    }
}
impl From<(Vec<Tags>, Vec<usize>)> for SecondOrderIO {
    fn from((name, size): (Vec<Tags>, Vec<usize>)) -> Self {
        Self { name, size }
    }
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
    pub fn n_u(&self) -> usize {
        self.u.size.iter().sum()
    }
    pub fn n_y(&self) -> usize {
        self.y.size.iter().sum()
    }
    pub fn n_mode(&self) -> usize {
        self.zeta.len()
    }
    pub fn b_rows(&self) -> impl Iterator<Item = &[f64]> {
        self.b.chunks(self.n_u())
    }
    pub fn c_rows(&self) -> impl Iterator<Item = &[f64]> {
        self.c.chunks(self.n_mode())
    }
}
impl fmt::Display for SecondOrderIO {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tags = self
            .name
            .iter()
            .zip(self.size.iter())
            .enumerate()
            .map(|(k, (n, s))| format!(" #{:02} {:24}: [{:5}]", k, n, s))
            .collect::<Vec<String>>()
            .join("\n");
        write!(f, "{}", tags)
    }
}
impl fmt::Display for SecondOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let min_damping = self.zeta.iter().cloned().fold(std::f64::INFINITY, f64::min);
        let max_damping = self
            .zeta
            .iter()
            .cloned()
            .fold(std::f64::NEG_INFINITY, f64::max);
        write!(
            f,
            "Second Order ODE:\n - # of modes:{}\n - first 5 eigen frequencies: {:9.3?}\n - last 5 eigen frequencies: {:9.3?}\n - damping coefficients [min;max]: [{:.4};{:.4}]\n - B {:?}\n - C: {:?}\n - U\n{}\n Total: {}\n - Y\n{}\n Total: {}",
            self.n_mode(),
	    &self.omega[..5],
	    &self.omega[self.n_mode()-5..],
	    min_damping, max_damping,
	    (self.n_mode(),self.b.len()/self.n_mode()),
	    (self.c.len()/self.n_mode(),self.n_mode()),
            self.u,
            self.n_u(),
            self.y,
            self.n_y()
        )
    }
}
impl SecondOrder {
    pub fn into(self, u: Vec<Tags>, y: Vec<Tags>) -> Self {
        let pos: Vec<usize> = u.iter().filter_map(|u| self.u.position(u)).collect();
        let (u_name, u_size): (Vec<_>, Vec<_>) = pos
            .iter()
            .map(|p| (self.u.name[*p].clone(), self.u.size[*p].clone()))
            .unzip();
        let b: Vec<_> = self
            .b_rows()
            .flat_map(|b_row| {
                let u_size = self.u.size.clone();
                pos.iter().map(move |p| {
                    let n: usize = u_size.iter().take(*p).sum();
                    b_row
                        .iter()
                        .skip(n)
                        .take(u_size[*p])
                        .cloned()
                        .collect::<Vec<f64>>()
                })
            })
            .flatten()
            .collect();
        let pos: Vec<usize> = y.iter().filter_map(|y| self.y.position(y)).collect();
        let (y_name, y_size): (Vec<_>, Vec<_>) = pos
            .iter()
            .map(|p| (self.y.name[*p].clone(), self.y.size[*p].clone()))
            .unzip();
        let c: Vec<_> = pos
            .iter()
            .flat_map(|p| {
                let n: usize = self.y.size.iter().take(*p).sum();
                self.c_rows()
                    .skip(n)
                    .take(self.y.size[*p])
                    .flatten()
                    .cloned()
                    .collect::<Vec<f64>>()
            })
            .collect();
        SecondOrder {
            u: (u_name, u_size).into(),
            y: (y_name, y_size).into(),
            b,
            c,
            omega: self.omega,
            zeta: self.zeta,
        }
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
