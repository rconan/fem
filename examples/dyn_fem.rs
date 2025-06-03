use fem::{FEM, IOTraits, ToPickle};
//use nalgebra as na;
//use rayon::prelude::*;
use serde_pickle as pkl;
use std::fs::File;
use std::time::Instant;

struct Timer {
    time: Instant,
}
impl Timer {
    pub fn tic() -> Self {
        Self {
            time: Instant::now(),
        }
    }
    pub fn toc(self) -> f64 {
        self.time.elapsed().as_secs_f64()
    }
    pub fn print_toc(self) {
        println!("... in {:3}s", self.toc());
    }
}

fn main() {
    let tic = Timer::tic();
    println!("Loading FEM ...");
    let mut fem = FEM::from_pkl("examples/modal_state_space_model_2ndOrder_v1.pkl").unwrap();
    println!("FEM\n{}", fem);
    tic.print_toc();
    fem.keep_input(4);
    fem.keep_output(24);
    println!("FEM\n{}", fem);
    fem.inputs2modes()
        .to_pickle("examples/forces_2_modes.pkl")
        .unwrap();

    let tic = Timer::tic();
    let sampling = 2000.;
    println!("Building 2x2 state space models ...");
    let mut ss = fem.state_space(sampling);
    //    let mut ss = fem.state_space(sampling);
    tic.print_toc();
    println!("# of state space models: {}", ss.len());
    //println!("{}", ss[0]);
    //println!("{}", ss[0].aa);
    ss[0].to_pickle("examples/bl0.pkl").unwrap();

    let mut u = vec![0.; fem.n_inputs()];
    u[0] = 1.;
    let duration = 100.;
    let n = (duration * sampling) as usize;
    let mut y: Vec<Vec<f64>> = vec![vec![0.; fem.n_outputs()]; n];

    println!("Running model ...");
    let tic = Timer::tic();
    y.iter_mut().for_each(|y_step| {
        ss.iter_mut().fold(y_step, |y, m| {
            y.iter_mut().zip(m.solve(&u)).for_each(|(yc, y)| {
                *yc += y;
            });
            y
        });
    });
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

    let mut f = File::create("examples/y.pkl").unwrap();
    pkl::to_writer(&mut f, &y, true).unwrap();
}
