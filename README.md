# Giant Magellan Telescope Finite Element Model

The crate is a Rust API for the GMT second order finite element model (FEM).

The FEM is loaded from a zip file which name must be `modal_state_space_model_2ndOrder.zip` and the location of the zip file is given by the environment variable `FEM_REPO`.

The FEM inputs and outputs are dynamically created during compilation from importing input and output tables contained within the zip archive.
This means that each time an application needs a new model, the `gmt-fem` crate need to be recompiled using the `modal_state_space_model_2ndOrder.zip` archive corresponding to the new model.
To force a re-compilation of the `gmt-crate`, the `gmt-crate` library need to be deleted from Rust Cargo cache like this:
```
cargo clean --release -p gmt-fem
```

A summary of the properties of a GMT FEM can be obtained by running the Cargo subcommand
```
cargo gmt-fem
```

The subcommand is installed with 
```
cargo install -f --features clap gmt-fem
```
Run 
```
cargo gmt-fem --help
```
to see the arguments to apply a custom model reduction.

For the reasons explained above, the subcommand need to be re-installed each time it is applied to a new model.

The zip archive `modal_state_space_model_2ndOrder.zip` is generated with the Matlab script `unwrapFEM.m` available in the `tools` directory.
The script uses the Matlab files `modal_state_space_model_2ndOrder.mat` and, if present, `static_reduction_model.mat` to build `modal_state_space_model_2ndOrder.zip`.