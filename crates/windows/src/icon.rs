use gpui_tray_core::Error;
use log::debug;
use std::sync::Arc;
use windows::Win32::Foundation::TRUE;
use windows::Win32::Graphics::Gdi::DeleteObject;
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};

/// A Windows icon handle wrapper that ensures proper cleanup.
pub struct Icon {
    hicon: Arc<HICON>,
}

impl Icon {
    /// Creates an icon from a GPUI Image.
    ///
    /// Automatically decodes the image and converts it to a Windows HICON.
    /// The image is resized to 32x32 pixels for optimal tray display.
    pub fn from_image(image: &gpui::Image) -> Result<Self, Error> {
        debug!("Creating icon from image, format: {:?}", image.format);

        let img = image::load_from_memory(&image.bytes)
            .map_err(|_| Error::InvalidIcon)?;

        let resized = img.resize_to_fill(32, 32, image::imageops::FilterType::Lanczos3);
        let rgba = resized.to_rgba8();

        let hicon = create_hicon(&rgba, 32, 32)?;

        Ok(Self {
            hicon: Arc::new(hicon),
        })
    }

    /// Returns the underlying HICON handle.
    pub fn as_hicon(&self) -> HICON {
        *self.hicon
    }
}

impl Drop for Icon {
    fn drop(&mut self) {
        if Arc::strong_count(&self.hicon) == 1 {
            debug!("Destroying icon");
            unsafe {
                let _ = DestroyIcon(*self.hicon);
            }
        }
    }
}

impl Clone for Icon {
    fn clone(&self) -> Self {
        Self {
            hicon: self.hicon.clone(),
        }
    }
}

fn create_hicon(rgba: &[u8], width: u32, height: u32) -> Result<HICON, Error> {
    use windows::Win32::Graphics::Gdi::{
        BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, GetDC, ReleaseDC,
    };
    use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, ICONINFO};

    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return Err(Error::Platform("Failed to get device context".into()));
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };

        let mut bits: *mut u8 = std::ptr::null_mut();
        let hbitmap = CreateDIBSection(
            Some(hdc),
            &bmi,
            windows::Win32::Graphics::Gdi::DIB_RGB_COLORS,
            &mut bits as *mut _ as *mut *mut std::ffi::c_void,
            None,
            0,
        )
        .map_err(|_| Error::Platform("Failed to create DIB section".into()))?;

        std::ptr::copy_nonoverlapping(rgba.as_ptr(), bits, rgba.len());

        let _ = ReleaseDC(None, hdc);

        let mut and_mask = vec![0xFFu8; ((width + 7) / 8 * height) as usize];
        for (i, chunk) in rgba.chunks(4).enumerate() {
            let alpha = chunk[3];
            if alpha < 128 {
                let x = (i % width as usize) as u32;
                let y = (i / width as usize) as u32;
                let byte_index = (y * ((width + 7) / 8) + (x / 8)) as usize;
                let bit_index = x % 8;
                and_mask[byte_index] &= !(1 << (7 - bit_index));
            }
        }

        let hmask = CreateBitmap(
            width as i32,
            height as i32,
            1,
            1,
            Some(and_mask.as_ptr() as *const _),
        );

        if hmask.is_invalid() {
            let _ = DeleteObject(hbitmap.into());
            return Err(Error::Platform("Failed to create mask bitmap".into()));
        }

        let icon_info = ICONINFO {
            fIcon: TRUE,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hmask,
            hbmColor: hbitmap,
        };

        let hicon = CreateIconIndirect(&icon_info)
            .map_err(|_| Error::Platform("Failed to create icon".into()))?;

        let _ = DeleteObject(hbitmap.into());
        let _ = DeleteObject(hmask.into());

        if hicon.is_invalid() {
            return Err(Error::InvalidIcon);
        }

        Ok(hicon)
    }
}
