use arrow::{array::StringArray, record_batch::RecordBatch};
use parquet::{
    arrow::{ArrowReader, ParquetFileArrowReader},
    file::reader::SerializedFileReader,
    util::cursor::SliceableCursor,
};
use std::{fs::File, io::Read, path::Path, sync::Arc};

fn main() -> anyhow::Result<()> {
    let path = Path::new("/home/rconan/Dropbox/Documents/GMT/CFD/Baseline2021/validation/2021windloads/20211103_2333_MT_mount_zen_30_m1HFN_CFD/");
    let file = File::open(path.join("modal_state_space_model_2ndOrder.zip"))?;
    let mut zip_file = zip::ZipArchive::new(file)?;
    for name in zip_file.file_names() {
        println!("{}", name);
    }

    let mut input_file = zip_file.by_name("modal_state_space_model_2ndOrder_in.parquet")?;
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
        println!(
            "Fields: {:#?}",
            schema
                .fields()
                .iter()
                .map(|f| f.name())
                .collect::<Vec<&String>>()
        );
        let input_table = RecordBatch::concat(&schema, &input_records)?;
        let (idx, _) = schema.column_with_name("group").unwrap();
        let inputs: Option<Vec<&str>> = input_table
            .column(idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap()
            .iter()
            .collect();
        if let Some(mut inputs) = inputs {
            inputs.dedup();
            println!("{:#?}", inputs);
        }
    }
    Ok(())
}
