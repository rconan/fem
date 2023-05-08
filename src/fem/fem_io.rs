//! # FEM inputs/outputs definitions

use std::{
    any::{type_name, Any},
    fmt,
    fmt::Debug,
    marker::PhantomData,
    ops::Range,
};

use crate::FEM;

use super::io::IO;
use nalgebra::DMatrix;

/// Find the index corresponding to `U` in the [FEM] [Inputs] and [Outputs] vectors
///
/// `U` is either an [actors_inputs] or [actors_outputs].
pub trait FemIo<U> {
    /// Returns the index position
    fn position(&self) -> Option<usize>;
}
type Item = (String, Vec<IO>);

//fem_macros::ad_hoc! {}
mod inputs {
    use super::{FemIo, GetIn, Item, SplitFem};
    use crate::{FemError, IOData, IO};
    pub mod actors_inputs {
        include!(concat!(env!("OUT_DIR"), "/fem_actors_inputs.rs"));
    }
    use actors_inputs::*;
    include!(concat!(env!("OUT_DIR"), "/fem_inputs.rs"));
    include!(concat!(env!("OUT_DIR"), "/fem_get_in.rs"));
}
mod outputs {
    use super::{FemIo, GetOut, Item, SplitFem};
    use crate::{FemError, IOData, IO};
    pub mod actors_outputs {
        include!(concat!(env!("OUT_DIR"), "/fem_actors_outputs.rs"));
    }
    use actors_outputs::*;
    include!(concat!(env!("OUT_DIR"), "/fem_outputs.rs"));
    include!(concat!(env!("OUT_DIR"), "/fem_get_out.rs"));
}
pub use inputs::{actors_inputs, Inputs};
pub use outputs::{actors_outputs, Outputs};

/// Hold the range of indices corresponding to `U` in the [FEM] [Inputs] and [Outputs] vectors
///
/// `U` is either an [actors_inputs] or [actors_outputs].
#[cfg_attr(features = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SplitFem<U> {
    range: Range<usize>,
    io: PhantomData<U>,
}

impl<U> SplitFem<U> {
    /// Creates a new [SplitFem] object
    pub fn new() -> Self {
        Self {
            range: Range::default(),
            io: PhantomData,
        }
    }
    /// Returns the actors type
    pub fn fem_type(&self) -> String {
        type_name::<U>().to_string()
    }
    /// Returns the range
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

/// Range setting for [SplitFem]
pub trait SetRange {
    /// Sets the range
    fn set_range(&mut self, start: usize, end: usize);
}
impl<U> SetRange for SplitFem<U> {
    fn set_range(&mut self, start: usize, end: usize) {
        self.range = Range { start, end };
    }
}
/// Interface between the FEM [Inputs] and the [DOS actors inputs](actors_inputs)
pub trait GetIn: SetRange + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    /// Returns the inputs to modes matrix for a given input
    fn get_in(&self, fem: &FEM) -> Option<DMatrix<f64>>;
    /// Trims the inputs to modes matrix to the given input
    fn trim_in(&self, fem: &FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>>;
    /// Returns the actors type
    fn fem_type(&self) -> String;
    /// Sets the input range of indices
    fn range(&self) -> Range<usize>;
    /// Returns the input position in the FEM [Inputs] vector
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
/// Interface between the FEM [Outputs] and the [DOS actors outputs](actors_outputs)
pub trait GetOut: SetRange + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    /// Returns the outputs to modes matrix for a given output
    fn get_out(&self, fem: &FEM) -> Option<DMatrix<f64>>;
    /// Trims the outputs to modes matrix to the given output
    fn trim_out(&self, fem: &FEM, matrix: &DMatrix<f64>) -> Option<DMatrix<f64>>;
    /// Returns the actors type
    fn fem_type(&self) -> String;
    /// Sets the output range of indices
    fn range(&self) -> Range<usize>;
    /// Returns the output position in the FEM [Outputs] vector
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
