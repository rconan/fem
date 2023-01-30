/*!
# GMT Finite Element Model client

The module implements the client interface for the [GMT FEM Rust API](https://docs.rs/gmt-fem)

*/

#[doc(hidden)]
pub mod prelude {
    pub use crate::{
        dos::{DiscreteModalSolver, Get, Set, Solver},
        fem_io,
    };
    pub use gmt_dos_actors::{
        io::{Data, Read, Size, Write},
        UniqueIdentifier, Update,
    };
    pub use std::sync::Arc;
}

use prelude::*;

#[cfg(feature = "cfd2022")]
pub mod cfd;
pub mod m1;
pub mod m2;
pub mod mount;

impl<S> Update for DiscreteModalSolver<S>
where
    DiscreteModalSolver<S>: Iterator,
    S: Solver + Default,
{
    fn update(&mut self) {
        log::debug!("update");
        self.next();
    }
}

impl<S, U: UniqueIdentifier<DataType = Vec<f64>>> Read<U> for DiscreteModalSolver<S>
where
    Vec<Option<fem_io::Inputs>>: fem_io::FemIo<U>,
    S: Solver + Default,
    U: 'static,
{
    fn read(&mut self, data: Arc<Data<U>>) {
        <DiscreteModalSolver<S> as Set<U>>::set(self, &**data)
    }
}

impl<S, U: UniqueIdentifier<DataType = Vec<f64>>> Write<U> for DiscreteModalSolver<S>
where
    Vec<Option<fem_io::Outputs>>: fem_io::FemIo<U>,
    S: Solver + Default,
    U: 'static,
{
    fn write(&mut self) -> Option<Arc<Data<U>>> {
        <DiscreteModalSolver<S> as Get<U>>::get(self).map(|data| Arc::new(Data::new(data)))
    }
}
