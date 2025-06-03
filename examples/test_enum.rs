use serde::{Deserialize, Serialize};
use serde_json;
use serde_pickle;
use std::fs::File;

#[derive(Debug, Deserialize, Serialize)]
pub struct Data {
    id: String,
    method: String,
    params: i32,
}
#[derive(Debug, Deserialize, Serialize)]
enum Message {
    Request(Data),
    Response(i32),
}

fn main() {
    let s = r#" {"Request": {"id": "u", "method": "q", "params": 1}} "#;
    let m: Message = serde_json::from_str(s).unwrap();
    println!("{:#?}", m);

    let mut file = File::create("examples/msg.pkl").unwrap();
    serde_pickle::to_writer(&mut file, &m, true).unwrap();

    let file = File::open("examples/pymsg.pkl").unwrap();
    //    let d: Message = serde_pickle::from_reader(file).unwrap();
    let v: serde_pickle::Value = serde_pickle::from_reader(file).unwrap();
    let d: Message = serde_pickle::from_value(v).unwrap();
    println!("{:#?}", d);
}
