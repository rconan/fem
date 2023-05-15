//! # FEM inputs/outputs definitions

use super::io::IO;

type Item = (String, Vec<IO>);

mod inputs {
    use super::Item;
    use crate::{FemError, IOData, IO};
    include!(concat!(env!("OUT_DIR"), "/fem_inputs.rs"));
}
mod outputs {
    use super::Item;
    use crate::{FemError, IOData, IO};
    include!(concat!(env!("OUT_DIR"), "/fem_outputs.rs"));
}
pub use inputs::Inputs;
pub use outputs::Outputs;
