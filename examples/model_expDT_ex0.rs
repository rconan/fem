// Example to compute the exp

use nalgebra::{Vector3, RowVector3, Matrix3};
use num_complex::Complex;

const Z_CPLX:Complex<f64> = Complex {re: 0., im: 0.};
const PI:f64 = 3.141592653589793;

fn main() {
    println!("Testing implementation of 2nd order model discretization algorithm!");

    let om:f64 = 486. * (2. * PI);
    let zeta:f64 = 0.02;
    let ts = 0.001;
    // Complex pole of the 2nd order model
    let lambda_cplx = Complex { re: - om*zeta, im: om * (1.-(zeta*zeta)).sqrt() };

    let exp_m = if om != 0.
    {
        // Some of the diagonalization matrix elements
        let v11 = Complex { re: 1. / ((1. + om.powi(4)).sqrt()), im: 0. };
        let v31 = (om*om) * v11;    
        let v12 = 1. / (1. + lambda_cplx.norm_sqr()).sqrt();
        let v13 = 1. / (1. + lambda_cplx.conj().norm_sqr()).sqrt();    
        //println!("v12:{}", format!("{:.6e}", v12));
        //println!("v22:{}", format!("{:.4e}", lambda_cplx * v12));
        //println!("v13:{}", format!("{:.6e}", v13));
        //println!("v23:{}", format!("{:.4e}", lambda_cplx.conj() * v13));
        
        // Matrix of eigenvectors
        let v = Matrix3::from_columns(&[
            Vector3::new(v11, Z_CPLX, v31),
            Vector3::new(Complex{re:v12, im:0.}, lambda_cplx * v12, Z_CPLX),
            Vector3::new(Complex{re:v13, im:0.}, lambda_cplx.conj() * v13, Z_CPLX)]);

        let k_row2 = (v12*(lambda_cplx.conj() - lambda_cplx)).inv();
        let k_row3 = (v13*(lambda_cplx.conj() - lambda_cplx)).inv();
        let inv_v = Matrix3::from_rows(&[
            RowVector3::new(Z_CPLX, Z_CPLX, v31.inv()),
            RowVector3::new(
                lambda_cplx.conj()*k_row2,
                Complex{re: -1., im: 0.}*k_row2,
                -lambda_cplx.conj().unscale(om*om)*k_row2
            ),
            RowVector3::new(
                -lambda_cplx*k_row3,
                Complex{re:1., im:0.}*k_row3,
                lambda_cplx.unscale(om*om)*k_row3
            )]);
        
        // Conversion from Complex to real for inverse evaluation is not working 
        //let real_m = (v * inv_v).as_slice().iter().map(|&x| x.re).collect();
        //assert_eq!(Matrix3::<f32>::identity(),real_m);    

        let diag_exp = Matrix3::from_columns(&[
            Vector3::new(Complex{re:1., im:0.}, Z_CPLX, Z_CPLX),
            Vector3::new(Z_CPLX, lambda_cplx.scale(ts).exp(), Z_CPLX),
            Vector3::new(Z_CPLX, Z_CPLX , lambda_cplx.conj().scale(ts).exp())]);

        v * diag_exp * inv_v
    } else
    {
        Matrix3::from_columns(&[
            Vector3::new(Complex{re:1., im:0.}, Z_CPLX, Z_CPLX),
            Vector3::new(Complex{re:1., im:0.}, Complex{re:1., im:0.}, Z_CPLX),
            Vector3::new(Complex{re:0.5, im:0.},
                Complex{re:1., im:0.},
                Complex{re:1., im:0.})])
    };
    // Take just the real part
    let phi = exp_m.fixed_slice::<2,2>(0,0);
    let gamma = exp_m.slice((0,2),(2,1));
    println!("Ad coefficients:");
    println!("phi11: {} phi12: {}",
        format!("{:.6e}", phi[0].re),
        format!("{:.6e}", phi[2].re));
    println!("phi21: {} phi22: {}",
        format!("{:.6e}", phi[1].re),
        format!("{:.6e}", phi[3].re));
    println!("Bd coefficients:");
    println!("Gamma1:{}\nGamma2:{}",
        format!("{:.6e}", gamma[0].re),
        format!("{:.6e}", gamma[1].re));

}
