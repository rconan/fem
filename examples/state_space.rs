use fem::{Bilinear, DiscreteApproximation::*, StateSpace2x2};
use nalgebra as na;
use plotters::prelude::*;

fn main() {
    let omega = 12.5; //2.*std::f64::consts::PI*2.;
    let zeta = 2e-2;
    let tau = 0.5e-3;
    let mut ss = StateSpace2x2::from_second_order(BiLinear(tau), omega, zeta, None, None);
    println!("a: {}", ss.aa);
    println!("b: {}", ss.bb);
    println!("c: {}", ss.cc);

    let mut bl = Bilinear::from_second_order(tau, omega, zeta, vec![1.], vec![1.]);
    println!("q: {:?}", bl.q);
    println!("m: {:?}", bl.m);

    let u = vec![0.];
    ss.x = na::Vector2::new(1., 0.);
    let n = 10000;
    let l = (n as f64 * tau) * 1.1;
    /*(0..10000).for_each(|k| {
        let t = tau * k as f64;
        let y = (-0.25 * t).exp() * (0.019988 * (12.5075 * t).sin() + (12.5075 * t).cos());
        println!("{:.3}: {:4?} {:4}", tau * k as f64, ss.solve(&u), y)
    });*/

    let root_drawing_area =
        SVGBackend::new("examples/second_order.svg", (600, 400)).into_drawing_area();

    root_drawing_area.fill(&WHITE).unwrap();

    let mut ctx = ChartBuilder::on(&root_drawing_area)
        .margin_top(40)
        .margin_right(40)
        // enables Y axis, the size is 40 px
        .set_label_area_size(LabelAreaPosition::Left, 40)
        // enable X axis, the size is 40 px
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(0f64..l, -1f64..1f64)
        .unwrap();

    ctx.configure_mesh().draw().unwrap();
    ctx.draw_series(LineSeries::new(
        (0..10000).map(|k| (k as f64 * tau, ss.solve(&u)[0])),
        &BLUE.mix(0.5),
    ))
    .unwrap()
    .label(format!("{:?}", ss.method))
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE.mix(0.5)));
    ctx.draw_series(LineSeries::new(
        (0..10000).map(|k| {
            let t = k as f64 * tau;
            let y = (-0.25 * t).exp() * (0.019988 * (12.5075 * t).sin() + (12.5075 * t).cos());
            (t, y)
        }),
        &RED.mix(0.5),
    ))
    .unwrap()
    .label("Expected")
    .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED.mix(0.5)));
    ctx.configure_series_labels()
        .border_style(&BLACK)
        .background_style(&WHITE.mix(0.5))
        .draw()
        .unwrap();
}
