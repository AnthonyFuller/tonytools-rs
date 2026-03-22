use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use regex_lite::Regex;
use once_cell::sync::Lazy;

#[derive(Serialize, Deserialize, Debug)]
pub struct ResourceMeta {
    pub hash_offset: u64,
    pub hash_reference_data: Vec<ResourceDependency>,
    pub hash_reference_table_dummy: u32,
    pub hash_reference_table_size: u32,
    pub hash_resource_type: String,
    pub hash_size: u32,
    pub hash_size_final: u32,
    pub hash_size_in_memory: u32,
    pub hash_size_in_video_memory: u32,
    pub hash_value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash_path: Option<String>,
}

impl ResourceMeta {
    pub fn new(
        hash: String,
        size: u32,
        four_cc: String,
        depends: IndexMap<String, String>,
    ) -> Self {
        Self {
            hash_value: if is_valid_hash(&hash) {
                hash
            } else {
                compute_hash(&hash)
            },
            hash_offset: 0x10000000,
            hash_size: 0x80000000 + size,
            hash_resource_type: four_cc,
            hash_reference_table_size: (0x9 * depends.len()) as u32 + 4,
            hash_reference_table_dummy: 0,
            hash_size_final: size,
            hash_size_in_memory: u32::MAX,
            hash_size_in_video_memory: u32::MAX,
            hash_path: None,
            hash_reference_data: depends
                .iter()
                .map(|(hash, flag)| ResourceDependency {
                    hash: hash.clone(),
                    flag: flag.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceDependency {
    pub hash: String,
    pub flag: String,
}

static HASH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[0-9A-F]{16}$").unwrap());

pub fn is_valid_hash(hash: &str) -> bool {
    HASH_RE.is_match(hash)
}

pub fn compute_hash(hash: &str) -> String {
    let hash = format!("{:X}", md5::compute(hash));
    format!("00{}", &hash[2..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_hash() {
        assert_eq!(is_valid_hash("0123456789ABCDEF"), true);
        assert_eq!(is_valid_hash("0123456789abcdef"), false);
        assert_eq!(is_valid_hash("f00bar"), false);
        assert_eq!(is_valid_hash("1751D6Q65KNDDABF"), false);
        assert_eq!(is_valid_hash("5FFABEEADD2CFFFE"), true);
    }

    #[test]
    fn test_compute_hash() {
        let input = "hello";
        assert_eq!(compute_hash(input), "0041402ABC4B2A76");
    }
}
