use serde_json;
use wasm_bindgen::prelude::*;

use crate::{hmlanguages::*, Version};

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

fn error_to_string(err: LangError) -> String {
    format!("{}", err)
}

fn parse_version(version_str: &str) -> Version {
    match version_str {
        "H2016" => Version::H2016,
        "H2" => Version::H2,
        "H3" => Version::H3,
        _ => Version::H3, // Default to H3
    }
}

#[wasm_bindgen]
pub fn clng_convert_to_json(
    data: &[u8],
    meta_json: &str,
    version_str: &str,
) -> Result<String, String> {
    let version = parse_version(version_str);

    let clng = clng::CLNG::new(version, None).map_err(error_to_string)?;
    let result = clng
        .convert(data, meta_json.to_string())
        .map_err(error_to_string)?;
    serde_json::to_string(&result).map_err(|e| format!("JSON serialization error: {}", e))
}

#[wasm_bindgen]
pub fn clng_convert_from_json(json_str: &str, version_str: &str) -> Result<Vec<u8>, String> {
    let version = parse_version(version_str);

    let clng = clng::CLNG::new(version, None).map_err(error_to_string)?;
    let result = clng
        .rebuild(json_str.to_string())
        .map_err(error_to_string)?;
    Ok(result.file)
}

#[wasm_bindgen]
pub fn ditl_convert_to_json(
    data: &[u8],
    meta_json: &str,
    hashlist_data: &[u8],
) -> Result<String, String> {
    let hashlist =
        hashlist::HashList::load(hashlist_data).map_err(|e| format!("HashList error: {}", e))?;
    let ditl = ditl::DITL::new(hashlist).map_err(error_to_string)?;
    let result = ditl
        .convert(data, meta_json.to_string())
        .map_err(error_to_string)?;
    serde_json::to_string(&result).map_err(|e| format!("JSON serialization error: {}", e))
}

#[wasm_bindgen]
pub fn ditl_convert_from_json(json_str: &str, hashlist_data: &[u8]) -> Result<Vec<u8>, String> {
    let hashlist =
        hashlist::HashList::load(hashlist_data).map_err(|e| format!("HashList error: {}", e))?;
    let mut ditl = ditl::DITL::new(hashlist).map_err(error_to_string)?;
    let result = ditl
        .rebuild(json_str.to_string())
        .map_err(error_to_string)?;
    Ok(result.file)
}

#[wasm_bindgen]
pub fn dlge_convert_to_json(
    data: &[u8],
    meta_json: &str,
    hashlist_data: &[u8],
    version_str: &str,
) -> Result<String, String> {
    let version = parse_version(version_str);

    let hashlist =
        hashlist::HashList::load(hashlist_data).map_err(|e| format!("HashList error: {}", e))?;
    let dlge = dlge::DLGE::new(hashlist, version, None, None, false).map_err(error_to_string)?;
    let result = dlge
        .convert(data, meta_json.to_string())
        .map_err(error_to_string)?;
    serde_json::to_string(&result).map_err(|e| format!("JSON serialization error: {}", e))
}

#[wasm_bindgen]
pub fn dlge_convert_from_json(
    json_str: &str,
    hashlist_data: &[u8],
    version_str: &str,
) -> Result<Vec<u8>, String> {
    let version = parse_version(version_str);

    let hashlist =
        hashlist::HashList::load(hashlist_data).map_err(|e| format!("HashList error: {}", e))?;
    let mut dlge =
        dlge::DLGE::new(hashlist, version, None, None, false).map_err(error_to_string)?;
    let result = dlge
        .rebuild(json_str.to_string())
        .map_err(error_to_string)?;
    Ok(result.file)
}

#[wasm_bindgen]
pub fn locr_convert_to_json(
    data: &[u8],
    meta_json: &str,
    hashlist_data: &[u8],
    version_str: &str,
) -> Result<String, String> {
    let version = parse_version(version_str);

    let hashlist =
        hashlist::HashList::load(hashlist_data).map_err(|e| format!("HashList error: {}", e))?;
    let locr = locr::LOCR::new(hashlist, version, None, false).map_err(error_to_string)?;
    let result = locr
        .convert(data, meta_json.to_string())
        .map_err(error_to_string)?;
    serde_json::to_string(&result).map_err(|e| format!("JSON serialization error: {}", e))
}

#[wasm_bindgen]
pub fn locr_convert_from_json(
    json_str: &str,
    hashlist_data: &[u8],
    version_str: &str,
) -> Result<Vec<u8>, String> {
    let version = parse_version(version_str);

    let hashlist =
        hashlist::HashList::load(hashlist_data).map_err(|e| format!("HashList error: {}", e))?;
    let locr = locr::LOCR::new(hashlist, version, None, false).map_err(error_to_string)?;
    let result = locr
        .rebuild(json_str.to_string())
        .map_err(error_to_string)?;
    Ok(result.file)
}

#[wasm_bindgen]
pub fn rtlv_convert_to_json(
    data: &[u8],
    meta_json: &str,
    version_str: &str,
) -> Result<String, String> {
    let version = parse_version(version_str);

    let rtlv = rtlv::RTLV::new(version, None).map_err(error_to_string)?;
    let result = rtlv
        .convert(data, meta_json.to_string())
        .map_err(error_to_string)?;
    serde_json::to_string(&result).map_err(|e| format!("JSON serialization error: {}", e))
}

#[wasm_bindgen]
pub fn rtlv_convert_from_json(json_str: &str, version_str: &str) -> Result<Vec<u8>, String> {
    let version = parse_version(version_str);

    let mut rtlv = rtlv::RTLV::new(version, None).map_err(error_to_string)?;
    let result = rtlv
        .rebuild(json_str.to_string())
        .map_err(error_to_string)?;
    Ok(result.file)
}
