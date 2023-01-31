//! CFD

use super::prelude::*;
use dos_clients_io::cfd_wind_loads::{CFDM1WindLoads, CFDMountWindLoads};

/// mount
impl<S> Read<CFDMountWindLoads> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn read(&mut self, data: Arc<Data<CFDMountWindLoads>>) {
        <DiscreteModalSolver<S> as Set<fem_io::CFD2021106F>>::set(self, &data)
    }
}
/// M1
impl<S> Read<CFDM1WindLoads> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn read(&mut self, data: Arc<Data<CFDM1WindLoads>>) {
        <DiscreteModalSolver<S> as Set<fem_io::OSSM1Lcl6F>>::set(self, &data)
    }
}

#[cfg(any(feature = "asm", feature = "fsm"))]
use dos_clients_io::cfd_wind_loads::CFDM2WindLoads;
/// M2
#[cfg(feature = "asm")]
impl<S> Read<CFDM2WindLoads> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn read(&mut self, data: Arc<Data<CFDM2WindLoads>>) {
        <DiscreteModalSolver<S> as Set<fem_io::MCM2Lcl6F>>::set(self, &data)
    }
}
#[cfg(feature = "fsm")]
impl<S> Read<CFDM2WindLoads> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn read(&mut self, data: Arc<Data<CFDM2WindLoads>>) {
        <DiscreteModalSolver<S> as Set<fem_io::MCM2LclForce6F>>::set(self, &data)
    }
}
