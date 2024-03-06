pub enum Version {
    H2016 = 2,
    H2,
    H3,
}

pub struct Rebuilt {
    pub file: Vec<u8>,
    pub meta: String,
}
