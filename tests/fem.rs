use gmt_fem::{fem_io, Switch, FEM};

#[test]
fn keep_switch() -> anyhow::Result<()> {
    let mut fem = FEM::from_env()?;
    println!("{fem}");
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    fem.keep_inputs(&[10, 11]).keep_outputs(&[18]);
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    println!("{fem}");
    fem.switch_inputs(Switch::Off, Some(&[11]));
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    println!("{fem}");
    let g = fem.reduced_static_gain().unwrap();
    dbg!(g.shape());
    fem.switch_inputs(Switch::On, Some(&[11]));
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    println!("{fem}");
    Ok(())
}
#[test]
fn switch() -> anyhow::Result<()> {
    let mut fem = FEM::from_env()?;
    println!("{fem}");
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    fem.switch_inputs(Switch::Off, None)
        .switch_outputs(Switch::Off, None);
    println!("{fem}");
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    fem.switch_input::<fem_io::MCM2S1VCDeltaF>(Switch::On)
        .unwrap()
        .switch_output::<fem_io::MCM2S1VCDeltaD>(Switch::On)
        .unwrap();
    println!("{fem}");
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    let g = fem.reduced_static_gain().unwrap();
    dbg!(g.shape());
    fem.switch_inputs(Switch::On, None)
        .switch_outputs(Switch::On, None);
    println!("{fem}");
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    Ok(())
}
#[test]
fn switch_by_name() -> anyhow::Result<()> {
    let mut fem = FEM::from_env()?;
    println!("{fem}");
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    fem.switch_inputs(Switch::Off, None)
        .switch_outputs(Switch::Off, None);
    println!("{fem}");
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    fem.switch_inputs_by_name(vec!["MC_M2_S1_VC_delta_F"], Switch::On)
        .unwrap()
        .switch_outputs_by_name(vec!["MC_M2_S1_VC_delta_D"], Switch::On)
        .unwrap();
    println!("{fem}");
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    // let g = fem.reduced_static_gain().unwrap();
    // dbg!(g.shape());
    fem.switch_inputs(Switch::On, None)
        .switch_outputs(Switch::On, None);
    println!("{fem}");
    println!("Size: [{},{}]", fem.n_inputs(), fem.n_outputs());
    Ok(())
}
