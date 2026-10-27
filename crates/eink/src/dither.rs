//! Error-diffusion to black and white. Atkinson spreads three quarters of the error, which keeps highlights clean on a panel
//! (the same algorithm the page's 1-bit preview uses, `ui.dither`).

/// Gray (0..=255, one byte per pixel) to a 1-bit frame.
pub fn atkinson(gray: &[u8], w: usize, h: usize) -> crate::Frame {
    let mut lum: Vec<f32> = gray.iter().map(|&v| v as f32).collect();
    const K: [(isize, isize); 6] = [(1, 0), (2, 0), (-1, 1), (0, 1), (1, 1), (0, 2)];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let old = lum[i];
            let v = if old < 128.0 { 0.0 } else { 255.0 };
            lum[i] = v;
            let err = (old - v) / 8.0;
            for (dx, dy) in K {
                let (nx, ny) = (x as isize + dx, y + dy as usize);
                if nx >= 0 && (nx as usize) < w && ny < h {
                    lum[ny * w + nx as usize] += err;
                }
            }
        }
    }
    crate::Frame { w, h, px: lum.iter().map(|&v| if v < 128.0 { 0 } else { 255 }).collect() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_black_and_white_are_unchanged() {
        let g: Vec<u8> = (0..64).map(|i| if i % 2 == 0 { 0 } else { 255 }).collect();
        assert_eq!(atkinson(&g, 8, 8).px, g);
    }

    #[test]
    fn mid_gray_is_about_three_quarters_as_dense_as_it_looks() {
        // Atkinson drops a quarter of the error, so mid-gray comes out somewhat lighter or darker than 50%, but never solid
        let f = atkinson(&vec![128; 64 * 64], 64, 64);
        let black = f.px.iter().filter(|&&p| p == 0).count() as f64 / f.px.len() as f64;
        assert!((0.2..0.8).contains(&black), "{black}");
        // a light gray stays mostly white, a dark gray mostly black
        let light = atkinson(&vec![230; 64 * 64], 64, 64).px.iter().filter(|&&p| p == 0).count();
        let dark = atkinson(&vec![25; 64 * 64], 64, 64).px.iter().filter(|&&p| p == 0).count();
        assert!(light < 64 * 64 / 4 && dark > 64 * 64 * 3 / 4);
    }
}
