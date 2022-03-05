use gmt_fem::{
    dos::{DiscreteModalSolver, Exponential, Get, Set},
    fem_io::*,
    FEM,
};

fn main() -> anyhow::Result<()> {
    type SS = DiscreteModalSolver<Exponential>;
    let fem = FEM::from_env()?;
    let mut state_space_obj = SS::from_fem(fem)
        .sampling(1000_f64)
        .proportional_damping(2. / 100.)
        .max_eigen_frequency(5f64)
        .ins::<OSSRotDriveTorque>()
        .ins::<OSSM1Lcl6F>()
        .ins::<OSSGIR6F>()
        .outs::<OSSRotEncoderAngle>()
        .outs::<OSSM1Lcl>()
        .outs::<OSSGIR6d>()
        .build()?;

    println!("{state_space_obj}");

    println!("ins : {:?}", state_space_obj.ins);
    println!("outs: {:?}", state_space_obj.outs);

    println!("u: {:?}", state_space_obj.u);
    println!("y: {:?}", state_space_obj.y);

    let u: Vec<f64> = (1..=42).map(|x| x as f64).collect();

    <SS as Set<OSSM1Lcl6F>>::set(&mut state_space_obj, &u);
    println!("u: {:?}", state_space_obj.u);
    println!("y: {:?}", <SS as Get<OSSGIR6d>>::get(&state_space_obj));

    Ok(())
}
