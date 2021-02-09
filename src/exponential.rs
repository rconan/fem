use nalgebra::Matrix2;
use serde::Serialize;
use num_complex::Complex;

#[derive(Debug,Serialize,Clone)]
pub struct Exponential {
    pub tau: f64,
    pub q: (f64, f64, f64, f64),
    pub m: (f64, f64, f64, f64),
    pub b: Vec<f64>,
    pub c: Vec<f64>,
    pub y: Vec<f64>,
    x: (f64, f64),
}
impl Exponential {
    pub fn from_second_order(
        tau: f64,
        omega: f64,
        zeta: f64,
        continuous_bb: Vec<f64>,
        continuous_cc: Vec<f64>,
    ) -> Self {
        /*
        let aa = Matrix2::<f64>::new(0., 1., -omega * omega, -2. * omega * zeta);
        let i = Matrix2::<f64>::identity();
        let qp = i + aa * (0.5 * tau);
        let iqm = (i - aa * (0.5 * tau)).try_inverse().unwrap();
        let q = (qp * iqm).as_slice().to_owned();
        let m = (iqm * tau.sqrt()).as_slice().to_owned();
        */
        let i = Matrix2::<f64>::identity();
        let x = Complex { re: omega, im: 0. };
        let y = Complex { re: zeta, im: 0. };
        let ia = Matrix2::new((-2. * y / x).re, -1. / (x * x).re, 1., 0.);
        let z = (x * x * (y * y - 1.)).sqrt();
        let zmxy = z - x * y;
        let zpxy = z + x * y;
        let ezmxy = (tau * zmxy).exp();
        let ezpxy = (-tau * zpxy).exp();
        let ad = Matrix2::new(
            ((zpxy * ezmxy + zmxy * ezpxy) / (2. * z)).re,
            ((ezmxy - ezpxy) / (2. * z)).re,
            (x * x * (ezpxy - ezmxy) / (2. * z)).re,
            ((zmxy * ezmxy + zpxy * ezpxy) / (2. * z)).re,
        );
        let bd = ia * (ad - i)/tau.sqrt();
        let n = continuous_cc.len();
        Self {
            tau,
            q: (ad[0], ad[2], ad[1], ad[3]),
            m: (bd[0], bd[2], bd[1], bd[3]),
            b: continuous_bb,
            c: continuous_cc,
            y: vec![0.; n],
            x: (0f64, 0f64),
        }
    }
    pub fn solve(&mut self, u: &[f64]) -> &[f64] {
        let (x0, x1) = self.x;
        let s = self.m.0 * x0 + self.m.1 * x1;
        self.y.iter_mut().zip(self.c.iter()).for_each(|(y, c)| {
            *y = c * s;
        });
        let v = self.b.iter().zip(u).fold(0., |s, (b, u)| s + b * u);
        self.x.0 = self.q.0 * x0 + self.q.1 * x1 + self.m.1 * v;
        self.x.1 = self.q.2 * x0 + self.q.3 * x1 + self.m.3 * v;
        self.y.as_slice()
    }
}
