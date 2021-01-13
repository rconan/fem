use complot as plt;
use complot::TriPlot;
use datax;
use fem;
use fem::IOTraits;
//use gmt_m1;
use nalgebra as na;
use plotters::prelude::*;
use spade::delaunay::FloatDelaunayTriangulation;
use std::time::Instant;

// FANS coordinates [x1,y1,x2,y2,...]
const OA_FANS: [f64; 28] = [
    -3.3071, -0.9610, -2.1084, -2.6908, -1.2426, -1.5376, -1.2426, 0., 3.3071, -0.9610, 2.1084,
    -2.6908, 1.2426, -1.5376, 1.2426, 0., -3.3071, 0.9610, -2.1084, 2.6908, -1.2426, 1.5376,
    3.3071, 0.9610, 2.1084, 2.6908, 1.2426, 1.5376,
];
const CS_FANS: [f64; 28] = [
    -3.3071, -1.2610, -2.1084, -2.6908, -1.2426, -1.5376, -4., 0., 3.3071, -1.2610, 2.1084,
    -2.6908, 1.2426, -1.5376, 4., 0., -3.3071, 1.2610, -2.1084, 2.6908, -1.2426, 1.5376, 3.3071,
    1.2610, 2.1084, 2.6908, 1.2426, 1.5376,
];

#[allow(dead_code)]
enum Segment {
    Outer,
    Center,
}

