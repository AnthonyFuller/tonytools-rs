#![feature(
    generic_const_exprs,
    associated_type_defaults,
    negative_impls,
    trait_alias,
    debug_closure_helpers
)]

pub mod hmlanguages;
pub mod hmtextures;
pub mod util;

#[derive(Default, Debug, PartialEq)]
pub enum Version {
    H2016,
    H2,
    #[default]
    H3,
    HMA,
    Alpha,
}
