use gpui_tray_core::{BackendError, Error, Result};
use log::debug;
use windows::Win32::Graphics::Gdi::{
    BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS, DeleteObject,
    GetDC, ReleaseDC,
};
use windows::Win32::UI::WindowsAndMessaging::{CreateIconIndirect, DestroyIcon, HICON, ICONINFO};

pub(crate) struct DecodedIcon {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub(crate) struct OwnedIcon(pub(crate) HICON);

impl Drop for OwnedIcon {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = DestroyIcon(self.0);
            }
        }
    }
}

pub(crate) fn decode_icon(image: &gpui::Image) -> Result<DecodedIcon> {
    let start = std::time::Instant::now();
    debug!(
        "decode start, bytes={}, format={:?}",
        image.bytes.len(),
        image.format
    );
    let decoded = image::load_from_memory(&image.bytes).map_err(|_| Error::InvalidIcon)?;
    let resized = decoded.resize_to_fill(32, 32, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8().into_raw();
    debug!("windows icon: decode finish in {:?}", start.elapsed());
    Ok(DecodedIcon {
        rgba,
        width: 32,
        height: 32,
    })
}

pub(crate) fn create_hicon(decoded: &DecodedIcon) -> Result<OwnedIcon> {
    let start = std::time::Instant::now();
    debug!("create_hicon start, {}x{}", decoded.width, decoded.height);
    unsafe {
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return Err(BackendError::platform("GetDC", "invalid device context").into());
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: decoded.width as i32,
                biHeight: -(decoded.height as i32),
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
            DIB_RGB_COLORS,
            &mut bits as *mut _ as *mut *mut std::ffi::c_void,
            None,
            0,
        )
        .map_err(|err| BackendError::platform("CreateDIBSection", format!("{err:?}")))?;

        let bgra: Vec<u8> = decoded
            .rgba
            .chunks_exact(4)
            .flat_map(|chunk| [chunk[2], chunk[1], chunk[0], chunk[3]])
            .collect();
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits, bgra.len());

        let _ = ReleaseDC(None, hdc);

        let mut and_mask = vec![0xFFu8; (decoded.width.div_ceil(8) * decoded.height) as usize];
        for (i, chunk) in decoded.rgba.chunks_exact(4).enumerate() {
            let alpha = chunk[3];
            if alpha < 128 {
                let x = (i % decoded.width as usize) as u32;
                let y = (i / decoded.width as usize) as u32;
                let byte_index = (y * decoded.width.div_ceil(8) + (x / 8)) as usize;
                let bit_index = x % 8;
                and_mask[byte_index] &= !(1 << (7 - bit_index));
            }
        }

        let hmask = CreateBitmap(
            decoded.width as i32,
            decoded.height as i32,
            1,
            1,
            Some(and_mask.as_ptr() as *const _),
        );

        if hmask.is_invalid() {
            let _ = DeleteObject(hbitmap.into());
            return Err(
                BackendError::platform("CreateBitmap", "failed to create mask bitmap").into(),
            );
        }

        let icon_info = ICONINFO {
            fIcon: windows::Win32::Foundation::TRUE,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hmask,
            hbmColor: hbitmap,
        };

        let hicon = CreateIconIndirect(&icon_info)
            .map_err(|err| BackendError::platform("CreateIconIndirect", format!("{err:?}")))?;

        let _ = DeleteObject(hbitmap.into());
        let _ = DeleteObject(hmask.into());

        if hicon.is_invalid() {
            return Err(Error::InvalidIcon);
        }

        debug!("create_hicon finish in {:?}", start.elapsed());
        Ok(OwnedIcon(hicon))
    }
}
