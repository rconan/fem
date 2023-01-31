//! M1 CONTROL

#[doc(hidden)]
pub use super::prelude;
use super::prelude::*;
use dos_clients_io::gmt_m1::{M1ModeShapes, M1RigidBodyMotions};

pub mod actuators;
pub mod hardpoints;
pub mod rigid_body_motions;

impl<S> Get<M1ModeShapes> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn get(&self) -> Option<Vec<f64>> {
        let mut encoders = <DiscreteModalSolver<S> as Get<fem_io::M1Segment1AxialD>>::get(self)?;
        encoders.extend(
            <DiscreteModalSolver<S> as Get<fem_io::M1Segment2AxialD>>::get(self)?.as_slice(),
        );
        encoders.extend(
            <DiscreteModalSolver<S> as Get<fem_io::M1Segment3AxialD>>::get(self)?.as_slice(),
        );
        encoders.extend(
            <DiscreteModalSolver<S> as Get<fem_io::M1Segment4AxialD>>::get(self)?.as_slice(),
        );
        encoders.extend(
            <DiscreteModalSolver<S> as Get<fem_io::M1Segment5AxialD>>::get(self)?.as_slice(),
        );
        encoders.extend(
            <DiscreteModalSolver<S> as Get<fem_io::M1Segment6AxialD>>::get(self)?.as_slice(),
        );
        encoders.extend(
            <DiscreteModalSolver<S> as Get<fem_io::M1Segment7AxialD>>::get(self)?.as_slice(),
        );
        Some(encoders)
    }
}
impl<S> Write<M1ModeShapes> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn write(&mut self) -> Option<Arc<Data<M1ModeShapes>>> {
        <DiscreteModalSolver<S> as Get<M1ModeShapes>>::get(self)
            .map(|data| Arc::new(Data::new(data)))
    }
}
//  * M1 rigid body motions
impl<S> Size<M1RigidBodyMotions> for DiscreteModalSolver<S>
where
    DiscreteModalSolver<S>: Iterator,
    S: Solver + Default,
{
    fn len(&self) -> usize {
        42
    }
}
impl<S> Write<M1RigidBodyMotions> for DiscreteModalSolver<S>
where
    S: Solver + Default,
{
    fn write(&mut self) -> Option<Arc<Data<M1RigidBodyMotions>>> {
        <DiscreteModalSolver<S> as Get<fem_io::OSSM1Lcl>>::get(self)
            .map(|data| Arc::new(Data::new(data)))
    }
}
