//! From the display page to a 1-bit panel: dithering, placing the 800x480 scene on the panel's geometry, finding what changed, and serving
//! the result over VNC (RFB) so a reMarkable 2 (or any viewer) can show it.

pub mod cdp;
pub mod dither;
pub mod rfb;

/// An image kept one byte per pixel: 0 is black, 255 is white; for a 1-bit panel only those two, for a grayscale one 16 levels.
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

/// What changed, as one box per 64x64 block that differs (the tight box of the differing pixels within it), so that two distant changes (the
/// seconds, a graph) are two small updates and not one box over everything between them.
pub fn damage_rects(a: &Frame, b: &Frame) -> Vec<Rect> {
    assert_eq!((a.w, a.h), (b.w, b.h));
    const T: usize = 64;
    let mut out = Vec::new();
    for ty in (0..a.h).step_by(T) {
        for tx in (0..a.w).step_by(T) {
            let (x1, y1) = ((tx + T).min(a.w), (ty + T).min(a.h));
            let (mut bx0, mut by0, mut bx1, mut by1) = (usize::MAX, usize::MAX, 0, 0);
            for y in ty..y1 {
                let (ra, rb) = (&a.px[y * a.w + tx..y * a.w + x1], &b.px[y * a.w + tx..y * a.w + x1]);
                if ra == rb {
                    continue;
                }
                let first = ra.iter().zip(rb).position(|(p, q)| p != q).unwrap();
                let last = ra.iter().zip(rb).rposition(|(p, q)| p != q).unwrap();
                bx0 = bx0.min(tx + first);
                bx1 = bx1.max(tx + last);
                by0 = by0.min(y);
                by1 = by1.max(y);
            }
            if bx0 != usize::MAX {
                out.push(Rect { x: bx0, y: by0, w: bx1 - bx0 + 1, h: by1 - by0 + 1 });
            }
        }
    }
    out
}

/// A frame as an 8-bit gray PNG (for looking at what is sent).
pub fn write_png(path: &str, f: &Frame) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), f.w as u32, f.h as u32);
    enc.set_color(png::ColorType::Grayscale);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut w| w.write_image_data(&f.px)).map_err(|e| e.to_string())
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
    /// The panel's native 1404x1872 frame with the landscape scene turned a quarter clockwise: for the tablet held in landscape, showing a
    /// viewer (such as AppLoad's) that hands over the portrait frame as it is.
    Remarkable2TurnedCw,
    /// The same turned a quarter anticlockwise.
    Remarkable2TurnedCcw,
}

/// The scene's size in CSS pixels.
pub const SCENE: (usize, usize) = (800, 480);

impl Panel {
    pub fn size(self) -> (usize, usize) {
        match self {
            Panel::Native => SCENE,
            Panel::Remarkable2Landscape => (1872, 1404),
            Panel::Remarkable2Portrait | Panel::Remarkable2TurnedCw | Panel::Remarkable2TurnedCcw => (1404, 1872),
        }
    }

    /// The landscape panel the scene is laid out on before a turned panel rotates it (the panel itself when it is not turned).
    fn laid_out_on(self) -> Panel {
        match self {
            Panel::Remarkable2TurnedCw | Panel::Remarkable2TurnedCcw => Panel::Remarkable2Landscape,
            other => other,
        }
    }

    /// Whether the panel is shown in 1 bit (dithered). All of them are: the dither is local (see `dither::atkinson_blocks`), so a change in one place
    /// never re-rolls the dots somewhere else. (Straight 16-gray output, `dither::levels16`, is kept for a viewer that wants it, but it looked
    /// worse on the reMarkable than the dithered gradients, so nothing uses it.)
    pub fn dithered(self) -> bool {
        true
    }

    /// The scene's size for this panel, in CSS pixels: the design's 800x480, or 800x600 on the reMarkable 2 so that it is 4:3 like the panel and
    /// fills it edge to edge (the layout's rows are flexible; nothing is stretched).
    pub fn scene(self) -> (usize, usize) {
        match self {
            Panel::Native => SCENE,
            _ => (SCENE.0, 600),
        }
    }

