use fem::{Bilinear, Exponential};

fn main() {
    let bil = Bilinear::from_second_order(0.5e-3, 10f64, 0.5 / 100., vec![1f64], vec![1f64]);
    println!("{:#?}", bil);
    let exp = Exponential::from_second_order(0.5e-3, 10f64, 0.5 / 100., vec![1f64], vec![1f64]);
    println!("{:#?}", exp);
}