fn main() {
    let mut inputs = fem::load_io("examples/20200319_Rodrigo_k6rot_100000_c_inputs.pkl").unwrap();
    println!("inputs #: {}", inputs.n());

    println!("Loading M1 thermal data ...");
    let now = Instant::now();
    //let m1 = gmt_m1::Mirror::default();
    let seg_type = Segment::Outer;
    let (actuators_coords, nodes, m1_cores, n_core, fans, stiffness, stiffness_size) = {
        match seg_type {
            Segment::Outer => {
                inputs.off().on_by("M1_actuators_segment_1", |x| {
                    x.properties.components.as_ref().unwrap()[2] == -1f64
                        && x.properties.components.as_ref().unwrap()[5] == 1f64
                });
                println!("actuators #: {}", inputs.n_on());
                let actuators_coords: Vec<f64> = inputs
                    .io("M1_actuators_segment_1")
                    .iter()
                    .flat_map(|x| x.properties.location.as_ref().unwrap()[0..2].to_vec())
                    .collect();
                let mut outputs = fem::load_io("examples/m1_s1_outputTable.pkl").unwrap();
                println!("outputs #: {}", outputs.n());
                outputs.off().on("M1_segment_1_axial_d");
                println!("nodes #: {}", outputs.n_on());
                let nodes: Vec<f64> = outputs
                    .io("M1_segment_1_axial_d")
                    .iter()
                    .flat_map(|x| x.properties.location.as_ref().unwrap()[0..2].to_vec())
                    .collect();
                let (m1_cores, m1_cores_size) =
                    datax::load_mat("examples/m1_cores_locations.mat", "m1_s1_core_xy").unwrap();
                println!("M1 cores size: {:?}", m1_cores_size);
                let n_core = m1_cores_size[0];
                let (stiffness, stiffness_size) =
                    datax::load_mat("examples/stiffnesses.mat", "m1_s1_stiffness").unwrap();
                println!("Stiffness: {:?}", &stiffness[0..10]);
                (
                    actuators_coords,
                    nodes,
                    m1_cores,
                    n_core,
                    OA_FANS,
                    stiffness,
                    stiffness_size,
                )
            }
            Segment::Center => {
                inputs.off().on_by("M1_actuators_segment_7", |x| {
                    x.properties.components.as_ref().unwrap()[2] == -1f64
                        && x.properties.components.as_ref().unwrap()[5] == 1f64
                });
                println!("actuators #: {}", inputs.n_on());
                let actuators_coords: Vec<f64> = inputs
                    .io("M1_actuators_segment_7")
                    .iter()
                    .flat_map(|x| x.properties.location.as_ref().unwrap()[0..2].to_vec())
                    .collect();
                let mut outputs = fem::load_io("examples/m1_s7_outputTable.pkl").unwrap();
                println!("outputs #: {}", outputs.n());
                outputs.off().on("M1_segment_7_axial_d");
                println!("nodes #: {}", outputs.n_on());
                let nodes: Vec<f64> = outputs
                    .io("M1_segment_7_axial_d")
                    .iter()
                    .flat_map(|x| x.properties.location.as_ref().unwrap()[0..2].to_vec())
                    .collect();
                let (m1_cores, m1_cores_size) =
                    datax::load_mat("examples/m1_cores_locations.mat", "m1_s7_core_xy").unwrap();
                println!("M1 cores size: {:?}", m1_cores_size);
                let n_core = m1_cores_size[0];
                let (stiffness, stiffness_size) =
                    datax::load_mat("examples/stiffnesses.mat", "m1_s7_stiffness").unwrap();
                (
                    actuators_coords,
                    nodes,
                    m1_cores,
                    n_core,
                    CS_FANS,
                    stiffness,
                    stiffness_size,
                )
            }
        }
    };
    println!("... done in {:.3}s", now.elapsed().as_secs_f64());
    println!("Stiffness: {:?}", stiffness_size);
    let n_node = nodes.len() / 2;
    println!("# of nodes: {}", n_node);

    // Uniform temperature distribution
    let peak_temperature = 30e-3;
    let sigma = 5e-2;
    let temperature_field: Vec<f64> = nodes
        .chunks(2)
        .map(|xy| {
            let (x, y) = (xy[0], xy[1]);
            (0..n_core).fold(0., |temp, i| {
                let (x_core, y_core) = (m1_cores[i], m1_cores[i + n_core]);
                let r = (x - x_core).hypot(y - y_core);
                let red = -0.5 * (r / sigma).powf(2.);
                temp + peak_temperature * red.exp()
            })
        })
        .collect();
    println!("field: {}", temperature_field.len());

    println!("Computing surface deformation ...");
    let now = Instant::now();
    let mat_stiffness = na::DMatrix::from_iterator(
        n_node,
        n_core,
        stiffness
            .chunks(stiffness_size[0])
            .flat_map(|x| x[0..n_node].to_vec()),
    );
    //    let core_temperature = na::DVector::from_element(n_core, 1.);
    let core_temperature = (na::DVector::new_random(n_core) * 2. - na::DVector::from_element(n_core,1.))*30e-3;
    let surface = mat_stiffness * core_temperature;
    println!("... done in {:.3}s", now.elapsed().as_secs_f64());
    println!("Surface: {:?}", &surface.as_slice()[0..10]);

    let mut tri = FloatDelaunayTriangulation::with_walk_locate();
    nodes.chunks(2).for_each(|xy| {
        tri.insert([xy[0], xy[1]]);
    });

    let fig = BitMapBackend::new("examples/temperature.distribution.png", (4096, 2048))
        .into_drawing_area();
    fig.fill(&WHITE).unwrap();
    let (temp_fig, surf_fig) = fig.split_horizontally((50).percent_width());
    let mut temp_ax = plt::chart([-4.5, 4.5, -4.5, 4.5], &temp_fig);
    let (x, y): (Vec<f64>, Vec<f64>) = nodes.chunks(2).map(|xy| (xy[0], xy[1])).unzip();
    tri.map(&x, &y, &temperature_field, &mut temp_ax);
    temp_ax
        .draw_series(
            (0..n_core).map(|i| Circle::new((m1_cores[i], m1_cores[i + n_core]), 20, &WHITE)),
        )
        .unwrap();
    temp_ax
        .draw_series(
            fans.chunks(2)
                .map(|xy| TriangleMarker::new((xy[0], xy[1]), 30, &WHITE)),
        )
        .unwrap();
    temp_ax
        .draw_series(
            actuators_coords
                .chunks(2)
                .map(|xy| Circle::new((xy[0], xy[1]), 10, BLACK.filled())),
        )
        .unwrap();

    let mut surf_ax = plt::chart([-4.5, 4.5, -4.5, 4.5], &surf_fig);
    tri.map(&x, &y, &surface.as_slice(), &mut surf_ax);
}
