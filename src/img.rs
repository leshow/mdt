use image;
use image::GenericImage;

use std::convert::AsRef;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use super::MDResult;

pub fn img_dim<P>(path: P) -> MDResult<(u32, u32)>
where
    P: AsRef<Path>,
{
    let reader = BufReader::new(File::open(path.as_ref())?);
    Ok(image::open(reader)?.dimensions())
}
