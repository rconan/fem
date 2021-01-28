use fem::{IOTraits, FEM, ToPickle};
use nalgebra as na;
use std::time::Instant;
use serde_pickle as pkl;
use std::fs::File;
use rayon::prelude::*;

struct Timer {
    time: Instant
}
impl Timer {
    pub fn tic() -> Self {
        Self{ time: Instant::now()}
    }
    pub fn toc(self) -> f64 {
        self.time.elapsed().as_secs_f64()
    }
    pub fn print_toc(self) {
        println!("... in {:3}s",self.toc());
    }
}

fn main() {
    let tic = Timer::tic();
    println!("Loading FEM ...");
    let mut fem = FEM::from_pkl("examples/modal_state_space_model_2ndOrder.pkl").unwrap();
    tic.print_toc();
    fem.inputs.off();
    fem.inputs.on("OSS_TopEnd_6F");
    fem.outputs.off();
    fem.outputs.on("MC_M2_lcl_6D");
    println!("in/out: {}/{}", fem.inputs.n_on(), fem.outputs.n_on());
    fem.inputs2modes().to_pickle("examples/forces_2_modes.pkl").unwrap();

    let tic = Timer::tic();
    println!("Building 2x2 state space models ...");
    let mut ss = fem.state_space(2e3);
    tic.print_toc();
    println!("# of state space models: {}", ss.len());
    println!("{}", ss[0]);
    println!("{}", ss[0].aa);
    ss[0].to_serde().to_pickle("examples/ss0.pkl").unwrap();

    let mut u = vec![0.; fem.inputs.n_on()];
    u[0] = 1.;
    let tic = Timer::tic();
    println!("Running model ...");
    let y: Vec<_> = (0..10000)
        .map(|_| {
            let z = na::DVector::zeros(fem.outputs.n_on());
            ss.iter_mut()
                .fold(z, |mut y, m| {
                    m.solve(&u);
                    y += &m.y;
                    y
                }).as_slice().to_vec()
        })
        .collect();
    /*
    ss.par_iter_mut().for_each(|m| {
        (0..10000).for_each(|_| {
             m.solve(&u);
        })
    });
    */
    tic.print_toc();
    //println!("y dim0: {}", y.len());
    //println!("y dim1: {:?}", y[0].len());
    //println!("y[0]: {:?}",y[9]);

    //let mut f = File::create("examples/y.pkl").unwrap();
    //pkl::to_writer(&mut f, &y, true).unwrap();
}
