//! cold plate

use super::prelude::*;
use dos_clients_io::gmt_m2::asm::M2ASMColdPlateForces;

/// forces
impl<S> Read<M2ASMColdPlateForces> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn read(&mut self, data: Arc<Data<M2ASMColdPlateForces>>) {
        <DiscreteModalSolver<S> as Set<fem_io::MCM2CP6F>>::set(self, &data)
    }
}
