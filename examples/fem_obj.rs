use dosio::ios;
use gmt_fem::{
    dos::{DiscreteModalSolver, Exponential, Position},
    fem_io::*,
    FEM,
};

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new().env().init().unwrap();
    let fem = FEM::from_env()?;
    println!("{}", fem);

    let state_space = DiscreteModalSolver::<Exponential>::from_fem(fem)
        .sampling(1000_f64)
        .proportional_damping(2. / 100.)
        .max_eigen_frequency(5f64)
        .inputs(ios!(OSSRotDriveTorque, OSSM1Lcl6F, OSSGIR6F))
        .outputs(ios!(OSSRotEncoderAngle, OSSM1Lcl, OSSGIR6d))
        .build()?;
    println!("{}", state_space);
    println!("Y sizes: {:?}", state_space.y_sizes);

    let fem = FEM::from_env()?;
    let mut state_space_obj = DiscreteModalSolver::<Exponential>::from_fem(fem)
        .sampling(1000_f64)
        .proportional_damping(2. / 100.)
        .max_eigen_frequency(5f64)
        .ins::<OSSRotDriveTorque>()
        .ins::<OSSM1Lcl6F>()
        .ins::<OSSGIR6F>()
        .outs::<OSSRotEncoderAngle>()
        .outs::<OSSM1Lcl>()
        .outs::<OSSGIR6d>()
        .build_obj()?;

    println!("ins : {:?}", state_space_obj.ins);
    println!("outs: {:?}", state_space_obj.outs);

    state_space
        .state_space
        .iter()
        .zip(state_space_obj.state_space.iter())
        .enumerate()
        .for_each(|(k, (a, b))| {
            if a == b {
                println!("#{k:02} Y");
            } else {
                println!("#{k:02} N");
            }
        });

    println!("u: {:?}", state_space_obj.u);
    println!("y: {:?}", state_space_obj.y);

    let u: Vec<f64> = (1..=42).map(|x| x as f64).collect();

    state_space_obj.set::<OSSM1Lcl6F>(&u);
    println!("u: {:?}", state_space_obj.u);
    println!("y: {:?}", state_space_obj.get::<OSSGIR6d>());

    Ok(())
}
