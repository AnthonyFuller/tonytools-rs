pub mod hmlanguages;
pub mod hmtextures;
pub mod util;

#[derive(Default, Debug, PartialEq, Copy, Clone)]
pub enum Version {
    H2016,
    H2,
    #[default]
    H3,
}
