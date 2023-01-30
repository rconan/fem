//! M2 rigid body motions

use crate::{
    dos::{DiscreteModalSolver, Get, Solver},
    fem_io,
};
use dos_clients_io::gmt_m2::M2RigidBodyMotions;
use gmt_dos_actors::io::{Data, Size, Write};
use std::sync::Arc;

impl<S> Size<M2RigidBodyMotions> for DiscreteModalSolver<S>
where
    DiscreteModalSolver<S>: Iterator,
    S: Solver + Default,
{
    fn len(&self) -> usize {
        42
    }
}
#[cfg(not(feature = "mcm2lcl"))]
impl<S> Write<M2RigidBodyMotions> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn write(&mut self) -> Option<Arc<Data<M2RigidBodyMotions>>> {
        <DiscreteModalSolver<S> as Get<fem_io::MCM2Lcl6D>>::get(self)
            .map(|data| Arc::new(Data::new(data)))
    }
}
#[cfg(feature = "mcm2lcl")]
impl<S> Write<M2RigidBodyMotions> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn write(&mut self) -> Option<Arc<Data<M2RigidBodyMotions>>> {
        <DiscreteModalSolver<S> as Get<fem_io::MCM2Lcl>>::get(self)
            .map(|data| Arc::new(Data::new(data)))
    }
}
