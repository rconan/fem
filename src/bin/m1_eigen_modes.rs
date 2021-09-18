use fem::FEM;
use geotrans::{Quaternion, Vector};
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

    let fem_path = Path::new("/home/rconan/projects/ns-opm-im/data/20210802_0755_MT_mount_v202104_FSM/static_reduction_model.73.pkl");
    let m1_eigen_modes = (1..=7)
        .map(|sid| {
            println!(
                "Segment #{} - STEP #1: loading the FEM from {:?}",
                sid, fem_path
            );
            let now = Instant::now();
            let mut fem = FEM::from_pickle(fem_path)?;
            //    println!("{}", fem);
            let n_io = (fem.n_inputs(), fem.n_outputs());
            fem.keep_inputs(&[sid - 1]);
            fem.keep_outputs_by(&[sid, 25], |x| {
                x.descriptions.contains(&format!("M1-S{}", sid))
            });
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

            for col in gain.column_iter() {
                let (shape, rbm) = col.as_slice().split_at(n_nodes);
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
            println!(" ... in {}ms", now.elapsed().as_millis());

            println!(
                "Segment #{} - Step #3: RBM gain singular values decomposition",
                sid
            );
            let now = Instant::now();
            let rbm_gain_svd = gain.rows(n_nodes, 6).svd(true, true);
            println!("RBM Singular values:");
            let mut rbm_singular_values = rbm_gain_svd.singular_values.as_slice().to_owned();
            rbm_singular_values.sort_by(|a, b| b.partial_cmp(a).unwrap());
            log::info!(
                "{:#?}",
                rbm_singular_values
                    .iter()
                    .map(|x| x / rbm_singular_values[0])
                    .collect::<Vec<f64>>()
            );
            let v_rbm_t = rbm_gain_svd.v_t.as_ref().unwrap();
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
            log::info!(
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
            let mut m1_u_s: Vec<(Vec<f64>, f64)> = m1_eigen_modes
                .u
                .as_ref()
                .unwrap()
                .column_iter()
                .zip(m1_eigen_modes.singular_values.iter())
                .map(|(u, s)| (u.as_slice().to_owned(), *s))
                .collect();
            m1_u_s.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            {
                let s0 = m1_u_s[0].1.recip();
                m1_u_s.iter_mut().for_each(|(_, s)| {
                    *s *= s0;
                });
            }
            log::info!(
                "M1S Singular values:\n{:#?}",
                m1_u_s.iter().map(|(_, s)| *s).collect::<Vec<f64>>()
            );
            println!(" ... in {}ms", now.elapsed().as_millis());

            let eigen_modes = m1_u_s
                .into_iter()
                .filter_map(|(u, s)| (s > 1e-9).then(|| u))
                .enumerate()
                .flat_map(|(k, u)| {
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
                    u
                })
                .collect::<Vec<f64>>();

            println!("Segment #{} - Step #6: Eigen modes gridding ", sid);
            let now = Instant::now();
            let delta = 8.4 / (n - 1) as f64;
            let n_node = nodes.len() / 3;
            let n_mode = eigen_modes.len() / n_node;
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

            Ok((eigen_modes, data_gridded, n_mode))
        })
        .collect::<Result<Vec<(Vec<f64>, Vec<f64>, usize)>, Box<dyn std::error::Error>>>();

    if let Ok(m1_eigen_modes) = m1_eigen_modes {
        println!("Step #7: Writing eigen modes to ceo file ");
        let now = Instant::now();
        let (modes, n_mode): (Vec<_>, Vec<_>) = m1_eigen_modes
            .into_iter()
            .map(|(_, modes, n_mode)| (modes, n_mode))
            .unzip();
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
    };

    Ok(())
}
