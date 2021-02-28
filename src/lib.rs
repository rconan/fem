use nalgebra as na;
use serde_pickle as pkl;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use rayon::prelude::*;

pub mod io;
pub use io::{IOData, IO};

pub mod fem;
pub use fem::{fem_io, FEM};

pub mod state_space;
pub use state_space::{DiscreteApproximation, SerdeStateSpace2x2, StateSpace2x2};

pub mod bilinear;
pub use bilinear::Bilinear;
pub mod exponential;
pub use exponential::Exponential;

#[derive(Debug, Default)]
pub struct DiscreteModalSolver {
    pub u: Vec<f64>,
    pub y: Vec<f64>,
    pub state_space: Vec<Exponential>,
}
impl Iterator for DiscreteModalSolver {
    type Item = ();
    fn next(&mut self) -> Option<Self::Item> {
       let n = self.y.len();
        //        match &self.u {
        let _u_ = &self.u;
        self.y =  self.state_space.par_iter_mut()
                        .fold(
                            || vec![0f64; n],
                            |mut a: Vec<f64>, m| {
                                a.iter_mut().zip(m.solve(_u_)).for_each(|(yc, y)| {
                                    *yc += y;
                                });
                                a
                            },
                        )
                        .reduce(
                            || vec![0f64; n],
                            |mut a: Vec<f64>, b: Vec<f64>| {
                                a.iter_mut().zip(b.iter()).for_each(|(a, b)| {
                                    *a += *b;
                                });
                                a
                            },
                        );
        Some(())
    }
}
/*
macro_rules! controller {
    (U( $($u_name:ident, $u_variant:ident),+), Y ( $($y_name:ident, $y_variant:ident),+)) => {
        #[derive(Debug,Clone)]
        pub enum U {
            $($u_variant(usize,usize)),+
        }
        impl U {
            fn new(pos: usize,i: &fem_io::Inputs) -> Self {
                match i {
                    $(fem_io::Inputs::$u_variant(_) => U::$u_variant(pos,i.len())),+
                }
            }
            pub fn range(&self) -> std::ops::Range<usize> {
                match self {
                    $(U::$u_variant(p,n) => *p..*p+*n),+
                }
            }
        }
        #[derive(Debug)]
        pub enum Y {
            $($y_variant(usize,usize)),+
        }
        impl Y {
            fn new(pos: usize,i: &fem_io::Outputs) -> Self {
                match i {
                    $(fem_io::Outputs::$y_variant(_) => Y::$y_variant(pos,i.len())),+
                }
            }
            pub fn range(&self) -> std::ops::Range<usize> {
                match self {
                    $(Y::$y_variant(p,n) => *p..*p+*n),+
                }
            }
        }
        /*
        impl<T> Controller<T> {
            pub fn new<T>(fem: &FEM, solver: &'a mut DiscreteModalSolver<T>) -> Self {
                Self {
                    $($u_name: fem.inputs.iter().filter_map(|x| {
                        let q = &solver.u[0..1];
                        match x.as_ref() {
                            Some(fem_io::Inputs::$u_variant(_)) => Some(U::$u_variant(& q)),
                            _ => None,
                        }
                    }).next(),)+
                    $($y_name: fem.outputs.iter().filter_map(|x| {
                        match x.as_ref() {
                            Some(fem_io::Outputs::$y_variant(_)) => Some(Y::$y_variant(& solver.u[0..1])),
                            _ => None,
                        }
                    }).next(),)+
                    //solver,
                }
            }
        }*/
    };
}
impl DiscreteModalSolver {
    pub fn new(fem: &mut FEM, sampling_rate: f64) -> Self {
        let mut u: Vec<U> = vec![];
        let mut pos_u = 0_usize;
        for i in fem.inputs.iter().filter_map(|x| x.as_ref()) {
            u.push(U::new(pos_u, i));
            pos_u += i.len();
        }
        let mut y: Vec<Y> = vec![];
        let mut pos_y = 0_usize;
        for o in fem.outputs.iter().filter_map(|x| x.as_ref()) {
            y.push(Y::new(pos_y, o));
            pos_y += o.len();
        }
        let tau = 1. / sampling_rate;
        let modes_2_nodes =
            na::DMatrix::from_row_slice(fem.n_outputs(), fem.n_modes(), &fem.modes2outputs());
        //println!("modes 2 nodes: {:?}", modes_2_nodes.shape());
        let forces_2_modes =
            na::DMatrix::from_row_slice(fem.n_modes(), fem.n_inputs(), &fem.inputs2modes());
        //println!("forces 2 modes: {:?}", forces_2_modes.shape());
        let w = fem.eigen_frequencies_to_radians();
        let zeta = &fem.proportional_damping_vec;
        let state_space: Vec<Exponential> = (0..fem.n_modes())
            .map(|k| {
                let b = forces_2_modes.row(k);
                let c = modes_2_nodes.column(k);
                Exponential::from_second_order(
                    tau,
                    w[k],
                    zeta[k],
                    b.clone_owned().as_slice().to_vec(),
                    c.as_slice().to_vec(),
                )
            })
            .collect();
        Self {
            u,
            y,
            _u_: vec![0f64;pos_u],
            _y_: vec![0f64;pos_y],
            state_space
        }
    }
    pub fn write(&mut self, u: &U, values: &[f64]) -> &mut Self {
        self._u_[u.range()].copy_from_slice(values);
        self
    }
    pub fn read(&self, y: &Y) -> &[f64] {
        &self._y_[y.range()]
    }
}

