//! A macro to build the fem inputs and outputs enum variants
//!
//! The macro get the variant identifiers from the field names of the structures `fem_inputs` and `fem_outputs` in the file `modal_state_space_model_2ndOrder.rs.mat`.
//! The location of the file is given by the environment variable `FEM_REPO`

use arrow::array::{LargeStringArray, StringArray};
use arrow::record_batch::RecordBatchReader;
use bytes::Bytes;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Literal, Span};
use quote::quote;
use std::env;
use std::{fs::File, io::Read, path::Path};
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
        let mut zip_file = zip::ZipArchive::new(file).expect("Zip archive");
        (
            // Get the inputs
            {
                let (names, variants) = get_fem_io(&mut zip_file, "in")
                    .map_err(|e| {
                        println!("{e}");
                        e
                    })
                    .expect("Get FEM Inputs");
                let io = build_fem_io(
                    Ident::new("Inputs", Span::call_site()),
                    names.clone(),
                    variants.clone(),
                );
                quote!(
                    impl TryFrom<String> for Box<dyn GetIn> {
                       type Error = FemError;
                       fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
                           match value.as_str() {
                               #(#names => Ok(Box::new(SplitFem::<#variants>::new()))),*,
                               _ => Err(FemError::Convert(value)),
                           }
                       }
                    }
                    #io
                )
            },
            // Get the outputs
            {
                let (names, variants) = get_fem_io(&mut zip_file, "out")
                    .map_err(|e| {
                        println!("{e}");
                        e
                    })
                    .expect("Get FEM Outputs");
                let io = build_fem_io(
                    Ident::new("Outputs", Span::call_site()),
                    names.clone(),
                    variants.clone(),
                );
                quote!(
                    impl TryFrom<String> for Box<dyn GetOut> {
                       type Error = FemError;
                       fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
                           match value.as_str() {
                               #(#names => Ok(Box::new(SplitFem::<#variants>::new()))),*,
                               _ => Err(FemError::Convert(value)),
                           }
                       }
                    }
                    #io
                )
            },
        )
    } else {
        println!("`FEM_REPO` environment variable is not set, using dummies instead.");
        (
            {
                let (names, variants): (Vec<_>, Vec<_>) =
                    ["OSSAzDriveTorque", "OSSElDriveTorque", "OSSRotDriveTorque"]
                        .iter()
                        .map(|&v| (Literal::string(v), Ident::new(v, Span::call_site())))
                        .unzip();
                build_fem_io(Ident::new("Inputs", Span::call_site()), names, variants)
            },
            {
                let (names, variants): (Vec<_>, Vec<_>) = [
                    "OSSAzEncoderAngle",
                    "OSSElEncoderAngle",
                    "OSSRotEncoderAngle",
                ]
                .iter()
                .map(|&v| (Literal::string(v), Ident::new(v, Span::call_site())))
                .unzip();
                build_fem_io(Ident::new("Outputs", Span::call_site()), names, variants)
            },
        )
    };

    quote!(
    //use uid::UniqueIdentifier;
    //use dos_actors::UID;
        pub trait FemIo<U> {
            fn position(&self) -> Option<usize>;
        }
        type Item = (String,Vec<IO>);
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

    let parquet_reader = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(contents))?
        .with_batch_size(2048)
        .build()?;
    let schema = parquet_reader.schema();

    parquet_reader
        .map(|maybe_table| {
            if let Ok(table) = maybe_table {
                let (idx, _) = schema.column_with_name("group").expect(&format!(
                    r#"failed to get {}puts "group" index with field:\n{:}"#,
                    fem_io,
                    schema.field_with_name("group").unwrap()
                ));
                let data: Option<Vec<String>> =
                    match schema.field_with_name("group").unwrap().data_type() {
                        arrow::datatypes::DataType::Utf8 => table
                            .column(idx)
                            .as_any()
                            .downcast_ref::<StringArray>()
                            .expect(&format!(
                                r#"failed to get {}puts "group" data at index #{} from field\n{:}"#,
                                fem_io,
                                idx,
                                schema.field_with_name("group").unwrap()
                            ))
                            .iter()
                            .map(|x| x.map(|x| x.to_owned()))
                            .collect(),
                        arrow::datatypes::DataType::LargeUtf8 => table
                            .column(idx)
                            .as_any()
                            .downcast_ref::<LargeStringArray>()
                            .expect(&format!(
                                r#"failed to get {}puts "group" data at index #{} from field\n{:}"#,
                                fem_io,
                                idx,
                                schema.field_with_name("group").unwrap()
                            ))
                            .iter()
                            .map(|x| x.map(|x| x.to_owned()))
                            .collect(),
                        other => panic!(
                            r#"Expected "Uft8" or "LargeUtf8" datatype, found {}"#,
                            other
                        ),
                    };
                data.ok_or(Error::NoData)
            } else {
                Err(Error::NoRecord)
            }
        })
        .collect::<Result<Vec<_>, Error>>()
        .map(|data| data.into_iter().flatten().collect::<Vec<_>>())
        .map(|mut data| {
            data.dedup();
            data.into_iter()
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
                .unzip()
        })
}

// Build the enum
fn build_fem_io(io: Ident, names: Vec<Literal>, variants: Vec<Ident>) -> proc_macro2::TokenStream {
    quote!(
        #(
        #[derive(Debug, ::gmt_dos_clients::interface::UID)]
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
            pub fn name(&self) -> &str {
                match self {
                    #(#io::#variants(io) => {
                        #names
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
        impl TryFrom<Item> for #io {
            type Error = FemError;
            fn try_from((key,value): Item) -> std::result::Result<Self, Self::Error> {
                match key.as_str() {
                    #(#names => Ok(#io::#variants(value))),*,
                    _ => Err(FemError::Convert(key)),
                }
            }
        }

    )
}
