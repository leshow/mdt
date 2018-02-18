use immeta;
use std::convert::AsRef;
// use std::fs::File;
// use std::io::BufReader;
use std::path::Path;

use super::MDResult;

pub fn img_dim<P>(path: P) -> MDResult<(u32, u32)>
where
    P: AsRef<Path>,
{
    let dim = immeta::load_from_file(path.as_ref())?.dimensions();
    Ok((dim.width, dim.height))
}
