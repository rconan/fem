//! M2 FSM Piezo-Stack Actuators

use super::prelude::*;
use dos_clients_io::gmt_m2::fsm::{M2FSMPiezoForces, M2FSMPiezoNodes};

/// forces
impl<S> Read<M2FSMPiezoForces> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn read(&mut self, data: Arc<Data<M2FSMPiezoForces>>) {
        <DiscreteModalSolver<S> as Set<fem_io::MCM2PZTF>>::set(self, &data)
    }
}
/// nodes
impl<S> Write<M2FSMPiezoNodes> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn write(&mut self) -> Option<Arc<Data<M2FSMPiezoNodes>>> {
        <DiscreteModalSolver<S> as Get<fem_io::MCM2PZTD>>::get(self)
            .map(|data| Arc::new(Data::new(data)))
    }
}
