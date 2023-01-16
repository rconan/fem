//! # FEM inputs/outputs definitions

use super::{
    io::{IOData, IO},
    FemError,
};
use dos_actors::{io::UniqueIdentifier, UID};
use serde;
use serde::Deserialize;

fem_macros::ad_hoc! {}
