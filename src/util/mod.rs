pub mod cipher;
pub mod rpkg;

macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

pub(crate) use vec_of_strings;

use crate::{LangError, LangResult, Version};

pub(crate) fn get_language_map(version: Version) -> LangResult<Vec<String>>
{
    match version {
        Version::H2016 | Version::H2 => Ok(vec_of_strings![
            "xx", "en", "fr", "it", "de", "es", "ru", "mx", "br", "pl", "cn", "jp", "tc"
        ]),
        Version::H3 => Ok(vec_of_strings!["xx", "en", "fr", "it", "de", "es", "ru", "cn", "tc", "jp"]),
        Version::KNT => Ok(vec_of_strings!["xx", "en", "fr", "it", "de", "es", "ru", "mx", "br", "pl", "cn", "jp", "tc", "ko", "tr"]),
        _ => Err(LangError::UnsupportedVersion),
    }
}
