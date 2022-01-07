use proc_macro::TokenStream;

#[cfg(feature = "hdf5")]
mod hdf5_io;
#[cfg(feature = "hdf5")]
use hdf5_io::{ad_hoc_macro, match_maker_macro};

#[cfg(feature = "prqt")]
mod parquet_io;
#[cfg(feature = "prqt")]
use parquet_io::{ad_hoc_macro, match_maker_macro};

/// Ad-hoc fem crate builder
#[proc_macro]
pub fn ad_hoc(_item: TokenStream) -> TokenStream {
    ad_hoc_macro(_item)
}
/// Implement the `MatchFem` trait
#[proc_macro]
pub fn match_maker(_item: TokenStream) -> TokenStream {
    match_maker_macro(_item)
}
