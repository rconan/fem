//! face sheet

use super::prelude::*;
use dos_clients_io::gmt_m2::asm::{M2ASMFaceSheetForces, M2ASMFaceSheetNodes};

/// forces
impl<S> Read<M2ASMFaceSheetForces> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn read(&mut self, data: Arc<Data<M2ASMFaceSheetForces>>) {
        <DiscreteModalSolver<S> as Set<fem_io::MCM2Lcl6F>>::set(self, &data)
    }
}
// * nodes
impl<S> Write<M2ASMFaceSheetNodes> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn write(&mut self) -> Option<Arc<Data<M2ASMFaceSheetNodes>>> {
        <DiscreteModalSolver<S> as Get<fem_io::MCM2Lcl6D>>::get(self)
            .map(|data| Arc::new(Data::new(data)))
    }
}
