use fem::{WindLoads, FEM};
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
    println!("Loading wind loads ...");
    let mut wind = WindLoads::from_pickle("examples/trimmer_finest_mesh_20Hz.neu.pkl").unwrap();
    tic.print_toc();

    let tic = Timer::tic();
    println!("Loading FEM ...");
    let mut fem = FEM::from_pickle("examples/modal_state_space_model_2ndOrder_v1.pkl").unwrap();
    tic.print_toc();
    println!("{}", fem);
    fem.keep_inputs(&[1, 2, 3, 4, 5, 6, 13])
        .keep_outputs(&[5, 24]);
    println!("{}", fem);

    let tic = Timer::tic();
    let sampling = 2000.0;
    println!("Building 2x2 state space models ...");
    fem.build(sampling);
    tic.print_toc();
    println!(
        "# of state space models: {}",
        fem.state_space.as_ref().unwrap().len()
    );

    wind.n_sample /= 10;
    println!("# of steps: {}", wind.n_sample);
    let mut u = vec![0f64; 6];
    u[0] = 1.;
    println!("Running model ...");
    let tic = Timer::tic();

    let mut y: Vec<Vec<f64>> = Vec::with_capacity(wind.n_sample * fem.n_outputs());
    for _ in 0..wind.n_sample {
        fem.set_u(&mut wind.outputs).next();
    }

    tic.print_toc();

    let mut f = File::create("examples/wind_loads_y.pkl").unwrap();
    pkl::to_writer(&mut f, &y, true).unwrap();
}
