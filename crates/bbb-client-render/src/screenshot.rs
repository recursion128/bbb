#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;
use std::{io::Cursor, path::Path};

use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::{anyhow, Context};

pub use crate::render_core::screenshot::*;
use crate::Renderer;

#[cfg(target_arch = "wasm32")]
type WebPngEncodeDispatcher = fn(Box<dyn FnOnce() + Send + 'static>);

#[cfg(target_arch = "wasm32")]
static WEB_PNG_ENCODE_DISPATCHER: bbb_render_types::wasm_sync::SpinRwLock<
    Option<WebPngEncodeDispatcher>,
> = bbb_render_types::wasm_sync::SpinRwLock::new(None);

/// Registers a lock-free worker-relay dispatcher for offloading screenshot PNG
/// encoding off the browser `Window` thread without calling `rayon::spawn`
/// directly on the main thread (which can trap on `memory.atomic.wait32`).
#[cfg(target_arch = "wasm32")]
pub fn set_web_png_encode_dispatcher(f: fn(Box<dyn FnOnce() + Send + 'static>)) {
    *WEB_PNG_ENCODE_DISPATCHER.write() = Some(f);
}

#[derive(Debug, Clone)]
pub struct CapturedScreenshot {
    pub width: u32,
    pub height: u32,
    pub mime_type: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum RenderOutcome {
    Skipped,
    Presented {
        screenshot: Option<Result<CapturedScreenshot>>,
        capture: Option<Result<ScreenshotPixels>>,
    },
}

impl Renderer {
    pub(super) fn prepare_screenshot_copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
    ) -> Result<PendingScreenshot> {
        let width = self.config.width;
        let height = self.config.height;
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let pixel_format = ScreenshotPixelFormat::from_texture_format(self.config.format)?;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = padded_bytes_per_row as u64 * height as u64;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-client-screenshot-buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        Ok(PendingScreenshot {
            buffer,
            width,
            height,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
            pixel_format,
        })
    }

    /// Maps the copied buffer and converts it to tightly-packed RGBA8 rows.
    /// The single home of the 256-byte padded-row and BGRA-order handling —
    /// both the screenshot file path and the offscreen readback harness
    /// consume frames through here.
    pub(super) fn read_screenshot_pixels(
        &self,
        pending: PendingScreenshot,
    ) -> Result<ScreenshotPixels> {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = pending;
            anyhow::bail!("synchronous GPU readback is unavailable on wasm32");
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let slice = pending.buffer.slice(..);
            let (tx, rx) = mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
            self.device
                .poll(wgpu::PollType::wait_indefinitely())
                .context("wait for screenshot readback")?;
            rx.recv()
                .context("screenshot map callback dropped")?
                .map_err(|err| anyhow!("map screenshot buffer: {err}"))?;

            let mapped = slice
                .get_mapped_range()
                .map_err(|err| anyhow!("access mapped screenshot buffer: {err}"))?;
            let mut rgba = Vec::with_capacity((pending.width * pending.height * 4) as usize);
            for row in mapped
                .chunks(pending.padded_bytes_per_row as usize)
                .take(pending.height as usize)
            {
                let row = &row[..pending.unpadded_bytes_per_row as usize];
                pending.pixel_format.append_row_as_rgba(row, &mut rgba);
            }
            drop(mapped);
            pending.buffer.unmap();

            Ok(ScreenshotPixels {
                rgba,
                width: pending.width,
                height: pending.height,
            })
        }
    }

    pub(super) fn save_screenshot_pixels(
        pixels: &ScreenshotPixels,
        path: &Path,
    ) -> Result<CapturedScreenshot> {
        let format = image::ImageFormat::from_path(path)?;
        let bytes = Self::encode_screenshot_pixels(pixels, format)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &bytes)?;
        Ok(CapturedScreenshot {
            width: pixels.width,
            height: pixels.height,
            mime_type: format.to_mime_type(),
            bytes,
        })
    }

    pub(super) fn encode_screenshot_pixels(
        pixels: &ScreenshotPixels,
        format: image::ImageFormat,
    ) -> Result<Vec<u8>> {
        let mut encoded = Cursor::new(Vec::new());
        image::write_buffer_with_format(
            &mut encoded,
            &pixels.rgba,
            pixels.width,
            pixels.height,
            image::ExtendedColorType::Rgba8,
            format,
        )?;
        Ok(encoded.into_inner())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn request_web_screenshot(
        &mut self,
        callback: impl FnOnce(Result<CapturedScreenshot>) + 'static,
    ) {
        self.pending_web_screenshot = Some(Box::new(callback));
    }

    #[cfg(target_arch = "wasm32")]
    pub(super) fn spawn_web_screenshot_readback(
        pending: PendingScreenshot,
        callback: Box<dyn FnOnce(Result<CapturedScreenshot>) + 'static>,
    ) {
        use bbb_render_types::wasm_sync::wait_slot;

        let (map_tx, map_rx) = wait_slot();
        pending
            .buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                map_tx.complete(result);
            });

        wasm_bindgen_futures::spawn_local(async move {
            let map_result = map_rx
                .await
                .map_err(|err| anyhow::anyhow!("map screenshot buffer: {err}"))
                .and_then(|res| res.map_err(|err| anyhow::anyhow!("map screenshot buffer: {err}")));

            let pixels_result = map_result.and_then(|()| {
                let slice = pending.buffer.slice(..);
                let mapped = slice
                    .get_mapped_range()
                    .map_err(|err| anyhow::anyhow!("access mapped screenshot buffer: {err}"))?;
                let mut rgba = Vec::with_capacity((pending.width * pending.height * 4) as usize);
                for row in mapped
                    .chunks(pending.padded_bytes_per_row as usize)
                    .take(pending.height as usize)
                {
                    let row = &row[..pending.unpadded_bytes_per_row as usize];
                    pending.pixel_format.append_row_as_rgba(row, &mut rgba);
                }
                drop(mapped);
                pending.buffer.unmap();

                Ok(ScreenshotPixels {
                    rgba,
                    width: pending.width,
                    height: pending.height,
                })
            });

            let captured = match pixels_result {
                Err(err) => Err(err),
                Ok(pixels) => {
                    let encode_png = move || {
                        Self::encode_screenshot_pixels(&pixels, image::ImageFormat::Png).map(
                            |bytes| CapturedScreenshot {
                                width: pixels.width,
                                height: pixels.height,
                                mime_type: "image/png",
                                bytes,
                            },
                        )
                    };
                    if let Some(dispatch) = *WEB_PNG_ENCODE_DISPATCHER.read() {
                        let (encode_tx, encode_rx) = wait_slot();
                        dispatch(Box::new(move || {
                            encode_tx.complete(encode_png());
                        }));
                        encode_rx
                            .await
                            .unwrap_or_else(|err| Err(anyhow::anyhow!("encode screenshot: {err}")))
                    } else if rayon::current_thread_index().is_some() {
                        let (encode_tx, encode_rx) = wait_slot();
                        rayon::spawn(move || {
                            encode_tx.complete(encode_png());
                        });
                        encode_rx
                            .await
                            .unwrap_or_else(|err| Err(anyhow::anyhow!("encode screenshot: {err}")))
                    } else {
                        encode_png()
                    }
                }
            };

            callback(captured);
        });
    }
}
