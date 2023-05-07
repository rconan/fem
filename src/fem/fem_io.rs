//! # FEM inputs/outputs definitions

use std::{
    any::{type_name, Any},
    fmt,
    fmt::Debug,
    marker::PhantomData,
    ops::Range,
};

use crate::FEM;

use super::{
    io::{IOData, IO},
    FemError,
};
use nalgebra::DMatrix;
use serde::{Deserialize, Serialize};

pub trait FemIo<U> {
    fn position(&self) -> Option<usize>;
}
type Item = (String, Vec<IO>);

//fem_macros::ad_hoc! {}
include!(concat!(env!("OUT_DIR"), "/fem_actors_inputs.rs"));
include!(concat!(env!("OUT_DIR"), "/fem_actors_outputs.rs"));
include!(concat!(env!("OUT_DIR"), "/fem_get_in.rs"));
include!(concat!(env!("OUT_DIR"), "/fem_get_out.rs"));
include!(concat!(env!("OUT_DIR"), "/fem_inputs.rs"));
include!(concat!(env!("OUT_DIR"), "/fem_outputs.rs"));

#[derive(Serialize, Deserialize)]
pub struct SplitFem<U> {
    range: Range<usize>,
    io: PhantomData<U>,
}

impl<U> SplitFem<U> {
    pub fn new() -> Self {
        Self {
            range: Range::default(),
            io: PhantomData,
        }
    }
    pub fn fem_type(&self) -> String {
        type_name::<U>().to_string()
    }
    pub fn range(&self) -> &Range<usize> {
        &self.range
    }
}
impl<U> Debug for SplitFem<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("SplitFem<{}>", self.fem_type()))
            .field("range", &self.range)
            .finish()
    }
}
impl<U> Default for SplitFem<U> {
    fn default() -> Self {
        Self::new()
    }
}
pub trait SetRange {
    fn set_range(&mut self, start: usize, end: usize);
}
impl<U> SetRange for SplitFem<U> {
    fn set_range(&mut self, start: usize, end: usize) {
        self.range = Range { start, end };
    }
}
pub trait GetIn: SetRange + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn get_in(&self, fem: &FEM) -> Option<DMatrix<f64>>;
    fn trim_in(&self, fem: &FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>>;
    fn fem_type(&self) -> String;
    fn range(&self) -> Range<usize>;
    fn position(&self, fem: &Vec<Option<Inputs>>) -> Option<usize>;
}
impl<U: 'static + Send + Sync> GetIn for SplitFem<U>
where
    Vec<Option<Inputs>>: FemIo<U>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn get_in(&self, fem: &FEM) -> Option<DMatrix<f64>> {
        fem.in2modes::<U>()
            .as_ref()
            .map(|x| DMatrix::from_row_slice(fem.n_modes(), x.len() / fem.n_modes(), x))
    }
    fn trim_in(&self, fem: &FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>> {
        fem.trim2in::<U>(matrix)
    }
    fn fem_type(&self) -> String {
        self.fem_type()
    }

    fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    fn position(&self, inputs: &Vec<Option<Inputs>>) -> Option<usize> {
        <Vec<Option<Inputs>> as FemIo<U>>::position(inputs)
    }
}
pub trait GetOut: SetRange + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn get_out(&self, fem: &FEM) -> Option<DMatrix<f64>>;
    fn trim_out(&self, fem: &FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>>;
    fn fem_type(&self) -> String;
    fn range(&self) -> Range<usize>;
    fn position(&self, outputs: &Vec<Option<Outputs>>) -> Option<usize>;
}
impl<U: 'static + Send + Sync> GetOut for SplitFem<U>
where
    Vec<Option<Outputs>>: FemIo<U>,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn get_out(&self, fem: &FEM) -> Option<DMatrix<f64>> {
        fem.modes2out::<U>()
            .as_ref()
            .map(|x| DMatrix::from_row_slice(x.len() / fem.n_modes(), fem.n_modes(), x))
    }
    fn trim_out(&self, fem: &FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>> {
        fem.trim2out::<U>(matrix)
    }
    fn fem_type(&self) -> String {
        self.fem_type()
    }

    fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    fn position(&self, outputs: &Vec<Option<Outputs>>) -> Option<usize> {
        <Vec<Option<Outputs>> as FemIo<U>>::position(outputs)
    }
}
