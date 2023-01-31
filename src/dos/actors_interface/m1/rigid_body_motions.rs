//! M1 segment rigid body motions

use super::prelude::*;
use dos_clients_io::gmt_m1::segment::RBM;

impl<const ID: u8, S: Solver + Default> Write<RBM<ID>> for DiscreteModalSolver<S> {
    fn write(&mut self) -> Option<Arc<Data<RBM<ID>>>> {
        let a: usize = (ID * 6).into();
        <DiscreteModalSolver<S> as Get<fem_io::OSSM1Lcl>>::get(self)
            .as_ref()
            .map(|data| Arc::new(Data::new((data[a - 6..a]).to_vec())))
    }
}
