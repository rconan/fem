use complot as plt;
use complot::TriPlot;
use fem;
use fem::IOTraits;
use rstar::RTree;
use serde::{Deserialize, Serialize};
use serde_pickle as pkl;
use spade::delaunay::FloatDelaunayTriangulation;
use std::fs::File;
use std::io::BufWriter;
use std::time::Instant;

#[allow(dead_code)]
enum Segment {
    Outer,
    Center,
}

#[derive(Serialize, Deserialize)]
pub struct BendingModes {
    nodes: Vec<f64>, // [x0,y0,x1,y1,...]
    modes: Vec<f64>,
}

fn main() {
    let seg_type = Segment::Center;
    println!("Loading M1 thermal data ...");
    let now = Instant::now();
    let (nodes, bending) = match seg_type {
        Segment::Outer => {
            let mut outputs = fem::load_io("examples/m1_s1_outputTable.pkl").unwrap();
            println!("outputs #: {}", outputs.n());
            outputs.off().on("M1_segment_1_axial_d");
            println!("nodes #: {}", outputs.n_on());
            let nodes: Vec<f64> = outputs
                .io("M1_segment_1_axial_d")
                .iter()
                .flat_map(|x| x.properties.location.as_ref().unwrap()[0..2].to_vec())
                .collect();
            let bm_file = File::open(
                "/home/rconan/Documents/GMT/BendingModes/bending_modes-rs/bending_modes_OA.pkl",
            )
            .unwrap();
            let bending: BendingModes = pkl::from_reader(bm_file).unwrap();
            println!("bending modes nodes: {}", bending.nodes.len() / 2);
            (nodes, bending)
        }
        Segment::Center => {
            let mut outputs = fem::load_io("examples/m1_s7_outputTable.pkl").unwrap();
            println!("outputs #: {}", outputs.n());
            outputs.off().on("M1_segment_7_axial_d");
            println!("nodes #: {}", outputs.n_on());
            let nodes: Vec<f64> = outputs
                .io("M1_segment_7_axial_d")
                .iter()
                .flat_map(|x| x.properties.location.as_ref().unwrap()[0..2].to_vec())
                .collect();
            let bm_file = File::open(
                "/home/rconan/Documents/GMT/BendingModes/bending_modes-rs/bending_modes_CS.pkl",
            )
            .unwrap();
            let bending: BendingModes = pkl::from_reader(bm_file).unwrap();
            println!("bending nodes: {}", bending.nodes.len() / 2);
            (nodes, bending)
        }
    };
    println!("... done in {:.3}s", now.elapsed().as_secs_f64());
    let n_node = nodes.len() / 2;
    let n_bm = bending.modes.len() / n_node;
    println!("Bending modes #: {}", n_bm);

    let mut tree = RTree::bulk_load(
        bending
            .nodes
            .chunks(2)
            .map(|yx| [yx[1], yx[0]])
            .collect::<Vec<[f64; 2]>>(),
    );
    let sorted_idx: Vec<usize> = nodes
        .chunks(2)
        .map(|xy| {
            let trupti = tree.nearest_neighbor(&[xy[0], xy[1]]).unwrap();
            let idx = bending
                .nodes
                .chunks(2)
                .position(|xy| xy[0] == trupti[1] && xy[1] == trupti[0])
                .unwrap();
            idx
        })
        .collect();

    println!("Sorted idx: {}", sorted_idx.len());
    let sorted_nodes: Vec<_> = sorted_idx
        .iter()
        .flat_map(|&i| {
            let yx: Vec<f64> = bending.nodes.chunks(2).nth(i).unwrap().to_vec();
            vec![yx[1], yx[0]]
        })
        .collect();
    let sorted_modes: Vec<_> = bending
        .modes
        .chunks(n_node)
        .flat_map(|mode| sorted_idx.iter().map(|&i| mode[i]).collect::<Vec<f64>>())
        .collect();
   let sorted_bm = BendingModes {
        nodes: sorted_nodes,
        modes: sorted_modes,
    };
    println!("FEM nodes: {:?}", &nodes[0..10]);
    println!("BM nodes : {:?}", &sorted_bm.nodes[0..10]);

    let mut tri = FloatDelaunayTriangulation::with_walk_locate();
    nodes.chunks(2).for_each(|xy| {
        tri.insert([xy[0], xy[1]]);
    });
    let fig = plt::png_canvas("examples/sorted_bending_modes.png");
    let mut ax = plt::chart([-4.5, 4.5, -4.5, 4.5], &fig);
    let (x, y): (Vec<f64>, Vec<f64>) = nodes.chunks(2).map(|xy| (xy[0], xy[1])).unzip();
    let surface: Vec<f64> = sorted_bm.modes.chunks(n_node).take(1).flat_map(|x| x.to_vec()).collect();
    tri.map(&x, &y, &surface.as_slice(), &mut ax);


    let filename = format!(
        "examples/bending_modes_{}.pkl",
        match seg_type {
            Segment::Outer => "OA",
            Segment::Center => "CS",
        }
    );
    let file = File::create(filename).unwrap();
    let mut wtr = BufWriter::with_capacity(1_000_000, file);
    pkl::to_writer(&mut wtr, &sorted_bm, true).unwrap();
}
