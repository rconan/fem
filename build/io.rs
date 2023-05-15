use std::fmt::Display;

use super::Names;

enum MatchArms {
    Same(String),
    Unique(Vec<String>),
    IgnoreUnique(Vec<String>),
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
struct Function<'a> {
    vis: String,
    name: String,
    object: String,
    args: Option<String>,
    fn_return: Option<String>,
    fn_where: Option<String>,
    arms: MatchArms,
    io: String,
    variants: &'a Names,
}
impl<'a> Function<'a> {
    pub fn new<S: Into<String>>(
        vis: S,
        name: S,
        object: S,
        arms: MatchArms,
        io: S,
        variants: &'a Names,
    ) -> Self {
        Self {
            vis: vis.into(),
            name: name.into(),
            object: object.into(),
            args: None,
            fn_return: None,
            fn_where: None,
            arms,
            io: io.into(),
            variants,
        }
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
        let variants = match &self.arms {
            MatchArms::Same(value) => self
                .variants
                .iter()
                .map(|name| {
                    format!(
                        "{io}::{variant}(io) => {{{value}}}",
                        io = self.io,
                        variant = name.variant(),
                        value = value.as_str()
                    )
                })
                .collect::<Vec<String>>()
                .join(",\n"),
            MatchArms::Unique(value) => self
                .variants
                .iter()
                .zip(value)
                .map(|(name, value)| {
                    format!(
                        "{io}::{variant}(io) => {{{value}}}",
                        io = self.io,
                        variant = name.variant(),
                        value = value
                    )
                })
                .collect::<Vec<String>>()
                .join(",\n"),
            MatchArms::IgnoreUnique(value) => self
                .variants
                .iter()
                .zip(value)
                .map(|(name, value)| {
                    format!(
                        "{io}::{variant}(_) => {{{value}}}",
                        io = self.io,
                        variant = name.variant(),
                        value = value
                    )
                })
                .collect::<Vec<String>>()
                .join(",\n"),
        };
        match (&self.args, &self.fn_return, &self.fn_where) {
            (None, None, None) => todo!(),
            (None, None, Some(_)) => todo!(),
            (None, Some(fn_return), None) => writeln!(
                f,
                "
            {vis} fn {name}({object}) -> {fn_return} {{
                match self {{
                    {variants}
                }}
            }}
            ",
                vis = self.vis,
                name = self.name,
                object = self.object,
                fn_return = fn_return,
                variants = variants
            ),
            (None, Some(_), Some(_)) => todo!(),
            (Some(_), None, None) => todo!(),
            (Some(_), None, Some(_)) => todo!(),
            (Some(args), Some(fn_return), None) => writeln!(
                f,
                "
            {vis} fn {name}({object}, {args}) -> {fn_return} {{
                match self {{
                    {variants}
                }}
            }}
            ",
                vis = self.vis,
                name = self.name,
                object = self.object,
                args = args,
                fn_return = fn_return,
                variants = variants
            ),
            (Some(args), Some(fn_return), Some(fn_where)) => writeln!(
                f,
                "
            {vis} fn {name}({object}, {args}) -> {fn_return} 
            where
                {fn_where}
            {{
                match self {{
                    {variants}
                }}
            }}
            ",
                vis = self.vis,
                name = self.name,
                object = self.object,
                args = args,
                fn_return = fn_return,
                fn_where = fn_where,
                variants = variants
            ),
        }
    }
}

