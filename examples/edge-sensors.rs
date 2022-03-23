fn main() -> anyhow::Result<()> {
    let fem = gmt_fem::FEM::from_env()?; //.static_from_env()?;
                                         //println!("{}", fem);
    let locations: Vec<_> = fem.outputs[23]
        .as_ref()
        .unwrap()
        .get_by(|x| Some(x.properties.location.as_ref().unwrap().clone()))
        .into_iter()
        .collect();
    for (k, loc) in locations.iter().enumerate() {
        println!(
            "{:03}, {:+8.4?}, {:+8.4?}, {:+8.4?}",
            k + 1,
            loc[0],
            loc[1],
            loc[2]
        );
    }
    Ok(())
}
