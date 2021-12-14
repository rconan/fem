macro_rules! count_tts {
    () => {0usize};
    ($_head:tt $($tail:tt)*) => {1usize + count_tts!($($tail)*)};
}
fn main() {
    let a = count_tts!(0);
    println!("a: {}",a);
    let a = count_tts!(2 0);
    println!("a: {}",a)
}
