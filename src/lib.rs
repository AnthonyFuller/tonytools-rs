#![allow(incomplete_features)]
#![feature(generic_const_exprs)]

pub mod hmlanguages;
pub(crate) mod hmtextures;
pub(crate) mod util;

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub enum Version {
    Unknown = -1,
    H2016,
    H2,
    #[default]
    H3,
}

pub use hmlanguages::*;
