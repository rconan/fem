//! A macro to build the fem inputs and outputs enum variants
//!
//! The macro get the variant identifiers from the field names of the structures `fem_inputs` and `fem_outputs` in the file `modal_state_space_model_2ndOrder.rs.mat`.
//! The location of the file is given by the environment variable `FEM_REPO`

use arrow::{array::StringArray, record_batch::RecordBatch};
use parquet::{
    arrow::{ArrowReader, ParquetFileArrowReader},
    file::reader::SerializedFileReader,
    util::cursor::SliceableCursor,
};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal, Span};
use quote::quote;
use std::env;
use std::{fs::File, io::Read, path::Path, sync::Arc};
use zip::ZipArchive;

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("No suitable record in file")]
    NoRecord,
    #[error("No suitable data in file")]
    NoData,
    #[error("Cannot read arrow table")]
    ReadArrow(#[from] arrow::error::ArrowError),
    #[error("Cannot read parquet file")]
    ReadParquet(#[from] parquet::errors::ParquetError),
    #[error("Cannot find archive in zip file")]
    Zip(#[from] zip::result::ZipError),
    #[error("Cannot read zip file content")]
    ReadZip(#[from] std::io::Error),
}

/// Ad-hoc fem crate builder
pub fn ad_hoc_macro(_item: TokenStream) -> TokenStream {
    let (inputs, outputs) = if let Ok(fem_repo) = env::var("FEM_REPO") {
        // Gets the FEM repository
        println!(
            "Building `fem::Inputs` and `fem::Outputs` enums to match inputs/outputs of FEM in {}",
            fem_repo
        );
        // Opens the mat file
        let path = Path::new(&fem_repo);
        let file = if let Ok(val) = File::open(path.join("modal_state_space_model_2ndOrder.zip")) {
            val
        } else {
            return quote!(compile_error!("Cannot find `modal_state_space_model_2ndOrder.zip` in `FEM_REPO`");).into();
        };
        let mut zip_file = if let Ok(val) = zip::ZipArchive::new(file) {
            val
        } else {
            return quote!(compile_error!("`modal_state_space_model_2ndOrder.zip` in `FEM_REPO` is not a valid zip archive");).into();
        };
        (
            // Get the inputs
            {
                let (names, variants) = if let Ok(val) = get_fem_io(&mut zip_file, "in") {
                    val
                } else {
                    return quote!(compile_error!("Cannot find struct `fem_inputs` in `modal_state_space_model_2ndOrder_in.parquet` in `FEM_REPO`");).into();
                };
                build_fem_io(Ident::new("Inputs", Span::call_site()), names, variants)
            },
            // Get the outputs
            {
                let (names, variants) = if let Ok(val) = get_fem_io(&mut zip_file, "out") {
                    val
                } else {
                    return quote!(compile_error!("Cannot find struct `fem_outputs` in `modal_state_space_model_2ndOrder_out.parquet` in `FEM_REPO`");).into();
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
        pub trait FemIo<U> {
            fn position(&self) -> Option<usize>;
        }
    #inputs
    #outputs
    )
    .into()
}

// Read the fields
fn get_fem_io(
    zip_file: &mut ZipArchive<File>,
    fem_io: &str,
) -> Result<(Vec<Literal>, Vec<Ident>), Error> {
    println!("FEM_{}PUTS", fem_io.to_uppercase());
    let mut input_file = zip_file.by_name(&format!(
        "modal_state_space_model_2ndOrder_{}.parquet",
        fem_io
    ))?;
    let mut contents: Vec<u8> = Vec::new();
    input_file.read_to_end(&mut contents)?;

    let mut arrow_reader = ParquetFileArrowReader::new(Arc::new(SerializedFileReader::new(
        SliceableCursor::new(Arc::new(contents)),
    )?));
    if let Ok(input_records) = arrow_reader
        .get_record_reader(2048)?
        .collect::<Result<Vec<RecordBatch>, arrow::error::ArrowError>>()
    {
        let schema = input_records.get(0).unwrap().schema();
        let table = RecordBatch::concat(&schema, &input_records)?;
        let (idx, _) = schema.column_with_name("group").unwrap();
        let data: Option<Vec<&str>> = table
            .column(idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .iter()
            .collect();
        if let Some(mut data) = data {
            data.dedup();
            Ok(data
                .into_iter()
                .enumerate()
                .map(|(k, fem_io)| {
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
                .unzip())
        } else {
            Err(Error::NoData)
        }
    } else {
        Err(Error::NoRecord)
    }
}

// Build the enum
fn build_fem_io(io: Ident, names: Vec<Literal>, variants: Vec<Ident>) -> proc_macro2::TokenStream {
    quote!(
        #(
        #[derive(Debug)]
        pub enum #variants {}
      impl FemIo<#variants> for Vec<Option<#io>> {
              fn position(&self) -> Option<usize>{
          self.iter().filter_map(|x| x.as_ref()).position(|x| if let #io::#variants(_) = x {true} else {false})
              }
      }
    )*

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
                            write!(f,"{:>24}: [{:5}]",stringify!(#variants),self.len())
                        } else {
                            write!(f,"{:>24}: [{:5}] {:?}",stringify!(#variants),self.len(),cs)
                        }}),*
                }
            }
        }
    )
}
