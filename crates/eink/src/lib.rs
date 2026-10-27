//! From the display page to a 1-bit panel: dithering, placing the 800x480 scene on the panel's geometry, finding what changed, and serving
//! the result over VNC (RFB) so a reMarkable 2 (or any viewer) can show it.

pub mod cdp;
pub mod dither;
pub mod rfb;

/// A 1-bit image kept one byte per pixel: 0 is black, 255 is white.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u8>,
}

impl Frame {
    pub fn white(w: usize, h: usize) -> Frame {
        Frame { w, h, px: vec![255; w * h] }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

/// The bounding box of the pixels that differ, or None if the frames are identical. Frames must be the same size.
pub fn damage(a: &Frame, b: &Frame) -> Option<Rect> {
    assert_eq!((a.w, a.h), (b.w, b.h));
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for y in 0..a.h {
        let (ra, rb) = (&a.px[y * a.w..(y + 1) * a.w], &b.px[y * a.w..(y + 1) * a.w]);
        if ra == rb {
            continue;
        }
        let first = ra.iter().zip(rb).position(|(p, q)| p != q).unwrap();
        let last = ra.iter().zip(rb).rposition(|(p, q)| p != q).unwrap();
        x0 = x0.min(first);
        x1 = x1.max(last);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }
    (x0 != usize::MAX).then(|| Rect { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 })
}

/// The panel the scene is shown on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    /// The scene's own size, 800x480.
    Native,
    /// reMarkable 2, held in landscape: 1872x1404.
    Remarkable2Landscape,
    /// reMarkable 2 in portrait: 1404x1872, the scene across the top.
    Remarkable2Portrait,
}

/// The scene's size in CSS pixels.
pub const SCENE: (usize, usize) = (800, 480);

impl Panel {
    pub fn size(self) -> (usize, usize) {
        match self {
            Panel::Native => SCENE,
            Panel::Remarkable2Landscape => (1872, 1404),
            Panel::Remarkable2Portrait => (1404, 1872),
        }
    }

    /// The factor the scene is rendered at so it fills the panel's width (rendering at the target scale, not stretching afterwards, keeps text sharp).
    pub fn scale(self) -> f64 {
        self.size().0 as f64 / SCENE.0 as f64
    }

    /// Where the rendered scene sits on the panel: horizontally filling, vertically centred.
    pub fn place(self, rendered: (usize, usize)) -> (usize, usize) {
        let (pw, ph) = self.size();
        (pw.saturating_sub(rendered.0) / 2, ph.saturating_sub(rendered.1) / 2)
    }
}

/// Gray pixels (one byte each, 0..=255) of size `rw x rh` put on a white panel, then dithered to 1 bit.
pub fn compose(gray: &[u8], rw: usize, rh: usize, panel: Panel) -> Frame {
    let (pw, ph) = panel.size();
    let (ox, oy) = panel.place((rw, rh));
    let mut canvas = vec![255u8; pw * ph];
    for y in 0..rh.min(ph.saturating_sub(oy)) {
        let n = rw.min(pw - ox);
        canvas[(oy + y) * pw + ox..(oy + y) * pw + ox + n].copy_from_slice(&gray[y * rw..y * rw + n]);
    }
    dither::atkinson(&canvas, pw, ph)
}

/// Decode a PNG to gray (luma over white for any transparency): the pixels and the size.
pub fn gray_from_png(bytes: &[u8]) -> Result<(Vec<u8>, usize, usize), String> {
    let mut dec = png::Decoder::new(bytes);
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (w, h) = (info.width as usize, info.height as usize);
    let ch = info.color_type.samples();
    let luma = |r: f32, g: f32, b: f32| 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let gray = (0..w * h)
        .map(|i| {
            let p = &buf[i * ch..(i + 1) * ch];
            let (v, a) = match ch {
                1 => (p[0] as f32, 255.0),
                2 => (p[0] as f32, p[1] as f32),
                3 => (luma(p[0] as f32, p[1] as f32, p[2] as f32), 255.0),
                _ => (luma(p[0] as f32, p[1] as f32, p[2] as f32), p[3] as f32),
            };
            (v * a / 255.0 + 255.0 * (1.0 - a / 255.0)).round() as u8
        })
        .collect();
    Ok((gray, w, h))
}

/// One screenshot of the page, dithered and placed on the panel.
pub fn capture(browser: &mut cdp::Browser, panel: Panel) -> Result<Frame, String> {
    let (gray, w, h) = gray_from_png(&browser.screenshot()?)?;
    Ok(compose(&gray, w, h, panel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_is_the_box_around_what_changed() {
        let a = Frame::white(10, 8);
        assert_eq!(damage(&a, &a), None);
        let mut b = a.clone();
        b.px[2 * 10 + 3] = 0;
        b.px[5 * 10 + 7] = 0;
        assert_eq!(damage(&a, &b), Some(Rect { x: 3, y: 2, w: 5, h: 4 }));
    }

    #[test]
    fn the_scene_fills_the_remarkables_width_and_sits_centred() {
        assert_eq!(Panel::Remarkable2Landscape.size(), (1872, 1404));
        let s = Panel::Remarkable2Landscape.scale();
        assert!((s - 2.34).abs() < 1e-9);
        let rendered = ((800.0 * s) as usize, (480.0 * s) as usize);
        assert_eq!(rendered, (1872, 1123));
        assert_eq!(Panel::Remarkable2Landscape.place(rendered), (0, 140));
        let f = compose(&vec![0; rendered.0 * rendered.1], rendered.0, rendered.1, Panel::Remarkable2Landscape);
        assert_eq!((f.w, f.h), (1872, 1404));
        assert_eq!(f.px[0], 255, "the margin is white");
        assert_eq!(f.px[700 * 1872 + 900], 0, "the scene is black where it was");
    }

    #[test]
    fn a_png_becomes_gray_over_white() {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, 2, 1);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            let mut w = e.write_header().unwrap();
            w.write_image_data(&[0, 0, 0, 255, 0, 0, 0, 0]).unwrap();
        }
        let (g, w, h) = gray_from_png(&out).unwrap();
        assert_eq!((w, h, g), (2, 1, vec![0, 255]));
    }
}
