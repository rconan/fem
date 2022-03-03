//! A macro to build the fem inputs and outputs enum variants
//!
//! The macro get the variant identifiers from the field names of the structures `fem_inputs` and `fem_outputs` in the file `modal_state_space_model_2ndOrder.rs.mat`.
//! The location of the file is given by the environment variable `FEM_REPO`

use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal, Span};
use quote::quote;
use std::env;
use std::path::Path;

/// Ad-hoc fem crate builder
pub fn ad_hoc_macro(_item: TokenStream) -> TokenStream {
    let (inputs, outputs) = if let Ok(fem_repo) = env::var("FEM_REPO") {
        // Gets the FEM repository
        println!(
            "Building `fem::Inputs` and `fem::Outputs` enums to match inputs/outputs of FEM in {}",
            fem_repo
        );
        // Opens the mat file
        let file = Path::new(&fem_repo).join("modal_state_space_model_2ndOrder.rs.mat");
        let h5 = if let Ok(val) = hdf5::File::open(file) {
            val
        } else {
            return quote!(compile_error!("Cannot find `modal_state_space_model_2ndOrder.rs.mat` in `FEM_REPO`");).into();
        };
        (
            // Get the inputs
            {
                let (names, variants) = if let Ok(val) = get_fem_io(&h5, "fem_inputs") {
                    val
                } else {
                    return quote!(compile_error!("Cannot find struct `fem_inputs` in `modal_state_space_model_2ndOrder.rs.mat` in `FEM_REPO`");).into();
                };
                build_fem_io(Ident::new("Inputs", Span::call_site()), names, variants)
            },
            // Get the outputs
            {
                let (names, variants) = if let Ok(val) = get_fem_io(&h5, "fem_outputs") {
                    val
                } else {
                    return quote!(compile_error!("Cannot find struct `fem_outputs` in `modal_state_space_model_2ndOrder.rs.mat` in `FEM_REPO`");).into();
                };
                build_fem_io(Ident::new("Outputs", Span::call_site()), names, variants)
            },
        )
    } else {
        println!("`FEM_REPO` environment variable is not set, using dummies instead.");
        (
            {
                let (names, variants): (Vec<_>, Vec<_>) =
                    ["Rodolphe", "Rodrigo", "Christoph", "Henry"]
                        .iter()
                        .map(|&v| (Literal::string(v), Ident::new(v, Span::call_site())))
                        .unzip();
                build_fem_io(Ident::new("Inputs", Span::call_site()), names, variants)
            },
            {
                let (names, variants): (Vec<_>, Vec<_>) =
                    ["Conan", "Romano", "Dribusch", "Fitzpatrick"]
                        .iter()
                        .map(|&v| (Literal::string(v), Ident::new(v, Span::call_site())))
                        .unzip();
                build_fem_io(Ident::new("Outputs", Span::call_site()), names, variants)
            },
        )
    };

    quote!(
    #inputs
    #outputs
    )
    .into()
}

// Read the fields
fn get_fem_io(h5: &hdf5::File, fem_io: &str) -> Result<(Vec<Literal>, Vec<Ident>), hdf5::Error> {
    println!("{}", fem_io.to_uppercase());
    h5.group(fem_io)?.attr("MATLAB_fields")?.read_raw().map(
        |data: Vec<hdf5::types::VarLenArray<hdf5::types::FixedAscii<1>>>| {
            data.into_iter()
                .enumerate()
                .map(|(k, v)| {
                    let fem_io = v.iter().map(|x| x.as_str()).collect::<String>();
                    let fem_io_rsed = fem_io
                        .split("_")
                        .map(|s| {
                            let (first, last) = s.split_at(1);
                            first.to_uppercase() + last
                        })
                        .collect::<String>();
                    println!(" #{:03}: {:>32} <=> {:<32}", k, fem_io, fem_io_rsed);
                    (
                        Literal::string(&fem_io),
                        Ident::new(&fem_io_rsed, Span::call_site()),
                    )
                })
                .unzip()
        },
    )
}

// Build the enum
fn build_fem_io(io: Ident, names: Vec<Literal>, variants: Vec<Ident>) -> proc_macro2::TokenStream {
    quote!(

        #(pub enum #variants {};)*

        #[derive(Deserialize, Debug, Clone)]
        pub enum #io {
            #(#[doc = #names]
            #[serde(rename = #names)]
            #variants(Vec<IO>)),*
        }
        impl #io {
            pub fn len(&self) -> usize {
                match self {
                    #(#io::#variants(io) => {
                        io.iter().fold(0,|a,x| a + x.is_on() as usize)
                    }),*
                }
            }
            pub fn get_by<F,T>(&self, pred: F) -> Vec<T>
                where
                F: Fn(&IOData) -> Option<T> + Copy,
            {
                match self {
                    #(#io::#variants(io) => {
                        io.iter().filter_map(|x| x.get_by(pred)).collect()
                    }),*
                }
            }
        }
        impl std::ops::Deref for #io {
            type Target = [IO];
            fn deref(&self) -> &Self::Target {
                match self {
                    #(#io::#variants(io) => io),*
                }
            }
        }
        impl std::ops::DerefMut for #io {
            fn deref_mut(&mut self) -> &mut Self::Target {
                match self {
                    #(#io::#variants(io) => io),*
                }
            }
        }
        impl std::fmt::Display for #io {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#io::#variants(io) => {
                        let mut cs: Vec<_> = io.iter().filter_map(|x| match x {
                            IO::On(data) => data.properties.cs_label.as_ref(),
                            IO::Off(_) => None
                        }).collect();
                        cs.sort();
                        cs.dedup();
                        if cs.len()>1 {
                            write!(f,"{:>24}: [{:5}]",#names,self.len())
                        } else {
                            write!(f,"{:>24}: [{:5}] {:?}",#names,self.len(),cs)
                        }}),*
                }
            }
        }
    )
}

