use std::{env, fs::{File, self}, path::Path, io::Read, ops::Deref, fmt::Display};

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
pub struct Name(String);
impl Deref for Name {
    type Target=str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}
impl Display for Name{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}",self.0)
    }
}
impl From<&Name> for String {
    fn from(value: &Name) -> Self {
        value.0.clone()
    }
}
impl Name {
    pub fn variant(&self) -> String {
        self
        .split("_")
        .map(|s| {
            let (first, last) = s.split_at(1);
            first.to_uppercase() + last
        })
        .collect::<String>()
    }
    /// pub enum {variant} {}
    pub fn enum_variant(&self) -> String {
        format!(r##"
        #[derive(Debug, ::gmt_dos_clients::interface::UID)]
        pub enum {variant} {{}}
        "##,variant=self.variant())
    }
    /// impl FemIo<{variant}> for Vec<Option<{io}>>
    /// 
    /// io: Inputs|Outputs
    pub fn impl_enum_variant_for_io(&self,io: &str) -> String {
        format!(r##"
        impl FemIo<{variant}> for Vec<Option<{io}>> {{
            fn position(&self) -> Option<usize>{{
                self.iter().filter_map(|x| x.as_ref())
                        .position(|x| if let {io}::{variant}(_) = x {{true}} else {{false}})
            }}
        }}
        "##,variant=self.variant(),io=io)
    }
}

pub struct Names(Vec<Name>);
impl FromIterator<Name> for Names{
    fn from_iter<T: IntoIterator<Item = Name>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
impl Deref for Names {
    type Target = Vec<Name>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Display for Names{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for variant in self.iter() {
            write!(f,"{}",variant.enum_variant())?;
        }
        Ok(())
    }
}

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
/* 
impl Names {
    /// impl TryFrom<String> for Box<dyn Get{io}>
    /// 
    /// io: In|Out
    pub fn impl_tryfrom_for_getio(&self,io: &str) -> String {
        let arms = self.iter()
            .map(|name|
            format!("{0} => Ok(Box::new(SplitFem::<{1}>::new()))",
                name,name.variant()))
            .collect::<Vec<String>>().join(",\n");
        format!("
        impl TryFrom<String> for Box<dyn Get{io}> {{
            type Error = FemError;
            fn try_from(value: String) -> std::result::Result<Self, Self::Error> {{
                match value.as_str() {{
                    {arms},
                    _ => Err(FemError::Convert(value)),
                }}
            }}
         }}
        ",io=io,arms=arms)
    }
    /// pub enum {io}
    /// 
    /// io: Inputs|Outputs
    pub fn enum_io(&self, io: &str) -> String {
        let variants = self.iter()
        .map(|name|
        format!(r##"
            #[doc = {0}]
            #[serde(rename = {0})]
            {1}(Vec<IO>)
        "##,name,name.variant()))
        .collect::<Vec<String>>().join(",\n");
        format!(r##"pub enum {io} {{
            {variants}
        }}"##,io=io,variants=variants)
    }
}
 */
pub enum MatchArms {
    Same(String),
    Unique(Vec<String>),
    IgnoreUnique(Vec<String>)
}

/// Function signature
///
/// <vis> fn <name>(<object>, <args>) -> <fn_return> 
/// where <fn_where>
/// {
///     match self {
///         <io>::<variant>(io) => { arms }
///     }
/// }
pub struct Function<'a> {
    vis: String,
    name: String,
    object: String,
    args: Option<String>,
    fn_return: Option<String>,
    fn_where: Option<String>,
    arms: MatchArms,
    io: String,
    variants: &'a Names
}
impl<'a> Function<'a> {
    pub fn new<S: Into<String>>(vis: S,
        name: S,
        object: S,
        arms: MatchArms,
        io: S,
        variants: &'a Names) -> Self {
            Self{ vis: vis.into(),
                  name: name.into(),
                  object: object.into(), 
                  args: None, 
                  fn_return: None, 
                  fn_where: None, 
                  arms, 
                  io: io.into(),
                  variants }
        }
    pub fn fn_return(mut self, fn_return: &str) -> Self {
        self.fn_return = Some(fn_return.into());
        self
    }
    pub fn fn_where(mut self, fn_where: &str) -> Self {
        self.fn_where = Some(fn_where.into());
        self
    }
        pub fn args(mut self, args: &str) -> Self {
        self.args = Some(args.into());
        self
    }
}
impl<'a> Display for Function<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let variants  = match &self.arms {
            MatchArms::Same(value) => self.variants.iter()
            .map(|name| format!("{io}::{variant}(io) => {{{value}}}",
            io=self.io,variant=name.variant(),value=value.as_str()))
            .collect::<Vec<String>>().join(",\n"),
            MatchArms::Unique(value) => self.variants.iter().zip(value)
            .map(|(name,value)| format!("{io}::{variant}(io) => {{{value}}}",
            io=self.io,variant=name.variant(),value=value))
            .collect::<Vec<String>>().join(",\n"),
            MatchArms::IgnoreUnique(value) => self.variants.iter().zip(value)
            .map(|(name,value)| format!("{io}::{variant}(_) => {{{value}}}",
            io=self.io,variant=name.variant(),value=value))
            .collect::<Vec<String>>().join(",\n"),        };
        match (&self.args,&self.fn_return,&self.fn_where){
            (None, None, None) => todo!(),
            (None, None, Some(_)) => todo!(),
            (None, Some(fn_return), None) => writeln!(f,"
            {vis} fn {name}({object}) -> {fn_return} {{
                match self {{
                    {variants}
                }}
            }}
            ",vis=self.vis,name=self.name,object=self.object,fn_return=fn_return,variants=variants),
            (None, Some(_), Some(_)) => todo!(),
            (Some(_), None, None) => todo!(),
            (Some(_), None, Some(_)) => todo!(),
            (Some(args), Some(fn_return), None) => writeln!(f,"
            {vis} fn {name}({object}, {args}) -> {fn_return} {{
                match self {{
                    {variants}
                }}
            }}
            ",vis=self.vis,name=self.name,object=self.object,args=args,fn_return=fn_return,variants=variants),
            (Some(args), Some(fn_return), Some(fn_where)) => writeln!(f,"
            {vis} fn {name}({object}, {args}) -> {fn_return} 
            where
                {fn_where}
            {{
                match self {{
                    {variants}
                }}
            }}
            ",vis=self.vis,name=self.name,object=self.object,args=args,fn_return=fn_return,fn_where=fn_where,variants=variants),
        }
    }
}

pub struct IO<'a>{
    kind: String,
    variants: &'a Names
}
impl<'a> IO<'a> {
    pub fn new<S:Into<String>>(kind: S, variants: &'a Names) -> Self {
        Self{ kind: kind.into(), variants }
    }
    /// impl TryFrom<String> for Box<dyn Get{io}>
    /// 
    /// io: In|Out
    pub fn impl_tryfrom_for_getio(&self) -> String {
        let arms = self.variants.iter()
            .map(|name|
            format!("{0} => Ok(Box::new(SplitFem::<{1}>::new()))",
                name,name.variant()))
            .collect::<Vec<String>>().join(",\n");
        format!("
        impl TryFrom<String> for Box<dyn Get{io}> {{
            type Error = FemError;
            fn try_from(value: String) -> std::result::Result<Self, Self::Error> {{
                match value.as_str() {{
                    {arms},
                    _ => Err(FemError::Convert(value)),
                }}
            }}
         }}
        ",io=self.kind,arms=arms)
    }
    /// pub enum {io}
    /// 
    /// io: Inputs|Outputs
    pub fn enum_io(&self) -> String {
        let variants = self.variants.iter()
        .map(|name|
        format!(r##"
            #[doc = "{0}"]
            #[serde(rename = "{0}")]
            {1}(Vec<IO>)
        "##,name,name.variant()))
        .collect::<Vec<String>>().join(",\n");
        format!(r##"
        #[derive(Deserialize, Debug, Clone)]
        pub enum {io} {{
            {variants}
        }}
        "##,io=self.kind,variants=variants)
    }
}
impl<'a> Display for IO<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f,"{}",self.enum_io())?;
        for variant in self.variants.iter() {
            writeln!(f,"{}",variant.impl_enum_variant_for_io(self.kind.as_str()))?;
        }
        // impl #io
        writeln!(f,"impl {} {{",self.kind)?;
        writeln!(f,"{}",Function::new("pub","len","&self",
                    MatchArms::Same(String::from("io.iter().fold(0,|a,x| a + x.is_on() as usize)")),
                    self.kind.as_str(),self.variants)
                    .fn_return("usize"))?;
        writeln!(f,"{}",Function::new("pub","get_by<F,T>","&self",
                    MatchArms::Same(String::from("io.iter().filter_map(|x| x.get_by(pred)).collect()")),
                    self.kind.as_str(),self.variants)
                    .args("pred: F").fn_return("Vec<T>").fn_where("F: Fn(&IOData) -> Option<T> + Copy"))?;       
        writeln!(f,"{}",Function::new("pub","name","&self",
                MatchArms::IgnoreUnique(self.variants.iter().map(|name| format!(r#""{}""#,name)).collect()),
                self.kind.as_str(),self.variants)
                .fn_return("&str"))?;
        writeln!(f,"}}")?;
        // impl std::ops::Deref for #io 
        writeln!(f,"impl std::ops::Deref for {} {{",self.kind)?;
        writeln!(f,"type Target = [IO];")?;
        writeln!(f,"{}",Function::new("","deref","&self",
                    MatchArms::Same(String::from("io")),
                    self.kind.as_str(),self.variants)
                    .fn_return("&Self::Target"))?;
        writeln!(f,"}}")?;
        // impl std::ops::DerefMut for #io 
        writeln!(f,"impl std::ops::DerefMut for {} {{",self.kind)?;
        writeln!(f,"{}",Function::new("","deref_mut","&mut self",
                    MatchArms::Same(String::from("io")),
                    self.kind.as_str(),self.variants)
                    .fn_return("&mut Self::Target"))?;
        writeln!(f,"}}")?;
        // impl std::fmt::Display for #io
        writeln!(f,"impl std::fmt::Display for {} {{",self.kind)?;
        writeln!(f,"{}",Function::new("","fmt","&self",
                    MatchArms::Unique(self.variants.iter().map(|name| format!(r#"
                    let mut cs: Vec<_> = io.iter().filter_map(|x| match x {{
                        IO::On(data) => data.properties.cs_label.as_ref(),
                        IO::Off(_) => None
                    }}).collect();
                    cs.sort();
                    cs.dedup();
                    if cs.len()>1 {{
                        write!(f,"{{:>24}}: [{{:5}}]",stringify!({variant}),self.len())
                    }} else {{
                        write!(f,"{{:>24}}: [{{:5}}] {{:?}}",stringify!({variant}),self.len(),cs)
                    }}"#,variant=name.variant())).collect()),
                    self.kind.as_str(),self.variants)
                    .args("f: &mut std::fmt::Formatter<'_>")
                    .fn_return("std::fmt::Result"))?;
        writeln!(f,"}}")?; 
        let arms = self.variants.iter()
        .map(|name|
        format!(r##""{name}" => Ok({io}::{variant}(value)),"##,name=name,io=self.kind,variant=name.variant()))
        .collect::<Vec<String>>().join("\n");
        writeln!(f,r##"
        impl TryFrom<Item> for {io} {{
            type Error = FemError;
            fn try_from((key,value): Item) -> std::result::Result<Self, Self::Error> {{
                match key.as_str() {{
                    {arms}
                    _ => Err(FemError::Convert(key)),
                }}
            }}
        }}            
        "##,io=self.kind,arms=arms)?;
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
    fs::write(dest_path.join("fem_get_out.rs"), format!("{}", GetIO::new("Out",&input_names)))?;

    fs::write(dest_path.join("fem_inputs.rs"), format!("{}", IO::new("Inputs",&input_names)))?;
    fs::write(dest_path.join("fem_outputs.rs"), format!("{}", IO::new("Outputs",&input_names)))?;

    Ok(())
}
