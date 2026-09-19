// handles loading and saving the image

use crate::Image;
use image::{ GrayImage, ImageReader, imageops::FilterType};


pub fn load_grayscale(path: &str) -> Result<Image, image::ImageError> {
    let img = ImageReader::open(path)?.decode()?;
    let resized = img.resize_exact(128, 128, FilterType::Lanczos3);
    let gray = resized.to_luma32f();

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

// checks similary between images giving how lossy image is
pub fn psnr(a: &Image, b: &Image) -> f32 {
    let n = a.data.len() as f32;
    let mse: f32 = a.data.iter()
                         .zip(b.data.iter())
                         .map(|(x,y)| (x-y).powi(2))
                         .sum::<f32>() / n;

    10.0*(1.0/ mse).log10()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psnr_of_identical_images_is_infinite() {
        let img = Image { w: 2, h: 2, data: vec![0.1, 0.5, 0.9, 1.0] };
        assert_eq!(psnr(&img, &img), f32::INFINITY);
    }

    #[test]
    fn psnr_decreases_as_error_increases() {
        let a = Image { w: 1, h: 3, data: vec![0.5, 0.5, 0.5] };
        let small_error = Image { w: 1, h: 3, data: vec![0.51, 0.51, 0.51] };
        let big_error = Image { w: 1, h: 3, data: vec![0.9, 0.9, 0.9] };

        assert!(psnr(&a, &small_error) > psnr(&a, &big_error));
    }

    #[test]
    fn psnr_of_maximally_different_images() {
        let black = Image { w: 1, h: 2, data: vec![0.0, 0.0] };
        let white = Image { w: 1, h: 2, data: vec![1.0, 1.0] };

        assert!((psnr(&black, &white) - 0.0).abs() < 1e-5);
    }
}