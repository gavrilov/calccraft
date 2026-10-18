//! The app icon: a small calculator, drawn in code.
//!
//! The same drawing is the window icon (`app::window_icon`) and the icon embedded
//! in the executable (`build.rs` includes this file). That's why it uses only `std`.
//! The design sits on a 64 × 64 grid and is rendered at any size with 4 × 4
//! supersampling, so every size of the .ico file is drawn sharp, not scaled.

// The dark palette's accent and number key colors.
const ACCENT: [u8; 3] = [0xDB, 0x9E, 0xE5];
const BODY: [u8; 3] = [0x44, 0x37, 0x44];
const KEY: [u8; 3] = [0xE6, 0xE6, 0xE6];

/// A rounded rectangle on the 64 × 64 grid.
struct Shape {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
    radius: f32,
}

impl Shape {
    const fn new(left: f32, top: f32, width: f32, height: f32, radius: f32) -> Self {
        Shape {
            left,
            top,
            right: left + width,
            bottom: top + height,
            radius,
        }
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        if x < self.left || x > self.right || y < self.top || y > self.bottom {
            return false;
        }
        let dx = (self.left + self.radius - x)
            .max(x - (self.right - self.radius))
            .max(0.0);
        let dy = (self.top + self.radius - y)
            .max(y - (self.bottom - self.radius))
            .max(0.0);
        dx * dx + dy * dy <= self.radius * self.radius
    }
}

const BODY_SHAPE: Shape = Shape::new(10.0, 4.0, 44.0, 56.0, 7.0);
const SCREEN: Shape = Shape::new(16.0, 10.0, 32.0, 12.0, 2.0);

/// Color at a point of the 64 × 64 grid; `None` is transparent.
fn color_at(x: f32, y: f32) -> Option<[u8; 3]> {
    if !BODY_SHAPE.contains(x, y) {
        return None;
    }
    if SCREEN.contains(x, y) {
        return Some(ACCENT);
    }
    // Three columns and four rows of keys; the bottom right one is the accent "=".
    for row in 0..4 {
        for col in 0..3 {
            let key = Shape::new(
                16.0 + col as f32 * 11.5,
                27.0 + row as f32 * 8.5,
                9.0,
                6.5,
                1.5,
            );
            if key.contains(x, y) {
                return Some(if (row, col) == (3, 2) { ACCENT } else { KEY });
            }
        }
    }
    Some(BODY)
}

/// The icon at `size` × `size` pixels, as RGBA with straight (not premultiplied) alpha.
pub fn rgba(size: u32) -> Vec<u8> {
    const SAMPLES: u32 = 4;
    let scale = 64.0 / size as f32;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0u32; 3];
            let mut covered = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let gx = (x as f32 + (sx as f32 + 0.5) / SAMPLES as f32) * scale;
                    let gy = (y as f32 + (sy as f32 + 0.5) / SAMPLES as f32) * scale;
                    if let Some(color) = color_at(gx, gy) {
                        for (total, channel) in sum.iter_mut().zip(color) {
                            *total += u32::from(channel);
                        }
                        covered += 1;
                    }
                }
            }
            // Average only the covered samples, so edges keep their color and fade by alpha.
            let channel = |total: u32| total.checked_div(covered).unwrap_or(0) as u8;
            let alpha = (covered * 255 / (SAMPLES * SAMPLES)) as u8;
            pixels.extend([channel(sum[0]), channel(sum[1]), channel(sum[2]), alpha]);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(pixels: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * size + x) * 4) as usize;
        pixels[i..i + 4].try_into().unwrap()
    }

    #[test]
    fn renders_any_size() {
        for size in [16, 32, 64, 256] {
            assert_eq!(rgba(size).len(), (size * size * 4) as usize);
        }
    }

    #[test]
    fn corners_are_transparent_and_screen_is_accent() {
        let pixels = rgba(64);
        assert_eq!(pixel(&pixels, 64, 0, 0)[3], 0);
        assert_eq!(pixel(&pixels, 64, 32, 16), [0xDB, 0x9E, 0xE5, 0xFF]);
        assert_eq!(pixel(&pixels, 64, 12, 40), [0x44, 0x37, 0x44, 0xFF]);
    }

    /// Writes the web version's icons into `assets/web/` (the tab's icon and the icon iOS
    /// puts on the home screen), which `index.html` links. Run it after changing the icon:
    /// `cargo test write_web_icons -- --ignored`.
    #[test]
    #[ignore]
    fn write_web_icons() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/web");
        std::fs::create_dir_all(&dir).unwrap();
        for size in [32, 180, 192, 512] {
            let file = std::fs::File::create(dir.join(format!("icon-{size}.png"))).unwrap();
            let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), size, size);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&rgba(size)).unwrap();
        }
    }
}
