//! rigid body

use super::prelude::*;
use dos_clients_io::gmt_m2::asm::{M2ASMRigidBodyForces, M2ASMRigidBodyNodes};

/// forces
impl<S> Read<M2ASMRigidBodyForces> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn read(&mut self, data: Arc<Data<M2ASMRigidBodyForces>>) {
        <DiscreteModalSolver<S> as Set<fem_io::MCM2RB6F>>::set(self, &data)
    }
}
/// nodes
impl<S> Write<M2ASMRigidBodyNodes> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn write(&mut self) -> Option<Arc<Data<M2ASMRigidBodyNodes>>> {
        <DiscreteModalSolver<S> as Get<fem_io::MCM2RB6D>>::get(self)
            .map(|data| Arc::new(Data::new(data)))
    }
}