/// Implement the `MatchFem` trait
pub fn match_maker_macro(_item: TokenStream) -> TokenStream {
    let (inputs, outputs) = if let Ok(fem_repo) = env::var("FEM_REPO") {
        // Gets the FEM repository
        println!(
            "Implementing `MatchFEM` trait to match `dosio` to FEM in {}",
            fem_repo
        );
        // Opens the mat file
        let file = Path::new(&fem_repo).join("modal_state_space_model_2ndOrder.rs.mat");
        let h5 = if let Ok(val) = hdf5::File::open(file) {
            val
        } else {
            return quote!(compile_error!("Cannot find `modal_state_space_model_2ndOrder.rs.mat` in `FEM_REPO`");).into();
        };
        (
            // Get the inputs
            {
                let variants = if let Ok(val) = get_fem_ident(&h5, "fem_inputs") {
                    val
                } else {
                    return quote!(compile_error!("Cannot find struct `fem_inputs` in `modal_state_space_model_2ndOrder.rs.mat` in `FEM_REPO`");).into();
                };
                match_inputs(variants)
            },
            // Get the outputs
            {
                let variants = if let Ok(val) = get_fem_ident(&h5, "fem_outputs") {
                    val
                } else {
                    return quote!(compile_error!("Cannot find struct `fem_outputs` in `modal_state_space_model_2ndOrder.rs.mat` in `FEM_REPO`");).into();
                };
                match_outputs(variants)
            },
        )
    } else {
        println!("`FEM_REPO` environment variable is not set, using dummies instead.");
        (
            {
                match_inputs(
                    ["Rodolphe", "Rodrigo", "Christoph", "Henry"]
                        .iter()
                        .map(|&v| Ident::new(v, Span::call_site()))
                        .collect(),
                )
            },
            {
                match_outputs(
                    ["Conan", "Romano", "Dribusch", "Fitzpatrick"]
                        .iter()
                        .map(|&v| Ident::new(v, Span::call_site()))
                        .collect(),
                )
            },
        )
    };

    quote!(
pub trait MatchFEM {
    fn match_fem_inputs(&self, fem_inputs: &fem::fem_io::Inputs) -> Option<Vec<crate::io::IO>>;
    fn match_fem_outputs(&self, fem_outputs: &fem::fem_io::Outputs) -> Option<Vec<crate::io::IO>>;
}
        impl<T: Debug> MatchFEM for IO<T> {
    #inputs
	    #outputs
	}
    )
    .into()
}

// Read the fields
fn get_fem_ident(h5: &hdf5::File, fem_io: &str) -> Result<Vec<Ident>, hdf5::Error> {
    h5.group(fem_io)?.attr("MATLAB_fields")?.read_raw().map(
        |data: Vec<hdf5::types::VarLenArray<hdf5::types::FixedAscii<1>>>| {
            data.into_iter()
                .map(|v| {
                    let fem_io = v.iter().map(|x| x.as_str()).collect::<String>();
                    let fem_io_rsed = fem_io
                        .split("_")
                        .map(|s| {
                            let (first, last) = s.split_at(1);
                            first.to_uppercase() + last
                        })
                        .collect::<String>();
                    Ident::new(&fem_io_rsed, Span::call_site())
                })
                .collect()
        },
    )
}

fn match_inputs(variants: Vec<Ident>) -> proc_macro2::TokenStream {
    quote!(
            /// Matches a FEM input to a DOS `IO` returning the FEM input value
            fn match_fem_inputs(&self, fem_inputs: &fem::fem_io::Inputs) -> Option<Vec<crate::io::IO>> {
                match (self,fem_inputs) {
                    #((IO::#variants{data: _}, fem::fem_io::Inputs::#variants(v)) => {
                        Some(v.clone())},)*
                    (_, _) => None,
                }
            }
    )
}
fn match_outputs(variants: Vec<Ident>) -> proc_macro2::TokenStream {
    quote!(
            /// Matches a FEM output to a DOS `IO` returning the FEM output value
            fn match_fem_outputs(&self, fem_outputs: &fem::fem_io::Outputs) -> Option<Vec<crate::io::IO>> {
                match (self,fem_outputs) {
                    #((IO::#variants{data: _}, fem::fem_io::Outputs::#variants(v)) => Some(v.clone()),)*
                        (_, _) => None,
                }
            }
    )
}
