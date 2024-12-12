pub mod cipher;
pub mod rpkg;

macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

pub(crate) use vec_of_strings;
