//! M1 segment hardpoints

use super::prelude::*;
use dos_clients_io::gmt_m1::segment::{HardpointsForces, HardpointsMotion};

impl<const ID: u8, S: Solver + Default> Read<HardpointsForces<ID>> for DiscreteModalSolver<S> {
    fn read(&mut self, data: Arc<Data<HardpointsForces<ID>>>) {
        let a: usize = (ID * 6).into();
        <DiscreteModalSolver<S> as Set<fem_io::OSSHarpointDeltaF>>::set_slice(
            self,
            &data,
            a - 6..a,
        );
    }
}

impl<const ID: u8, S: Solver + Default> Write<HardpointsMotion<ID>> for DiscreteModalSolver<S> {
    fn write(&mut self) -> Option<Arc<Data<HardpointsMotion<ID>>>> {
        let a: usize = (ID * 12).into();
        <DiscreteModalSolver<S> as Get<fem_io::OSSHardpointD>>::get(self)
            .as_ref()
            .map(|data| Arc::new(Data::new((data[a - 12..a]).to_vec())))
    }
}
