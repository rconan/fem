use hdf5::H5Type;
use hdf5_derive;

#[derive(hdf5::H5Type, Clone, PartialEq, Debug)]
#[repr(C)]
struct FEM {
    modelDescription: String,
}

fn main() -> hdf5::Result<()>  {
    let file = hdf5::File::open("examples/modal_state_space_model_2ndOrder.rs.mat")?;
    Ok(())
}

