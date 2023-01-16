use gmt_fem::{
    dos::{DiscreteStateSpace, Exponential},
    FEM,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fem = FEM::from_env()?;
    let nu = fem.eigen_frequencies.clone();
    let state_space = DiscreteStateSpace::<Exponential>::from(fem);
    state_space.fem_info();
    let hsv = state_space.hankel_singular_values()?;

    let max_hsv = hsv.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let nu_hsv: Vec<_> = nu.into_iter().zip(hsv.into_iter()).collect();
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

    Ok(())
}
