use gmt_fem::FEM;
use std::fs::File;

fn main() -> anyhow::Result<()> {
    let fem = FEM::from_env()?;
    println!("{fem}");

    /*     let nodes: Vec<_> = fem
    .inputs
    .iter()
    .flat_map(|input| {
        input
            .as_ref()
            .map(|i| i.get_by(|i| i.properties.location.clone()))
            .unwrap()
    })
    .collect(); */

    let nodes: Vec<_> = fem.inputs[0]
        .as_ref()
        .map(|i| i.get_by(|i| i.properties.location.clone()))
        .unwrap();

    let mut file = File::create("cfd_nodes.pkl")?;
    serde_pickle::to_writer(&mut file, &nodes, Default::default())?;

    Ok(())
}
