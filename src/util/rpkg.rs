use fancy_regex::Regex;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

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

pub fn is_valid_hash(hash: &str) -> bool {
    let re = Regex::new(r"^[0-9A-F]{16}$").unwrap();
    re.is_match(hash).unwrap()
}

pub fn compute_hash(hash: &str) -> String {
    let hash = format!("{:X}", md5::compute(hash));
    format!("00{}", &hash[2..16])
}
