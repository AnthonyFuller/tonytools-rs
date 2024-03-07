pub mod hashlist;
pub mod locr;

#[derive(Debug)]
pub enum LangError {
    InvalidLanguageMap,
    DidNotReachEOF,
    JsonError,
}

pub struct Rebuilt {
    pub file: Vec<u8>,
    pub meta: String,
}
