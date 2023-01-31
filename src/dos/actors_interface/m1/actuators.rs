//! M1 segment actuators

use super::prelude::*;
use dos_clients_io::gmt_m1::segment::ActuatorAppliedForces;

impl<const ID: u8, S: Solver + Default> Read<ActuatorAppliedForces<ID>> for DiscreteModalSolver<S> {
    fn read(&mut self, data: Arc<Data<ActuatorAppliedForces<ID>>>) {
        match ID {
            1 => <DiscreteModalSolver<S> as Set<fem_io::M1ActuatorsSegment1>>::set(self, &data),
            2 => <DiscreteModalSolver<S> as Set<fem_io::M1ActuatorsSegment2>>::set(self, &data),
            3 => <DiscreteModalSolver<S> as Set<fem_io::M1ActuatorsSegment3>>::set(self, &data),
            4 => <DiscreteModalSolver<S> as Set<fem_io::M1ActuatorsSegment4>>::set(self, &data),
            5 => <DiscreteModalSolver<S> as Set<fem_io::M1ActuatorsSegment5>>::set(self, &data),
            6 => <DiscreteModalSolver<S> as Set<fem_io::M1ActuatorsSegment6>>::set(self, &data),
            7 => <DiscreteModalSolver<S> as Set<fem_io::M1ActuatorsSegment7>>::set(self, &data),
            _ => unreachable!(),
        }
    }
}