    /// Where a point of the panel's frame (pixels) is in the scene (CSS pixels): undoes the placement, the quarter turn and the scale. None off the scene.
    pub fn to_scene(self, fx: usize, fy: usize) -> Option<(f64, f64)> {
        let (lw, lh) = self.laid_out_on().size();
        let (x, y) = match self {
            Panel::Remarkable2TurnedCw => (fy as isize, lh as isize - 1 - fx as isize),
            Panel::Remarkable2TurnedCcw => (lw as isize - 1 - fy as isize, fx as isize),
            _ => (fx as isize, fy as isize),
        };
        let s = self.scale();
        let (scene_w, scene_h) = self.scene();
        let (ox, oy) = self.place(((scene_w as f64 * s) as usize, (scene_h as f64 * s) as usize));
        let (sx, sy) = ((x - ox as isize) as f64 / s, (y - oy as isize) as f64 / s);
        (sx >= 0.0 && sy >= 0.0 && sx < scene_w as f64 && sy < scene_h as f64).then_some((sx, sy))
    }

    /// The display page to hold open for this panel.
    pub fn page(self, display: &str) -> String {
        let (w, h) = self.scene();
        format!("{display}/live?panel&size={w}x{h}")
    }

    /// The factor the scene is rendered at so it fills the panel's width (rendering at the target scale, not stretching afterwards, keeps text sharp).
    pub fn scale(self) -> f64 {
        self.laid_out_on().size().0 as f64 / SCENE.0 as f64
    }

    /// Where the rendered scene sits on the panel: horizontally filling, vertically centred.
    pub fn place(self, rendered: (usize, usize)) -> (usize, usize) {
        let (pw, ph) = self.laid_out_on().size();
        (pw.saturating_sub(rendered.0) / 2, ph.saturating_sub(rendered.1) / 2)
    }
}

/// Gray pixels (one byte each, 0..=255) of size `rw x rh` put on a white panel, then dithered to 1 bit (or quantised to 16 grays, per `Panel::dithered`).
pub fn compose(gray: &[u8], rw: usize, rh: usize, panel: Panel) -> Frame {
    let (pw, ph) = panel.laid_out_on().size();
    let (ox, oy) = panel.place((rw, rh));
    let mut canvas = vec![255u8; pw * ph];
    for y in 0..rh.min(ph.saturating_sub(oy)) {
        let n = rw.min(pw - ox);
        canvas[(oy + y) * pw + ox..(oy + y) * pw + ox + n].copy_from_slice(&gray[y * rw..y * rw + n]);
    }
    let (canvas, w, h) = match panel {
        Panel::Remarkable2TurnedCw => (turn(&canvas, pw, ph, true), ph, pw),
        Panel::Remarkable2TurnedCcw => (turn(&canvas, pw, ph, false), ph, pw),
        _ => (canvas, pw, ph),
    };
    if panel.dithered() { dither::atkinson_blocks(&canvas, w, h) } else { dither::levels16(&canvas, w, h) }
}

