use std::{env, fs::{File, self}, path::Path, io::Read, fmt::Display};

use arrow::{array::{StringArray, LargeStringArray}, record_batch::RecordBatchReader};
use bytes::Bytes;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
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

mod names;
pub use names::{Name,Names};
mod io;
pub use io::IO;

pub struct GetIO<'a>{
    kind: String,
    variants: &'a Names,
}
impl<'a> GetIO<'a> {
    pub fn new<S: Into<String>>(kind: S, variants: &'a Names) -> Self {
        Self { kind: kind.into(), variants}
    }
}
impl<'a> Display for GetIO<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let arms = self.variants.iter()
            .map(|name|
            format!(r#""{0}" => Ok(Box::new(SplitFem::<{1}>::new()))"#,
                name,name.variant()))
            .collect::<Vec<String>>().join(",\n");
        write!(f,"
        impl TryFrom<String> for Box<dyn Get{io}> {{
            type Error = FemError;
            fn try_from(value: String) -> std::result::Result<Self, Self::Error> {{
                match value.as_str() {{
                    {arms},
                    _ => Err(FemError::Convert(value)),
                }}
            }}
         }}
        ",io=self.kind,arms=arms)?;
        Ok(())
    }
} 

// Read the fields
fn get_fem_io(zip_file: &mut ZipArchive<File>, fem_io: &str) -> Result<Names,Error> {
    println!("FEM_{}PUTS", fem_io.to_uppercase());
    let Ok(mut input_file) = zip_file.by_name(&format!(
        "rust/modal_state_space_model_2ndOrder_{}.parquet",
        fem_io
    )) else {
        panic!(r#"cannot find "rust/modal_state_space_model_2ndOrder_{}.parquet" in archive"#,fem_io)
    };
    let mut contents: Vec<u8> = Vec::new();
    input_file.read_to_end(&mut contents)?;

    let Ok(parquet_reader) = 
     ParquetRecordBatchReaderBuilder::try_new(Bytes::from(contents))
    else { panic!("failed to create `ParquetRecordBatchReaderBuilder`") };
    let Ok(parquet_reader) = 
        parquet_reader.with_batch_size(2048).build() 
    else { panic!("failed to create `ParquetRecordBatchReader`")};
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
                let name = Name(fem_io);
                println!(" #{:03}: {:>32} <=> {:<32}", k, name, name.variant());
                name
            })
            .collect()
    })
}

fn main() -> anyhow::Result<()> {
    let Ok(fem_repo) = env::var("FEM_REPO") else {
        panic!(r#"the environment variable "FEM_REPO" is not set"#)
    };
    // Gets the FEM repository
    println!(
        "Building `fem::Inputs` and `fem::Outputs` enums to match inputs/outputs of FEM in {}",
        fem_repo
    );
    // Opens the mat file
    let path = Path::new(&fem_repo);
    let Ok(file) = File::open(path.join("modal_state_space_model_2ndOrder.zip")) 
    else {
        panic!("Cannot find `modal_state_space_model_2ndOrder.zip` in `FEM_REPO`");
    };
    let mut zip_file = zip::ZipArchive::new(file)?;

    let Ok(input_names) = get_fem_io(&mut zip_file, "in") 
    else {panic!("failed to parse FEM inputs variables")};
    let Ok(output_names) = get_fem_io(&mut zip_file, "out") 
    else {panic!("failed to parse FEM outputs variables")};

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir);

    fs::write(dest_path.join("fem_actors_inputs.rs"), format!("{}", input_names))?;
    fs::write(dest_path.join("fem_actors_outputs.rs"), format!("{}", output_names))?;

    fs::write(dest_path.join("fem_get_in.rs"), format!("{}", GetIO::new("In",&input_names)))?;
    fs::write(dest_path.join("fem_get_out.rs"), format!("{}", GetIO::new("Out",&output_names)))?;

    fs::write(dest_path.join("fem_inputs.rs"), format!("{}", IO::new("Inputs",&input_names)))?;
    fs::write(dest_path.join("fem_outputs.rs"), format!("{}", IO::new("Outputs",&output_names)))?;

    Ok(())
}
