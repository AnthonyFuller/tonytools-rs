use serde::{Deserialize, Serialize};
use regex::Regex;

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
    pub hash_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceDependency {
    pub hash: String,
    pub flag: String,
}

pub fn is_valid_hash(hash: &str) -> bool {
    let re = Regex::new(r"^[0-9A-F]{16}$").unwrap();
    return re.is_match(hash);
}
