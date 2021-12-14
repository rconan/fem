use fem::FEM;
use std::path::Path;

fn main() {
    let fem_data_path = Path::new("/home/rconan/projects/dos/data").join("20210225_1447_MT_mount_v202102_ASM_wind2");
    let mut fem = FEM::from_pickle(
        fem_data_path.join("modal_state_space_model_2ndOrder.73.pkl"),
    ).unwrap();

}
