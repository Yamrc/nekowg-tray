use gpui_tray_core::Error;
use std::sync::Arc;
use zbus::zvariant::{Structure, StructureBuilder, Type};

const ICON_SIZES: [u32; 4] = [16, 24, 32, 48];

#[derive(Debug, Clone, Type)]
pub struct Pixmap {
    width: i32,
    height: i32,
    bytes: Vec<u8>,
}

impl Pixmap {
    pub fn new(width: i32, height: i32, bytes: Vec<u8>) -> Self {
        Self {
            width,
            height,
            bytes,
        }
    }
}

impl From<Pixmap> for Structure<'_> {
    fn from(value: Pixmap) -> Self {
        StructureBuilder::new()
            .add_field(value.width)
            .add_field(value.height)
            .add_field(value.bytes)
            .build()
            .expect("Pixmap structure build should not fail")
    }
}

pub struct Icon {
    pixmaps: Arc<Vec<Pixmap>>,
}

impl Icon {
    pub fn from_image(image: &gpui::Image) -> Result<Self, Error> {
        let img = image::load_from_memory(&image.bytes).map_err(|_| Error::InvalidIcon)?;

        let mut pixmaps = Vec::with_capacity(ICON_SIZES.len());

        for size in ICON_SIZES {
            let resized = img.resize_to_fill(size, size, image::imageops::FilterType::Lanczos3);
            let rgba = resized.to_rgba8();
            let argb = rgba_to_argb(&rgba);

            pixmaps.push(Pixmap::new(size as i32, size as i32, argb));
        }

        Ok(Self {
            pixmaps: Arc::new(pixmaps),
        })
    }

    pub fn as_pixmaps(&self) -> &[Pixmap] {
        &self.pixmaps
    }
}

impl Clone for Icon {
    fn clone(&self) -> Self {
        Self {
            pixmaps: self.pixmaps.clone(),
        }
    }
}

fn rgba_to_argb(rgba: &[u8]) -> Vec<u8> {
    let mut argb = Vec::with_capacity(rgba.len());
    for chunk in rgba.chunks_exact(4) {
        // ARGB format in native byte order
        // Each pixel is 4 bytes: [A, R, G, B]
        argb.push(chunk[3]);
        argb.push(chunk[0]);
        argb.push(chunk[1]);
        argb.push(chunk[2]);
    }
    argb
}
