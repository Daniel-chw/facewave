// handles loading and saving the image

use crate::Image;
use image::{DynamicImage, GrayImage, ImageReader};


pub fn load_grayscale(path: &str) -> Result<Image, image::ImageError> {
    let img = ImageReader::open(path)?.decode()?;
    let gray = img.to_luma32f();

    let w = gray.width() as usize;
    let h = gray.height() as usize;
    let data = gray.into_raw();

    Ok(Image { w, h, data })

}

pub fn save(img: &Image, path: &str) -> image::ImageResult<()>  {
    let w = img.w as u32;
    let h = img.h as u32;
    
    let pixels: Vec<u8> = img.data.iter().map(|&v| (v*255.0) as u8).collect();

    let buf = GrayImage::from_raw(w, h, pixels).unwrap();

    buf.save(path)

}

pub fn psnr(a: &Image, b: &Image) -> f32 {
    todo!()
}