/// A quarter turn of a `w x h` image: clockwise or anticlockwise; the result is `h x w`.
fn turn(src: &[u8], w: usize, h: usize, clockwise: bool) -> Vec<u8> {
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let (nx, ny) = if clockwise { (h - 1 - y, x) } else { (y, w - 1 - x) };
            out[ny * h + nx] = src[y * w + x];
        }
    }
    out
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
    fn the_remarkables_scene_is_four_by_three_and_fills_the_panel_edge_to_edge() {
        assert_eq!(Panel::Remarkable2Landscape.size(), (1872, 1404));
        assert_eq!(Panel::Native.scene(), (800, 480));
        let s = Panel::Remarkable2Landscape.scale();
        assert!((s - 2.34).abs() < 1e-9);
        let scene = Panel::Remarkable2Landscape.scene();
        let rendered = ((scene.0 as f64 * s) as usize, (scene.1 as f64 * s) as usize);
        assert_eq!(rendered, (1872, 1404));
        assert_eq!(Panel::Remarkable2Landscape.place(rendered), (0, 0));
        assert_eq!(Panel::Remarkable2Landscape.page("http://d"), "http://d/live?panel&size=800x600");
        let f = compose(&vec![0; rendered.0 * rendered.1], rendered.0, rendered.1, Panel::Remarkable2Landscape);
        assert_eq!((f.w, f.h), (1872, 1404));
        assert_eq!(f.px[0], 0, "no margin: the scene reaches the corner");
        assert_eq!(f.px[700 * 1872 + 900], 0, "the scene is black where it was");
    }

    #[test]
    fn a_turned_panel_is_the_portrait_frame_with_the_scene_quarter_turned() {
        assert_eq!(Panel::Remarkable2TurnedCw.size(), (1404, 1872));
        assert_eq!(Panel::Remarkable2TurnedCw.scale(), Panel::Remarkable2Landscape.scale());
        // a 3x2 image with its top-left pixel set: clockwise it lands top-right of the 2x3 result, anticlockwise bottom-left
        let img = [0u8, 9, 9, 9, 9, 9];
        let cw = turn(&img, 3, 2, true);
        assert_eq!((cw[1], cw.iter().filter(|v| **v == 0).count()), (0, 1));
        let ccw = turn(&img, 3, 2, false);
        assert_eq!(ccw[2 * 2], 0);
        // and a scene whose top-left is black shows black in the portrait frame's top-right (clockwise)
        let s = Panel::Remarkable2TurnedCw.scale();
        let scene = Panel::Remarkable2TurnedCw.scene();
        let (rw, rh) = ((scene.0 as f64 * s) as usize, (scene.1 as f64 * s) as usize);
        let mut g = vec![255u8; rw * rh];
        for y in 0..40 { for x in 0..40 { g[y * rw + x] = 0; } }
        let f = compose(&g, rw, rh, Panel::Remarkable2TurnedCw);
        assert_eq!((f.w, f.h), (1404, 1872));
        assert_eq!(f.px[20 * 1404 + (1404 - 1 - 20)], 0);
        assert_eq!(f.px[20 * 1404 + 20], 255);
    }

    #[test]
    fn a_touch_on_the_panel_finds_its_place_in_the_scene() {
        // landscape and portrait: just the scale (and, in portrait, the centring)
        assert_eq!(Panel::Remarkable2Landscape.to_scene(1872 / 2, 1404 / 2), Some((400.0, 300.0)));
        // turned: the same scene point, found through the quarter turn that compose makes
        for panel in [Panel::Remarkable2TurnedCw, Panel::Remarkable2TurnedCcw] {
            let s = panel.scale();
            let scene = panel.scene();
            let (rw, rh) = ((scene.0 as f64 * s) as usize, (scene.1 as f64 * s) as usize);
            let mut g = vec![255u8; rw * rh];
            for y in 600..640 { for x in 300..340 { g[y * rw + x] = 0; } }   // a black patch at landscape (300..340, 600..640)
            let f = compose(&g, rw, rh, panel);
            let i = f.px.iter().position(|p| *p == 0).unwrap();
            let (sx, sy) = panel.to_scene(i % f.w, i / f.w).unwrap();
            assert!((sx * s - 300.0).abs() < 41.0 && (sy * s - 600.0).abs() < 41.0, "{panel:?}: {sx},{sy}");
        }
        assert_eq!(Panel::Remarkable2TurnedCcw.to_scene(0, 5000), None);
    }

    #[test]
    fn distant_changes_are_separate_small_updates() {
        let a = Frame::white(200, 200);
        let mut b = a.clone();
        b.px[3 * 200 + 5] = 0;
        b.px[190 * 200 + 190] = 0;
        let r = damage_rects(&a, &b);
        assert_eq!(r, vec![Rect { x: 5, y: 3, w: 1, h: 1 }, Rect { x: 190, y: 190, w: 1, h: 1 }]);
        assert!(damage_rects(&a, &a).is_empty());
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
