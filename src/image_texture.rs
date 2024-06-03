use image::{GenericImageView, ImageBuffer, Rgba};

#[derive(Debug, Clone)]
pub struct ImageTexture {
    pub from_color: bool,
    pub color: Option<[f32; 3]>,
    pub image_buffer: ImageBuffer<Rgba<u8>, Vec<u8>>,
}

impl ImageTexture {
    pub fn new_from_color(color: [f32; 3], texture_size: [u32; 2]) -> ImageTexture {
        ImageTexture {
            from_color: true,
            color: Some(color),
            image_buffer: solid_color_image(color, texture_size),
        }
    }

    pub fn new_from_image(path: &str, texture_size: [u32; 2]) -> ImageTexture {
        ImageTexture {
            from_color: false,
            color: None,
            image_buffer: load_png_image(path, texture_size),
        }
    }

    pub fn update_color(&mut self) {
        // we dont recolor textures that were loaded from files
        if let Some(color) = self.color {
            let rgba_color = [
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                255,
            ];
            for pixel in self.image_buffer.pixels_mut() {
                *pixel = Rgba(rgba_color);
            }
        }
    }
}

pub fn solid_color_image(
    color: [f32; 3],
    texture_size: [u32; 2],
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(texture_size[0], texture_size[1]);
    let rgba_color = [
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        255,
    ];
    for pixel in img.pixels_mut() {
        *pixel = Rgba(rgba_color);
    }
    img
}

pub fn load_png_image(path: &str, texture_size: [u32; 2]) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let img = image::open(path).expect("could not load the image");
    let dim = img.dimensions();
    assert_eq!(
        dim,
        texture_size.into(),
        "Image dimension has to be the same for all images, defined in the scene definition"
    );
    img.to_rgba8()
}
