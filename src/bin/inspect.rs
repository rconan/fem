use gmt_fem::{dos, FEM};
use simple_logger::SimpleLogger;
use std::path::Path;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "FEM Inspector")]
struct Opt {
    /// Root path to FEM repository
    root: String,
    /// FEM repository
    repo: String,
    /// FEM model filename
    #[structopt(short, long, default_value = "modal_state_space_model_2ndOrder.73.pkl")]
    file: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    SimpleLogger::new().init()?;
    let fem_data_path = Path::new(&opt.root).join(&opt.repo);
    println!("FEM INSPECTOR: {:?}", fem_data_path.join(&opt.file));
    match FEM::from_pickle(fem_data_path.join(&opt.file)) {
        Ok(fem) => {
            println!("{}", fem)
        }
        Err(err) => {
            println!("{}", err);
            match dos::SecondOrder::from_pickle(fem_data_path.join(&opt.file).to_str().unwrap()) {
                Ok(fem) => println!("{}", fem),
                Err(err) => print!("{}", err),
            };
        }
    };
    Ok(())
}
