use gmt_fem::{
    dos::{DiscreteStateSpace, Exponential},
    FEM,
};

fn frequency_base2_histogram<'a>(nu: &[f64], max_nu: f64) -> Vec<usize> {
    (0..)
        .map_while(|i| {
            let upper = 2i32 << i;
            let lower = if i == 0 { 0 } else { upper >> 1 };
            if lower as f64 > max_nu {
                None
            } else {
                Some(
                    nu.iter()
                        .filter(|&&nu| nu >= lower as f64 && nu < upper as f64)
                        .enumerate()
                        .last()
                        .map_or_else(|| 0, |(i, _)| i + 1),
                )
            }
        })
        .collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fem = FEM::from_env()?;

    let nu = fem.eigen_frequencies.clone();
    let max_nu = nu.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut nu_hist = vec![frequency_base2_histogram(&nu, max_nu)];

    let state_space = DiscreteStateSpace::<Exponential>::from(fem);
    state_space.fem_info();
    let hsv = state_space.hankel_singular_values()?;

    let max_hsv = hsv.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let nu_hsv: Vec<_> = nu.iter().cloned().zip(hsv.into_iter()).collect();

    let n_mode = nu_hsv.len();

    print!(
        r#"
HANKEL SINGULAR VALUES MODEL REDUCTION
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
"#
    );

    let model_log_reduction = vec![-6, -5, -4, -3];

    for (k, exp) in model_log_reduction.into_iter().enumerate() {
        let hsv_threshold = 10f64.powi(exp);
        let red_nu_hsv: Vec<&(f64, f64)> = nu_hsv
            .iter()
            .filter(|(_, hsv)| *hsv > max_hsv * hsv_threshold)
            .collect();

        nu_hist.push(frequency_base2_histogram(
            red_nu_hsv
                .iter()
                .map(|(nu, _)| *nu)
                .collect::<Vec<f64>>()
                .as_slice(),
            max_nu,
        ));

        let min_nu = red_nu_hsv
            .iter()
            .map(|(nu, _)| nu)
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let max_nu = red_nu_hsv
            .iter()
            .map(|(nu, _)| nu)
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        println!(
            r#"
{}. reduced model:
 . hankel singular value threshold: {:.3e} ({:e})
 . # of modes: {} ({:.1})%
 . eigen frequencies range: {:.3?}Hz
    "#,
            k + 1,
            max_hsv * hsv_threshold,
            hsv_threshold,
            red_nu_hsv.len(),
            100. * red_nu_hsv.len() as f64 / n_mode as f64,
            (min_nu, max_nu)
        );
    }

    println!(" {}", "-".repeat(43));
    println!(" |{:^41}|", "Models Frequency Histograms");
    println!(" |{}|", "-".repeat(41));
    println!(" |{:^6}|{:^34}|", "Bin", "Models");
    print!(" |{:^5}", "Hz");
    for i in 0..nu_hist.len() {
        print!(" | {:^4}", i);
    }
    println!(" |");
    println!(" {}|", "|------".repeat(1 + nu_hist.len()));
    let n_bin = nu_hist[0].len();
    for i in 0..n_bin {
        let upper = 2 << i;
        print!(" |{:>5} ", upper);
        for hist in &nu_hist {
            print!("|{:>5} ", hist[i]);
        }
        println!("|");
    }
    println!(" {}", "-".repeat(43));

    Ok(())
}
