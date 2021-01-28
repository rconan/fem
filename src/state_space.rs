use nalgebra::{DMatrix, DVector, Matrix2, RowDVector, Vector2};
use num_complex::Complex;
use serde::Serialize;
use serde_pickle as pkl;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Serialize, Clone)]
pub enum DiscreteApproximation {
    Forward(f64),
    Backward(f64),
    BiLinear(f64),
    Exponential(f64),
}

#[derive(Serialize)]
pub struct SerdeStateSpace2x2 {
    pub method: DiscreteApproximation,
    pub aa: Vec<f64>,
    pub bb: Vec<f64>,
    pub cc: Vec<f64>,
    pub dd: Option<Vec<f64>>,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}
pub struct StateSpace2x2 {
    pub method: DiscreteApproximation,
    pub aa: Matrix2<f64>,
    pub bb: DMatrix<f64>,
    pub cc: DMatrix<f64>,
    pub dd: Option<DMatrix<f64>>,
    pub x: Vector2<f64>,
    pub y: DVector<f64>,
}
impl StateSpace2x2 {
    pub fn from_second_order(
        method: DiscreteApproximation,
        omega: f64,
        zeta: f64,
        continuous_bb: Option<&[f64]>,
        continuous_cc: Option<&[f64]>,
    ) -> Self {
        let aa = Matrix2::<f64>::new(0., 1., -omega * omega, -2. * omega * zeta);
        let i = Matrix2::<f64>::identity();
        let bb = continuous_bb
            .and_then(|bb| {
                let n = bb.len();
                Some(DMatrix::from_rows(&[
                    RowDVector::zeros(n),
                    RowDVector::from_row_slice(bb),
                ]))
            })
            .or_else(|| Some(DMatrix::from_vec(2, 1, vec![0., 1.])))
            .unwrap();
        let cc = continuous_cc
            .and_then(|cc| {
                let n = cc.len();
                Some(DMatrix::from_columns(&[
                    DVector::from_column_slice(cc),
                    DVector::zeros(n),
                ]))
            })
            .or_else(|| Some(DMatrix::from_vec(1, 2, vec![1., 0.])))
            .unwrap();
        //let n_u = bb.len() / 2;
        let n_y = cc.len() / 2;
        use DiscreteApproximation::*;
        let (aa, bb, cc, dd): (
            Matrix2<f64>,
            DMatrix<f64>,
            DMatrix<f64>,
            Option<DMatrix<f64>>,
        ) = match method {
            Forward(tau) => (i + aa * tau, bb * tau, cc, None),
            Backward(tau) => {
                let iq = (i - aa * tau).try_inverse().unwrap();
                let iqb = iq * &bb * tau;
                let ciq = &cc * iq;
                let dd = &cc * iq * &bb * tau;
                (
                    iq,
                    DMatrix::from_column_slice(iqb.nrows(), iqb.ncols(), iqb.as_slice()),
                    DMatrix::from_column_slice(ciq.nrows(), ciq.ncols(), ciq.as_slice()),
                    Some(dd),
                )
            }
            BiLinear(tau) => {
                let qp = i + aa * (0.5 * tau);
                let iqm = (i - aa * (0.5 * tau)).try_inverse().unwrap();
                let q = qp * iqm;
                let iqb = iqm * &bb * tau.sqrt();
                let ciq = tau.sqrt() * &cc * iqm;
                let dd = &cc * iqm * &bb * (0.5 * tau);
                (
                    q,
                    DMatrix::from_column_slice(iqb.nrows(), iqb.ncols(), iqb.as_slice()),
                    DMatrix::from_column_slice(ciq.nrows(), ciq.ncols(), ciq.as_slice()),
                    Some(dd),
                )
            }
            Exponential(tau) => {
                // https://www.wolframalpha.com/input/?i=inverse+%7B%7B0%2C+1%7D%2C+%7B-x%5E2%2C+-2yx%7D%7D
                // https://www.wolframalpha.com/input/?i=Matrixexp%5B%7B%7B0%2Ct%7D%2C%7B-tx%5E2%2C-2txy%7D%7D%5D
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
                let bd = ia*(ad-i)*bb;
                (ad,  DMatrix::from_column_slice(bd.nrows(), bd.ncols(), bd.as_slice()), cc, None)
            }
        };
        Self {
            method,
            aa,
            bb,
            cc,
            dd,
            x: Vector2::zeros(),
            y: DVector::zeros(n_y),
        }
    }
    pub fn solve(&mut self, u: &[f64]) -> &[f64] {
        let u = DVector::from_column_slice(u);
        self.y = &self.cc * self.x;
        if let Some(ref dd) = self.dd {
            self.y += dd * &u;
        };
        self.x = self.aa * self.x + &self.bb * u;
        self.y.as_slice()
    }
    pub fn to_serde(&self) -> SerdeStateSpace2x2 {
        SerdeStateSpace2x2 {
            method: self.method.clone(),
            aa: self.aa.as_slice().to_owned(),
            bb: self.bb.as_slice().to_owned(),
            cc: self.cc.as_slice().to_owned(),
            dd: self.dd.as_ref().and_then(|x| Some(x.as_slice().to_owned())),
            x: self.x.as_slice().to_owned(),
            y: self.y.as_slice().to_owned(),
        }
    }
}
impl fmt::Display for StateSpace2x2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.dd {
            Some(ref dd) => write!(
                f,
                "State space model:\n - A: {:?}\n - B: {:?}\n - C: {:?}\n - D: {:?}",
                self.aa.shape(),
                self.bb.shape(),
                self.cc.shape(),
                dd.shape(),
            ),
            None => write!(
                f,
                "State space model:\n - A: {:?}\n - B: {:?}\n - C: {:?}",
                self.aa.shape(),
                self.bb.shape(),
                self.cc.shape()
            ),
        }
    }
}
