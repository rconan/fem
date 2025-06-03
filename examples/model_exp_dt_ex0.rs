// Compare different model of tranformation of continuous second order ODE into a discrete state space model
// Run with: `cargo run --release --example model_exp_dt_ex0 --features dos`

use gmt_fem::dos::{Exponential, ExponentialMatrix, Solver};

const PI: f64 = 3.141592653589793;

fn main() {
    println!("Testing implementation of 2nd order model discretization algorithm!");

    let om: f64 = 2e3 * (2. * PI);
    let zeta: f64 = 0.02;
    let ts = 0.001;

    let expm = ExponentialMatrix::from_second_order(ts, om, zeta, vec![], vec![]);
    println!("ExpM: {}", expm);
    let exp = Exponential::from_second_order(ts, om, zeta, vec![], vec![]);
    println!("Exp: {}", exp);
}