impl Iterator for DiscreteModalSolver {
    type Item = Vec<f64>;
    fn next(&mut self) -> Option<Self::Item> {
       let n = self._y_.len();
        //        match &self.u {
        let _u_ = &self._u_;
        self._y_ =  self.state_space.par_iter_mut()
                        .fold(
                            || vec![0f64; n],
                            |mut a: Vec<f64>, m| {
                                a.iter_mut().zip(m.solve(_u_)).for_each(|(yc, y)| {
                                    *yc += y;
                                });
                                a
                            },
                        )
                        .reduce(
                            || vec![0f64; n],
                            |mut a: Vec<f64>, b: Vec<f64>| {
                                a.iter_mut().zip(b.iter()).for_each(|(a, b)| {
                                    *a += *b;
                                });
                                a
                            },
                        );
        Some(self._y_.clone())
    }
}
controller!(
    U(
        m1_actuators_segment_1,
        M1ActuatorsSegment1,
        m1_actuators_segment_2,
        M1ActuatorsSegment2,
        m1_actuators_segment_3,
        M1ActuatorsSegment3,
        m1_actuators_segment_4,
        M1ActuatorsSegment4,
        m1_actuators_segment_5,
        M1actuatorsSegment5,
        m1_actuators_segment_6,
        M1actuatorsSegment6,
        m1_actuators_segment_7,
        M1ActuatorsSegment7,
        m1_distributed_windf,
        M1DistributedWindf,
        mc_m2_grav_cs0,
        MCM2GravCS0,
        mc_m2_pzt_s1_f,
        MCM2PZTS1F,
        mc_m2_pzt_s2_f,
        MCM2PZTS2F,
        mc_m2_pzt_s3_f,
        MCM2PZTS3F,
        mc_m2_pzt_s4_f,
        MCM2PZTS4F,
        mc_m2_pzt_s5_f,
        MCM2PZTS5F,
        mc_m2_pzt_s6_f,
        MCM2PZTS6F,
        mc_m2_pzt_s7_f,
        MCM2PZTS7F,
        mc_m2_lcl_force_6f,
        MCM2Lcl6F,
        mc_m2_small_s1_6f,
        MCM2SmallS16F,
        mc_m2_small_s2_6f,
        MCM2SmallS26F,
        mc_m2_small_s3_6f,
        MCM2SmallS36F,
        mc_m2_small_s4_6f,
        MCM2SmallS46F,
        mc_m2_small_s5_6f,
        MCM2SmallS56F,
        mc_m2_small_s6_6f,
        MCM2SmallS66F,
        mc_m2_small_s7_6f,
        MCM2SmallS76F,
        oss_azdrive_f,
        OSSAzDriveF,
        oss_base_6f,
        OSSBASE6F,
        oss_cring_6f,
        OSSCRING6F,
        oss_cell_lcl_6f,
        OSSCellLcl6F,
        oss_eldrive_f,
        OSSElDriveF,
        oss_girdrive_f,
        OSSGIRDriveF,
        oss_gir_6f,
        OSSGIR6F,
        oss_grav_cs0,
        OSSGravCS0,
        oss_harpoint_delta_f,
        OSSHarpointDeltaF,
        oss_m1_lcl_6f,
        OSSM1Lcl6F,
        oss_topend_6f,
        OSSTopEnd6F,
        oss_truss_6f,
        OSSTruss6F
    ),
    Y(
        oss_azdrive_d,
        OSSAzDriveD,
        oss_eldrive_d,
        OSSElDriveD,
        oss_girdrive_d,
        OSSGIRDriveD,
        oss_base_6d,
        OSSBASE6D,
        oss_hardpoint_d,
        OSSHardpointD,
        oss_m1_lcl,
        OSSM1Lcl,
        oss_m1_los,
        OSSM1LOS,
        oss_imus_6d,
        OSSIMUs6d,
        oss_truss_6d,
        OSSTruss6d,
        oss_cell_lcl,
        OSSCellLcl,
        mc_m2_small_s1_6d,
        MCM2SmallS16D,
        mc_m2_pzt_s1_d,
        MCM2PZTS1D,
        mc_m2_small_s2_6d,
        MCM2SmallS26D,
        mc_m2_pzt_s2_d,
        MCM2PZTS2D,
        mc_m2_small_s3_6d,
        MCM2SmallS36D,
        mc_m2_pzt_s3_d,
        MCM2PZTS3D,
        mc_m2_small_s4_6d,
        MCM2SmallS46D,
        mc_m2_pzt_s4_d,
        MCM2PZTS4D,
        mc_m2_small_s5_6d,
        MCM2SmallS56D,
        mc_m2_pzt_s5_d,
        MCM2PZTS5D,
        mc_m2_small_s6_6d,
        MCM2SmallS66D,m
//        mc_m2_pzt_s6_d,
//        MCM2PZTS6D,
        mc_m2_small_s7_6d,
        MCM2SmallS76D,
        mc_m2_pzt_s7_d,
        MCM2PZTS7D,
        mc_m2_lcl_6d,
        MCM2Lcl6D,
        mc_m2_los_6d,
        MCM2LOS6D,
        m1_surfaces_d,
        M1SurfacesD,
        m1_edge_sensors,
        M1EdgeSensors,
        m1_segment_1_axial_d,
        M1Segment1AxialD,
        m1_segment_2_axial_d,
        M1Segment2AxialD,
        m1_segment_3_axial_d,
        M1Segment3AxialD,
        m1_segment_4_axial_d,
        M1Segment4AxialD,
        m1_segment_5_axial_d,
        M1Segment5AxialD,
        m1_segment_6_axial_d,
        M1Segment6AxialD,
        m1_segment_7_axial_d,
        M1Segment7AxialD
    )
);
 */