pub struct IO<'a> {
    kind: String,
    variants: &'a Names,
}
impl<'a> IO<'a> {
    pub fn new<S: Into<String>>(kind: S, variants: &'a Names) -> Self {
        Self {
            kind: kind.into(),
            variants,
        }
    }
    /// impl TryFrom<String> for Box<dyn Get{io}>
    ///
    /// io: In|Out
    pub fn impl_tryfrom_for_getio(&self) -> String {
        let arms = self
            .variants
            .iter()
            .map(|name| {
                format!(
                    "{0} => Ok(Box::new(SplitFem::<{1}>::new()))",
                    name,
                    name.variant()
                )
            })
            .collect::<Vec<String>>()
            .join(",\n");
        format!(
            "
        impl TryFrom<String> for Box<dyn Get{io}> {{
            type Error = FemError;
            fn try_from(value: String) -> std::result::Result<Self, Self::Error> {{
                match value.as_str() {{
                    {arms},
                    _ => Err(FemError::Convert(value)),
                }}
            }}
         }}
        ",
            io = self.kind,
            arms = arms
        )
    }
    /// pub enum {io}
    ///
    /// io: Inputs|Outputs
    pub fn enum_io(&self) -> String {
        let variants = self
            .variants
            .iter()
            .map(|name| {
                format!(
                    r##"
            #[doc = "{0}"]
            #[cfg_attr(feature="serde", serde(rename = "{0}"))]
            {1}(Vec<IO>)
        "##,
                    name,
                    name.variant()
                )
            })
            .collect::<Vec<String>>()
            .join(",\n");
        format!(
            r##"
        #[cfg_attr(feature="serde",derive(serde::Serialize, serde::Deserialize))]
        #[derive(Debug, Clone)]
        pub enum {io} {{
            {variants}
        }}
        "##,
            io = self.kind,
            variants = variants
        )
    }
}
impl<'a> Display for IO<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.enum_io())?;
/*         for variant in self.variants.iter() {
            writeln!(
                f,
                "{}",
                variant.impl_enum_variant_for_io(self.kind.as_str())
            )?;
        } */
        // impl #io
        writeln!(f, "impl {} {{", self.kind)?;
        writeln!(
            f,
            "{}",
            Function::new(
                "pub",
                "len",
                "&self",
                MatchArms::Same(String::from(
                    "io.iter().fold(0,|a,x| a + x.is_on() as usize)"
                )),
                self.kind.as_str(),
                self.variants
            )
            .fn_return("usize")
        )?;
        writeln!(
            f,
            "{}",
            Function::new(
                "pub",
                "get_by<F,T>",
                "&self",
                MatchArms::Same(String::from(
                    "io.iter().filter_map(|x| x.get_by(pred)).collect()"
                )),
                self.kind.as_str(),
                self.variants
            )
            .args("pred: F")
            .fn_return("Vec<T>")
            .fn_where("F: Fn(&IOData) -> Option<T> + Copy")
        )?;
        writeln!(
            f,
            "{}",
            Function::new(
                "pub",
                "name",
                "&self",
                MatchArms::IgnoreUnique(
                    self.variants
                        .iter()
                        .map(|name| format!(r#""{}""#, name))
                        .collect()
                ),
                self.kind.as_str(),
                self.variants
            )
            .fn_return("&str")
        )?;
        writeln!(f, "}}")?;
        // impl std::ops::Deref for #io
        writeln!(f, "impl std::ops::Deref for {} {{", self.kind)?;
        writeln!(f, "type Target = [IO];")?;
        writeln!(
            f,
            "{}",
            Function::new(
                "",
                "deref",
                "&self",
                MatchArms::Same(String::from("io")),
                self.kind.as_str(),
                self.variants
            )
            .fn_return("&Self::Target")
        )?;
        writeln!(f, "}}")?;
        // impl std::ops::DerefMut for #io
        writeln!(f, "impl std::ops::DerefMut for {} {{", self.kind)?;
        writeln!(
            f,
            "{}",
            Function::new(
                "",
                "deref_mut",
                "&mut self",
                MatchArms::Same(String::from("io")),
                self.kind.as_str(),
                self.variants
            )
            .fn_return("&mut Self::Target")
        )?;
        writeln!(f, "}}")?;
        // impl std::fmt::Display for #io
        writeln!(f, "impl std::fmt::Display for {} {{", self.kind)?;
        writeln!(
            f,
            "{}",
            Function::new(
                "",
                "fmt",
                "&self",
                MatchArms::Unique(
                    self.variants
                        .iter()
                        .map(|name| format!(
                            r#"
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
                    }}"#,
                            variant = name.variant()
                        ))
                        .collect()
                ),
                self.kind.as_str(),
                self.variants
            )
            .args("f: &mut std::fmt::Formatter<'_>")
            .fn_return("std::fmt::Result")
        )?;
        writeln!(f, "}}")?;
        let arms = self
            .variants
            .iter()
            .map(|name| {
                format!(
                    r##""{name}" => Ok({io}::{variant}(value)),"##,
                    name = name,
                    io = self.kind,
                    variant = name.variant()
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        writeln!(
            f,
            r##"
        impl TryFrom<Item> for {io} {{
            type Error = FemError;
            fn try_from((key,value): Item) -> std::result::Result<Self, Self::Error> {{
                match key.as_str() {{
                    {arms}
                    _ => Err(FemError::Convert(key)),
                }}
            }}
        }}            
        "##,
            io = self.kind,
            arms = arms
        )?;
        Ok(())
    }
}
