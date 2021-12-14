use fem::FEM;
use geotrans::{Quaternion, Vector};
use na::DMatrixSlice;
use nalgebra as na;
use spade::{delaunay::FloatDelaunayTriangulation, HasPosition};
use std::{
    fs::File,
    io::{prelude::*, BufWriter},
    path::Path,
    time::Instant,
};

struct DataPoint {
    point: [f64; 2],
    data: f64,
}
impl HasPosition for DataPoint {
    type Point = [f64; 2];
    fn position(&self) -> [f64; 2] {
        self.point
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let n = 101; // sampling
    let width = 8.4; // segment map width

    let home = std::env::var("HOME")?;
    let fem_path = Path::new(&home);

    let m1_eigen_modes = (1..=7)
        .map(|sid| {
            println!(
                "Segment #{} - STEP #1: loading the FEM from {:?}",
                sid, fem_path.join(
        "projects/ns-opm-im/data/20210802_0755_MT_mount_v202104_FSM/static_reduction_model.73.pkl",
    )
            );
            let now = Instant::now();
            let mut fem = FEM::from_pickle(fem_path.join(
        "projects/ns-opm-im/data/20210802_0755_MT_mount_v202104_FSM/static_reduction_model.73.pkl",
    ))?;
            //    println!("{}", fem);
            let n_io = (fem.n_inputs(), fem.n_outputs());
            fem.keep_inputs(&[sid - 1]);
            fem.keep_outputs(&[sid, 24, 25]);
            fem.filter_outputs_by(&[sid, 25], |x| {
                x.descriptions.contains(&format!("M1-S{}", sid))
            });
            /*fem.keep_outputs_by(&[sid, 25], |x| {
                x.descriptions.contains(&format!("M1-S{}", sid))
            });*/
            println!("{}", fem);
            println!(" ... in {}ms", now.elapsed().as_millis());

            // Static gain
            let gain = fem.reduced_static_gain(n_io).unwrap();
            println!(
                "Segment #{} - Step #2: filtering RBM from mirror surface",
                sid
            );
            let now = Instant::now();
            let nodes = fem.outputs[sid]
                .as_ref()
                .unwrap()
                .get_by(|x| x.properties.location.as_ref().map(|x| x.to_vec()))
                .into_iter()
                .flatten()
                .collect::<Vec<f64>>();
            let n_nodes = nodes.len() / 3;

            let mut m1s_influences = vec![];

            match std::env::var("RBM_FILTER")
                .unwrap_or(String::from("ROTATIONS"))
                .as_str()
            {
                "ROTATIONS" => {
                    for col in gain.column_iter() {
                        let (shape, others) = col.as_slice().split_at(n_nodes);
                        let (_, rbm) = others.split_at(84);
                        let (t_xyz, r_xyz) = rbm.split_at(3);

                        let rxyz_surface = {
                            // 3D rotation of mirror surface nodes
                            let q = Quaternion::unit(r_xyz[2], Vector::k())
                                * Quaternion::unit(r_xyz[1], Vector::j())
                                * Quaternion::unit(r_xyz[0], Vector::i());
                            let trans_nodes: Vec<f64> = nodes
                                .chunks(3)
                                .flat_map(|x| {
                                    let p: Quaternion = Vector::from(x).into();
                                    let w: Quaternion = &q * p * &q.complex_conjugate();
                                    let vv: Vec<f64> = (*Vector::from(w.vector_as_slice())).into();
                                    vv
                                })
                                .collect();
                            trans_nodes
                                .chunks(3)
                                .map(|x| x[2])
                                .zip(nodes.chunks(3).map(|x| x[2]))
                                .map(|(z_rbm, z)| z_rbm - z)
                                .collect::<Vec<f64>>()
                        };
                        // Removing Rx, Ry, Rz and Tz
                        m1s_influences.extend(
                            shape
                                .iter()
                                .zip(rxyz_surface.iter())
                                .map(|(a, r)| a - r - t_xyz[2])
                                .collect::<Vec<f64>>(),
                        );
                    }
                }
                "SHAPES" => {
                    let rxyz_surface: Vec<f64> = gain
                        .column_iter()
                        .flat_map(|col| {
                            let (shape, rbm) = col.as_slice().split_at(n_nodes);
                            let (t_xyz, r_xyz) = rbm.split_at(3);

                            // 3D rotation of mirror surface nodes
                            let q = Quaternion::unit(r_xyz[2], Vector::k())
                                * Quaternion::unit(r_xyz[1], Vector::j())
                                * Quaternion::unit(r_xyz[0], Vector::i());
                            let trans_nodes: Vec<f64> = nodes
                                .chunks(3)
                                .flat_map(|x| {
                                    let p: Quaternion = Vector::from(x).into();
                                    let w: Quaternion = &q * p * &q.complex_conjugate();
                                    let vv: Vec<f64> = (*Vector::from(w.vector_as_slice())).into();
                                    vv
                                })
                                .collect();
                            trans_nodes
                                .chunks(3)
                                .map(|x| x[2])
                                .zip(nodes.chunks(3).map(|x| x[2]))
                                .map(|(z_rbm, z)| z_rbm - z)
                                .collect::<Vec<f64>>()
                        })
                        .collect();
                    let rxyz_surface_svd = na::DMatrix::from_column_slice(
                        nodes.len() / 3,
                        gain.ncols(),
                        &rxyz_surface,
                    )
                    .svd(true, true);
                    println!("Rxyz Singular values:");
                    let mut rxyz_singular_values =
                        rxyz_surface_svd.singular_values.as_slice().to_owned();
                    rxyz_singular_values.sort_by(|a, b| b.partial_cmp(a).unwrap());
                    log::debug!(
                        "{:#?}",
                        rxyz_singular_values
                            .iter()
                            .map(|x| x / rxyz_singular_values[0])
                            .collect::<Vec<f64>>()
                    );
                    let m1_surface_svd = gain.rows(0, n_nodes).svd(true, true);
                    let u = m1_surface_svd.u.as_ref().unwrap();
                    let u_rbm = rxyz_surface_svd.u.as_ref().unwrap();
                    let u_wo_rbm = u - (u_rbm * (u_rbm.transpose() * u));
                    m1s_influences.extend(
                        (u_wo_rbm
                            * na::DMatrix::from_diagonal(&m1_surface_svd.singular_values)
                            * m1_surface_svd.v_t.as_ref().unwrap())
                        .as_slice(),
                    )
                }
                &_ => {}
            }
            println!(" ... in {}ms", now.elapsed().as_millis());

            println!(
                "Segment #{} - Step #3: RBM gain singular values decomposition",
                sid
            );
            let now = Instant::now();
            let rbm_gain_svd = na::DMatrix::from_iterator(
                42,
                gain.ncols(),
                gain.rows(n_nodes, 84).column_iter().flat_map(|col| {
                    col.as_slice()
                        .chunks(12)
                        .flat_map(|x| {
                            x[..6]
                                .iter()
                                .zip(&x[6..])
                                .map(|(cell, mirror)| cell - mirror)
                                .collect::<Vec<f64>>()
                        })
                        .collect::<Vec<f64>>()
                }),
            )
            .svd(true, true);
            //            let rbm_gain_svd = gain.rows(n_nodes + 84, 6).svd(true, true);
            println!("RBM Singular values:");
            let mut rbm_singular_values = rbm_gain_svd.singular_values.as_slice().to_owned();
            rbm_singular_values.sort_by(|a, b| b.partial_cmp(a).unwrap());
            log::debug!(
                "{:#?}",
                rbm_singular_values
                    .iter()
                    .map(|x| x / rbm_singular_values[0])
                    .collect::<Vec<f64>>()
            );
            let v_rbm_t = rbm_gain_svd.v_t.as_ref().unwrap().rows(0, 6);
            println!(" ... in {}ms", now.elapsed().as_millis());

            println!(
                "Segment #{} - Step #4: M1S gain singular values decomposition",
                sid
            );
            let now = Instant::now();
            let mat = na::DMatrix::from_column_slice(
                n_nodes,
                m1s_influences.len() / n_nodes,
                &m1s_influences,
            );
            let m1s_svd = mat.svd(true, true);
            let mut m1s_singular_values = m1s_svd.singular_values.as_slice().to_owned();
            m1s_singular_values.sort_by(|a, b| b.partial_cmp(a).unwrap());
            log::debug!(
                "M1S Singular values:\n{:#?}",
                m1s_singular_values
                    .iter()
                    .map(|x| x / m1s_singular_values[0])
                    .collect::<Vec<f64>>()
            );
            println!(" ... in {}ms", now.elapsed().as_millis());

            println!(
                "Segment #{} - Step #5: Filtering out RBM forces from M1S",
                sid
            );
            let now = Instant::now();
            let v = m1s_svd.v_t.as_ref().unwrap().transpose();
            let v_wo_rbm = &v - (v_rbm_t.transpose() * (v_rbm_t * &v));
            let reconstructed_gain = m1s_svd.u.as_ref().unwrap()
                * na::DMatrix::from_diagonal(&m1s_svd.singular_values)
                * v_wo_rbm.transpose();
            println!(" ... in {}ms", now.elapsed().as_millis());

            println!("Segment #{} - Step #6: Orthonormalization ", sid);
            let now = Instant::now();
            let m1_eigen_modes = reconstructed_gain.svd(true, true);

            // sorting the eigen modes according to singular values
            let mut m1_u_v_s: Vec<(Vec<f64>, Vec<f64>, f64)> = m1_eigen_modes
                .u
                .as_ref()
                .unwrap()
                .column_iter()
                .zip(
                    m1_eigen_modes
                        .v_t
                        .as_ref()
                        .unwrap()
                        .transpose()
                        .column_iter(),
                )
                .zip(m1_eigen_modes.singular_values.iter())
                .map(|((u, v), s)| (u.as_slice().to_owned(), v.as_slice().to_owned(), *s))
                .collect();
            m1_u_v_s.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

            let s0 = m1_u_v_s[0].2.recip();
            {
                let s: Vec<_> = m1_u_v_s.iter().map(|(_, _, s)| s * s0).collect();
                log::debug!("M1S Singular values:\n{:#?}", s);
            }
            println!(" ... in {}ms", now.elapsed().as_millis());

            // truncating the singular values
            let (m1_u, m1_v_s): (Vec<_>, Vec<_>) = m1_u_v_s
                .into_iter()
                .filter_map(|(u, v, s)| (s * s0 > 1e-9).then(|| (u, v, s)))
                .enumerate()
                .map(|(k, (u, v, s))| {
                    let delaunay = triangle_rs::Builder::new()
                        .set_tri_points(
                            nodes
                                .clone()
                                .chunks(3)
                                .map(|x| x[..2].to_vec())
                                .flatten()
                                .collect::<Vec<f64>>(),
                        )
                        .set_switches("Q")
                        .build();

                    let cells = delaunay
                        .triangle_iter()
                        .map(|t| t.iter().fold(0., |a, &i| a + u[i] / 3.));
                    let data = delaunay.triangle_vertex_iter().zip(cells.into_iter());
                    let filename = format!("m1-eigen-modes/s{}_{:03}.svg", sid, k + 1);
                    complot::tri::Heatmap::from((
                        data,
                        Some(complot::Config::new().filename(filename)),
                    ));
                    (u, (v, s))
                })
                .unzip();

            // Computing eigen modes coefficients to forces transformation matrix: VS^-1 (as column-wise `Vec`)
            let eigen_modes: Vec<_> = m1_u.into_iter().flatten().collect();
            let (m1_v, m1_s): (Vec<Vec<f64>>, Vec<f64>) = m1_v_s.into_iter().unzip();
            let n_node = nodes.len() / 3;
            let n_mode = eigen_modes.len() / n_node;
            log::debug!(
                "V: [{:?}] ; gain: [{:?}] ; mode # [{}]",
                (m1_v.len(), m1_v[0].len()),
                gain.shape(),
                n_mode
            );
            let coefs2forces =
                (na::DMatrix::from_iterator(gain.ncols(), n_mode, m1_v.into_iter().flatten())
                    * na::DMatrix::from_diagonal(&na::DVector::from_iterator(
                        n_mode,
                        m1_s.into_iter().map(f64::recip),
                    )))
                .as_slice()
                .to_owned();

            println!("Segment #{} - Step #6: Eigen modes gridding ", sid);
            let now = Instant::now();
            let delta = 8.4 / (n - 1) as f64;
            let mut data_gridded: Vec<f64> = Vec::with_capacity(n * n * n_mode);
            let mut modes = eigen_modes.chunks(n_node);
            for _ in 0..n_mode {
                let mut delaunay = FloatDelaunayTriangulation::with_walk_locate();
                nodes
                    .chunks(3)
                    .zip(modes.next().unwrap().iter())
                    .for_each(|(node, &data)| {
                        delaunay.insert(DataPoint {
                            point: [node[0], node[1]],
                            data,
                        });
                    });
                for i in 0..n {
                    let x = i as f64 * delta - width * 0.5;
                    for j in 0..n {
                        let y = j as f64 * delta - width * 0.5;
                        if let Some(interpolated) = delaunay.nn_interpolation(&[x, y], |dp| dp.data)
                        {
                            data_gridded.push(interpolated)
                        } else {
                            return Err("Interpolation failed.".into());
                        };
                    }
                }
            }
            println!(" ... in {}ms", now.elapsed().as_millis());

            Ok((eigen_modes, coefs2forces, data_gridded, n_mode))
        })
        .collect::<Result<Vec<(Vec<f64>, Vec<f64>, Vec<f64>, usize)>, Box<dyn std::error::Error>>>(
        );

    if let Ok(m1_eigen_modes) = m1_eigen_modes {
        log::debug!(
            "M1 eigen modes [{:?}]",
            (m1_eigen_modes.len(), m1_eigen_modes[0].0.len())
        );
        let (left, right): (Vec<_>, Vec<_>) = m1_eigen_modes
            .into_iter()
            .map(|(eigens, c2f, modes, n_mode)| ((eigens, c2f), (modes, n_mode)))
            .unzip();
        let (modes, n_mode): (Vec<Vec<f64>>, Vec<_>) = right.into_iter().unzip();
        {
            println!("Step #7: Writing gridded eigen modes to ceo file ");
            let now = Instant::now();
            let n_mode_max = n_mode.iter().cloned().fold(0, usize::max);
            let n_max = n_mode_max * n * n;
            let u = modes.into_iter().flat_map(|u| {
                let n_u = u.len();
                if n_u < n_max {
                    let mut v = u;
                    v.append(&mut vec![0f64; n_max - n_u]);
                    v
                } else {
                    u
                }
                .iter()
                .flat_map(|x| x.to_ne_bytes())
                .collect::<Vec<u8>>()
            });

            let mut buffer = (n as i32).to_ne_bytes().to_vec();
            buffer.extend(&width.to_ne_bytes());
            buffer.extend(&7_i32.to_ne_bytes());
            buffer.extend(&(n_mode_max as i32).to_ne_bytes());
            buffer.extend((0i32..7i32).flat_map(|i| i.to_ne_bytes()));
            buffer.extend(u);

            let file = File::create("m1_eigen_modes.ceo")?;
            let mut writer = BufWriter::with_capacity(n * n, file);
            writer.write_all(&buffer)?;
            println!(" ... in {}ms", now.elapsed().as_millis());
        }
        {
            println!("Step #8: Writing eigen modes to bincode file ");
            let now = Instant::now();
            let file = File::create("m1_eigen_modes.bin")?;
            bincode::serialize_into(file, &(left, n_mode))?;
            println!(" ... in {}ms", now.elapsed().as_millis());
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m1_s1_hardpoints() {
        let home = std::env::var("HOME").unwrap();
        let fem_path = Path::new(&home);
        let mut fem = FEM::from_pickle(fem_path.join("projects/ns-opm-im/data/20210802_0755_MT_mount_v202104_FSM/static_reduction_model.73.pkl")).unwrap();
        println!("{}", fem);
        let n_io = (fem.n_inputs(), fem.n_outputs());
        let sid = 1;
        fem.keep_inputs(&[sid - 1]);
        fem.keep_outputs(&[sid, 24, 25]);
        fem.filter_outputs_by(&[sid, 25], |x| {
            x.descriptions.contains(&format!("M1-S{}", sid))
        });
        println!("{}", fem);

        let nodes = fem.outputs[sid]
            .as_ref()
            .unwrap()
            .get_by(|x| x.properties.location.as_ref().map(|x| x.to_vec()))
            .into_iter()
            .flatten()
            .collect::<Vec<f64>>();
        let n_nodes = nodes.len() / 3;

        let gain = fem.reduced_static_gain(n_io).unwrap();
        let surface_gain = gain.rows(0, n_nodes);
        let hardpoints_gain = gain.rows(n_nodes, 84);
        let rbm_gain = gain.rows(n_nodes + 84, 6);
        let rbm_gain_svd = rbm_gain.svd(true, true);

        let mut rbm = vec![0f64; 6];
        rbm[3] = 1e-6;
        let rbm = ((2f64 * na::DVector::new_random(6)
            - na::DVector::from_column_slice(&vec![1f64; 6]))
            * 1e-6)
            .as_slice()
            .to_owned();
        let forces = rbm_gain_svd.v_t.as_ref().unwrap().transpose()
            * na::DMatrix::from_diagonal(&rbm_gain_svd.singular_values.map(f64::recip))
            * rbm_gain_svd.u.as_ref().unwrap().transpose()
            * na::DVector::from_column_slice(&rbm);
        let surface = surface_gain * forces;

        let file = File::open("m1_eigen_modes.bin").unwrap();
        let data: Vec<(Vec<f64>, Vec<f64>)> = bincode::deserialize_from(file).unwrap();
        let (eigens, b2f) = &data[sid - 1];
        let n_mode = eigens.len() / n_nodes;
        let modes = na::DMatrix::from_column_slice(n_nodes, n_mode, eigens);
        let b = modes.transpose() * &surface;
        let surface_from_modes = modes * &b;
        println!("Input/Output RBM:");
        rbm.iter()
            .zip(b.as_slice().iter())
            .for_each(|(i, o)| println!("{:7.3} / {:7.3}", i * 1e6, o * 1e6));

        let forces_from_b = na::DMatrix::from_column_slice(rbm_gain.ncols(), n_mode, b2f) * b;
        let rbm_from_filtered_forces = rbm_gain * &forces_from_b;
        println!("Input/Output RBM:");
        rbm.iter()
            .zip(rbm_from_filtered_forces.as_slice().iter())
            .for_each(|(i, o)| println!("{:7.3} / {:7.3}", i * 1e6, o * 1e6));

        let hpd = hardpoints_gain * &forces_from_b;
        println!(
            "Hardpoints [x10^6:\n{:#?}",
            hpd.as_slice()
                .chunks(12)
                .flat_map(|x| {
                    x[..6]
                        .iter()
                        .zip(&x[6..])
                        .map(|(cell, mirror)| cell - mirror)
                        .collect::<Vec<f64>>()
                })
                .map(|x| x * 1e6)
                .collect::<Vec<f64>>()
        );
    }
    #[test]
    fn m1_s1_rbm() {
        let fem_path = Path::new("/home/ubuntu/projects/ns-opm-im/data/20210802_0755_MT_mount_v202104_FSM/static_reduction_model.73.pkl");
        let mut fem = FEM::from_pickle(fem_path).unwrap();
        //    println!("{}", fem);
        let n_io = (fem.n_inputs(), fem.n_outputs());
        let sid = 1;
        fem.keep_inputs(&[sid - 1]);
        fem.keep_outputs_by(&[sid, 25], |x| {
            x.descriptions.contains(&format!("M1-S{}", sid))
        });
        println!("{}", fem);

        let nodes = fem.outputs[sid]
            .as_ref()
            .unwrap()
            .get_by(|x| x.properties.location.as_ref().map(|x| x.to_vec()))
            .into_iter()
            .flatten()
            .collect::<Vec<f64>>();
        let n_nodes = nodes.len() / 3;

        let gain = fem.reduced_static_gain(n_io).unwrap();
        let surface_gain = gain.rows(0, n_nodes);
        let rbm_gain = gain.rows(n_nodes, 6);
        let rbm_gain_svd = rbm_gain.svd(true, true);

        let mut rbm = vec![0f64; 6];
        rbm[3] = 1e-6;
        let rbm = ((2f64 * na::DVector::new_random(6)
            - na::DVector::from_column_slice(&vec![1f64; 6]))
            * 1e-6)
            .as_slice()
            .to_owned();
        let forces = rbm_gain_svd.v_t.as_ref().unwrap().transpose()
            * na::DMatrix::from_diagonal(&rbm_gain_svd.singular_values.map(f64::recip))
            * rbm_gain_svd.u.as_ref().unwrap().transpose()
            * na::DVector::from_column_slice(&rbm);
        let surface = surface_gain * forces;
        {
            let delaunay = triangle_rs::Builder::new()
                .set_tri_points(
                    nodes
                        .chunks(3)
                        .map(|x| x[..2].to_vec())
                        .flatten()
                        .collect::<Vec<f64>>(),
                )
                .set_switches("Q")
                .build();

            let u = surface.as_slice();
            let cells = delaunay
                .triangle_iter()
                .map(|t| t.iter().fold(0., |a, &i| a + u[i] / 3.))
                .map(|x| x * 1e6);
            let data = delaunay.triangle_vertex_iter().zip(cells.into_iter());
            let filename = "test_m1_s1_rbm_surface.svg";
            complot::tri::Heatmap::from((data, Some(complot::Config::new().filename(filename))));
        }
        let file = File::open("m1_eigen_modes.bin").unwrap();
        let data: Vec<(Vec<f64>, Vec<f64>)> = bincode::deserialize_from(file).unwrap();
        let (eigens, b2f) = &data[sid - 1];
        let n_mode = eigens.len() / n_nodes;
        let modes = na::DMatrix::from_column_slice(n_nodes, n_mode, eigens);
        let b = modes.transpose() * &surface;
        let surface_from_modes = modes * &b;
        {
            let delaunay = triangle_rs::Builder::new()
                .set_tri_points(
                    nodes
                        .clone()
                        .chunks(3)
                        .map(|x| x[..2].to_vec())
                        .flatten()
                        .collect::<Vec<f64>>(),
                )
                .set_switches("Q")
                .build();

            let u = surface_from_modes.as_slice();
            let cells = delaunay
                .triangle_iter()
                .map(|t| t.iter().fold(0., |a, &i| a + u[i] / 3.))
                .map(|x| x * 1e6);
            let data = delaunay.triangle_vertex_iter().zip(cells.into_iter());
            let filename = "test_m1_s1_rbm_surface_from_modes.svg";
            complot::tri::Heatmap::from((data, Some(complot::Config::new().filename(filename))));
        }
        let forces_from_b = na::DMatrix::from_column_slice(rbm_gain.ncols(), n_mode, b2f) * b;
        let rbm_from_filtered_forces = rbm_gain * &forces_from_b;
        println!("Input/Output RBM:");
        rbm.iter()
            .zip(rbm_from_filtered_forces.as_slice().iter())
            .for_each(|(i, o)| println!("{:7.3} / {:7.3}", i * 1e6, o * 1e6));

        let surface_from_filtered_forces = surface_gain * &forces_from_b;
        {
            let delaunay = triangle_rs::Builder::new()
                .set_tri_points(
                    nodes
                        .clone()
                        .chunks(3)
                        .map(|x| x[..2].to_vec())
                        .flatten()
                        .collect::<Vec<f64>>(),
                )
                .set_switches("Q")
                .build();

            let u = surface_from_filtered_forces.as_slice();
            let cells = delaunay
                .triangle_iter()
                .map(|t| t.iter().fold(0., |a, &i| a + u[i] / 3.))
                .map(|x| x * 1e6);
            let data = delaunay.triangle_vertex_iter().zip(cells.into_iter());
            let filename = "test_m1_s1_rbm_surface_from_filtered_forces.svg";
            complot::tri::Heatmap::from((data, Some(complot::Config::new().filename(filename))));
        }
        let surface_from_filtered_forces = surface_gain * &forces_from_b;
        {
            let delaunay = triangle_rs::Builder::new()
                .set_tri_points(
                    nodes
                        .clone()
                        .chunks(3)
                        .map(|x| x[..2].to_vec())
                        .flatten()
                        .collect::<Vec<f64>>(),
                )
                .set_switches("Q")
                .build();

            let u: Vec<_> = surface
                .as_slice()
                .iter()
                .zip(surface_from_filtered_forces.as_slice().iter())
                .map(|(v, w)| v - w)
                .collect();
            let cells = delaunay
                .triangle_iter()
                .map(|t| t.iter().fold(0., |a, &i| a + u[i] / 3.))
                .map(|x| x * 1e6);
            let data = delaunay.triangle_vertex_iter().zip(cells.into_iter());
            let filename = "test_m1_s1_rbm_surface_residual.svg";
            complot::tri::Heatmap::from((data, Some(complot::Config::new().filename(filename))));
        }
    }

    #[test]
    fn m1_s1_st_b2b_static() {
        let home = std::env::var("HOME").unwrap();
        let fem_path = Path::new(&home);
        let mut fem = FEM::from_pickle(fem_path.join("projects/ns-opm-im/data/20210802_0755_MT_mount_v202104_FSM/static_reduction_model.73.pkl")).unwrap();
        //    println!("{}", fem);
        let n_io = (fem.n_inputs(), fem.n_outputs());
        let sid = 1;
        fem.keep_inputs(&[sid - 1]);
        fem.keep_outputs_by(&[sid], |x| x.descriptions.contains(&format!("M1-S{}", sid)));
        println!("{}", fem);

        let nodes = fem.outputs[sid]
            .as_ref()
            .unwrap()
            .get_by(|x| x.properties.location.as_ref().map(|x| x.to_vec()))
            .into_iter()
            .flatten()
            .collect::<Vec<f64>>();
        let n_node = nodes.len() / 3;

        let file = File::open("m1_eigen_modes.bin").unwrap();
        let data: Vec<(Vec<f64>, Vec<f64>)> = bincode::deserialize_from(file).unwrap();
        let (eigens, coefs2forces) = &data[sid - 1];
        let n_eigen_mode = eigens.len() / n_node;

        let gain = fem.reduced_static_gain(n_io).unwrap();

        let coefs = na::DVector::from_column_slice(&[1e-6, 0., 0.]);
        let forces = na::DMatrix::from_column_slice(
            coefs2forces.len() / n_eigen_mode,
            n_eigen_mode,
            &coefs2forces,
        )
        .columns(0, coefs.nrows())
            * &coefs;

        let surface = gain * forces;

        {
            let delaunay = triangle_rs::Builder::new()
                .set_tri_points(
                    nodes
                        .clone()
                        .chunks(3)
                        .map(|x| x[..2].to_vec())
                        .flatten()
                        .collect::<Vec<f64>>(),
                )
                .set_switches("Q")
                .build();

            let u = surface.as_slice();
            let cells = delaunay
                .triangle_iter()
                .map(|t| t.iter().fold(0., |a, &i| a + u[i] / 3.))
                .map(|x| x * 1e6);
            let data = delaunay.triangle_vertex_iter().zip(cells.into_iter());
            let filename = "test_m1_s1_b2b_static.svg";
            complot::tri::Heatmap::from((data, Some(complot::Config::new().filename(filename))));
        }

        let m1_s1_eigens = na::DMatrix::from_column_slice(n_node, n_eigen_mode, eigens);
        let m1_s1_coefs_from_figure = m1_s1_eigens.columns(0, 3).transpose() * surface;
        println!("Eigen modes coefs in/out:");
        m1_s1_coefs_from_figure
            .iter()
            .zip(coefs.iter())
            .for_each(|(o, i)| println!("{:7.3}/{:7.3}", i * 1e6, o * 1e6));
    }

    #[test]
    fn m1_s1_st_b2b_dynamic() {
        let mut fem = FEM::from_env().unwrap();
        //    println!("{}", fem);
        let n_io = (fem.n_inputs(), fem.n_outputs());
        let sid = 1;
        fem.keep_inputs(&[sid - 1]);
        fem.keep_outputs_by(&[sid], |x| x.descriptions.contains(&format!("M1-S{}", sid)));
        println!("{}", fem);

        let nodes = fem.outputs[sid]
            .as_ref()
            .unwrap()
            .get_by(|x| x.properties.location.as_ref().map(|x| x.to_vec()))
            .into_iter()
            .flatten()
            .collect::<Vec<f64>>();
        let n_node = nodes.len() / 3;

        let file = File::open("m1_eigen_modes.bin").unwrap();
        let data: Vec<(Vec<f64>, Vec<f64>)> = bincode::deserialize_from(file).unwrap();
        let (eigens, coefs2forces) = &data[sid - 1];
        let n_eigen_mode = eigens.len() / n_node;

        let gain = fem.static_gain();

        let coefs = na::DVector::from_column_slice(&[1e-6, 0., 0.]);
        let forces = na::DMatrix::from_column_slice(
            coefs2forces.len() / n_eigen_mode,
            n_eigen_mode,
            &coefs2forces,
        )
        .columns(0, coefs.nrows())
            * &coefs;

        let surface = gain * forces;

        {
            let delaunay = triangle_rs::Builder::new()
                .set_tri_points(
                    nodes
                        .clone()
                        .chunks(3)
                        .map(|x| x[..2].to_vec())
                        .flatten()
                        .collect::<Vec<f64>>(),
                )
                .set_switches("Q")
                .build();

            let u = surface.as_slice();
            let cells = delaunay
                .triangle_iter()
                .map(|t| t.iter().fold(0., |a, &i| a + u[i] / 3.))
                .map(|x| x * 1e6);
            let data = delaunay.triangle_vertex_iter().zip(cells.into_iter());
            let filename = "test_m1_s1_b2b_dynamic.svg";
            complot::tri::Heatmap::from((data, Some(complot::Config::new().filename(filename))));
        }

        let m1_s1_eigens = na::DMatrix::from_column_slice(n_node, n_eigen_mode, eigens);
        let m1_s1_coefs_from_figure = m1_s1_eigens.columns(0, 3).transpose() * surface;
        println!("Eigen modes coefs in/out:");
        m1_s1_coefs_from_figure
            .iter()
            .zip(coefs.iter())
            .for_each(|(o, i)| println!("{:7.3}/{:7.3}", i * 1e6, o * 1e6));
    }
}
