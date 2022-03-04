use proc_macro::TokenStream;

mod parquet_io;
use parquet_io::ad_hoc_macro;

/// Ad-hoc fem crate builder
#[proc_macro]
pub fn ad_hoc(_item: TokenStream) -> TokenStream {
    ad_hoc_macro(_item)
}
