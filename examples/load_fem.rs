use gmt_fem::FEM;
use std::path::Path;

fn main() {
    let fem = FEM::from_env().unwrap();
    println!("{fem}");
    dbg!(&fem.inputs[14]);
}
