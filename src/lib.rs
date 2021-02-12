use serde_pickle as pkl;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub mod io;
pub use io::{IOData, IO};

pub mod wind_loads;
pub use wind_loads::WindLoads;

pub mod fem;
pub use fem::{fem_io, FEM};

pub mod state_space;
pub use state_space::{DiscreteApproximation, SerdeStateSpace2x2, StateSpace2x2};

pub mod bilinear;
pub use bilinear::Bilinear;
pub mod exponential;
pub use exponential::Exponential;

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

pub trait IOTraits {
    fn n(&self) -> usize;
    fn n_on(&self) -> usize;
    fn off(&mut self) -> &mut Self;
    fn on(&mut self, io_name: &str) -> &mut Self;
    fn on_by<F>(&mut self, io_name: &str, pred: F) -> &mut Self
    where
        F: Fn(&IOData) -> bool;
    fn io(&self, io_name: &str) -> Vec<&IOData>;
}

impl IOTraits for BTreeMap<String, Vec<IO>> {
    /// Gets the number of `io`
    fn n(&self) -> usize {
        self.values().fold(0, |a, x| a + x.len())
    }
    /// Gets the number of `io` that are turned on
    fn n_on(&self) -> usize {
        self.values().fold(0, |a, x| {
            a + x.iter().fold(0, |a, x| a + x.is_on() as usize)
        })
    }
    /// Turns off all `io`
    fn off(&mut self) -> &mut Self {
        self.values_mut().for_each(|value| {
            value.iter_mut().for_each(|io| {
                *io = io.clone().switch_off();
            })
        });
        self
    }
    /// Turns on the given `io`
    fn on(&mut self, io_name: &str) -> &mut Self {
        self.get_mut(io_name)
            .expect(&format!("IO {} not found", io_name))
            .iter_mut()
            .for_each(|io| {
                *io = io.clone().switch_on();
            });
        self
    }
    /// Turns on the given `io` for entries that match the give predicate
    fn on_by<F>(&mut self, io_name: &str, pred: F) -> &mut Self
    where
        F: Fn(&IOData) -> bool,
    {
        self.get_mut(io_name)
            .expect(&format!("IO {} not found", io_name))
            .iter_mut()
            .for_each(|io| {
                if let IO::Off(v) = io {
                    if pred(v) {
                        *io = IO::On(v.clone());
                    }
                }
            });
        self
    }
    /// Returns the turned-on entries of the requested `io`
    fn io(&self, io_name: &str) -> Vec<&IOData> {
        self.get(io_name)
            .expect(&format!("Input {} not found", io_name))
            .iter()
            .filter_map(|io| match io {
                IO::On(v) => Some(v),
                IO::Off(_) => None,
            })
            .collect()
    }
}
