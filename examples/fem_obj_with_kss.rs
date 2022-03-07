use gmt_fem::{
    dos::{DiscreteModalSolver, ExponentialMatrix, Get, Set},
    fem_io::*,
    FEM,
};

fn main() -> anyhow::Result<()> {
    //simple_logger::SimpleLogger::new().env().init().unwrap();

    type SS = DiscreteModalSolver<ExponentialMatrix>;
    let gmt_fem_dt = FEM::from_env()?.static_from_env();
    println!("{}", gmt_fem_dt);
    let n_io = (gmt_fem_dt.n_inputs(),gmt_fem_dt.n_outputs());
    let mut state_space_obj = SS::from_fem(gmt_fem_dt)
        .sampling(1000_f64)
        .proportional_damping(2. / 100.)
        .max_eigen_frequency(75f64)
        .ins::<OSSAzDriveTorque>()
        .ins::<OSSElDriveTorque>()
        .ins::<OSSRotDriveTorque>()
        .ins::<OSSM1Lcl6F>()
        .ins::<OSSGIR6F>()
        .outs::<OSSAzEncoderAngle>()
        .outs::<OSSElEncoderAngle>()
        .outs::<OSSRotEncoderAngle>()
        .outs::<OSSM1Lcl>()
        .outs::<OSSGIR6d>()
        .use_static_gain_compensation(n_io).build()?;

    println!("{}",state_space_obj);

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