pub fn load_io<P: AsRef<Path>>(path: P) -> Result<BTreeMap<String, Vec<IO>>, Box<dyn Error>> {
    let f = File::open(path)?;
    let r = BufReader::with_capacity(1_000_000, f);
    Ok(pkl::from_reader(r)?)
}
pub trait ToPickle {
    fn to_pickle<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>>;
}
impl ToPickle for Vec<f64> {
    fn to_pickle<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let mut f = File::create(path)?;
        pkl::to_writer(&mut f, &self, true)?;
        Ok(())
    }
}
impl ToPickle for SerdeStateSpace2x2 {
    fn to_pickle<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let mut f = File::create(path)?;
        pkl::to_writer(&mut f, &self, true)?;
        Ok(())
    }
}
impl ToPickle for Bilinear {
    fn to_pickle<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let mut f = File::create(path)?;
        pkl::to_writer(&mut f, &self, true)?;
        Ok(())
    }
}

pub trait IOTraits {
    fn n(&self) -> usize;
    fn n_on(&self) -> usize;
    fn off(&mut self) -> &mut Self;
    fn on(&mut self, io_name: &str) -> &mut Self;
    fn on_by<F>(&mut self, io_name: &str, pred: F) -> &mut Self
    where
        F: Fn(&IOData) -> bool;
    fn io(&self, io_name: &str) -> Vec<&IOData>;
}

impl IOTraits for BTreeMap<String, Vec<IO>> {
    /// Gets the number of `io`
    fn n(&self) -> usize {
        self.values().fold(0, |a, x| a + x.len())
    }
    /// Gets the number of `io` that are turned on
    fn n_on(&self) -> usize {
        self.values().fold(0, |a, x| {
            a + x.iter().fold(0, |a, x| a + x.is_on() as usize)
        })
    }
    /// Turns off all `io`
    fn off(&mut self) -> &mut Self {
        self.values_mut().for_each(|value| {
            value.iter_mut().for_each(|io| {
                *io = io.clone().switch_off();
            })
        });
        self
    }
    /// Turns on the given `io`
    fn on(&mut self, io_name: &str) -> &mut Self {
        self.get_mut(io_name)
            .expect(&format!("IO {} not found", io_name))
            .iter_mut()
            .for_each(|io| {
                *io = io.clone().switch_on();
            });
        self
    }
    /// Turns on the given `io` for entries that match the give predicate
    fn on_by<F>(&mut self, io_name: &str, pred: F) -> &mut Self
    where
        F: Fn(&IOData) -> bool,
    {
        self.get_mut(io_name)
            .expect(&format!("IO {} not found", io_name))
            .iter_mut()
            .for_each(|io| {
                if let IO::Off(v) = io {
                    if pred(v) {
                        *io = IO::On(v.clone());
                    }
                }
            });
        self
    }
    /// Returns the turned-on entries of the requested `io`
    fn io(&self, io_name: &str) -> Vec<&IOData> {
        self.get(io_name)
            .expect(&format!("Input {} not found", io_name))
            .iter()
            .filter_map(|io| match io {
                IO::On(v) => Some(v),
                IO::Off(_) => None,
            })
            .collect()
    }
}
