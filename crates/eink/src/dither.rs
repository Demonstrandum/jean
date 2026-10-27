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

/// Gray to 16 levels (0x00, 0x11 ... 0xFF), nothing else: the same input always gives the same pixel, so a repaint of unchanged content changes nothing.
pub fn levels16(gray: &[u8], w: usize, h: usize) -> crate::Frame {
    crate::Frame { w, h, px: gray.iter().map(|&v| (v & 0xF0) | (v >> 4)).collect() }
}

/// Atkinson in independent 64x64 blocks. Error diffusion over a whole frame makes one changed pixel re-roll every dot after it, so a ticking
/// seconds glyph would repaint the gradients below it; with the error reset at each block, only the blocks whose input changed change.
pub fn atkinson_blocks(gray: &[u8], w: usize, h: usize) -> crate::Frame {
    const T: usize = 64;
    let mut px = vec![255u8; w * h];
    for ty in (0..h).step_by(T) {
        for tx in (0..w).step_by(T) {
            let (bw, bh) = ((w - tx).min(T), (h - ty).min(T));
            let mut block = Vec::with_capacity(bw * bh);
            for y in 0..bh {
                block.extend_from_slice(&gray[(ty + y) * w + tx..(ty + y) * w + tx + bw]);
            }
            let out = atkinson(&block, bw, bh);
            for y in 0..bh {
                px[(ty + y) * w + tx..(ty + y) * w + tx + bw].copy_from_slice(&out.px[y * bw..(y + 1) * bw]);
            }
        }
    }
    crate::Frame { w, h, px }
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

    #[test]
    fn sixteen_levels_keep_black_and_white_and_quantise_the_rest() {
        let f = levels16(&[0, 255, 128, 17], 4, 1);
        assert_eq!(f.px, vec![0x00, 0xFF, 0x88, 0x11]);
    }

    #[test]
    fn a_change_in_one_block_leaves_the_others_alone() {
        let (w, h) = (192, 128);
        let mut gray: Vec<u8> = (0..w * h).map(|i| ((i % w) * 255 / w) as u8).collect();
        let before = atkinson_blocks(&gray, w, h);
        gray[30 * w + 30] = 255; // one pixel in the top-left block (a dark patch of the gradient)
        let after = atkinson_blocks(&gray, w, h);
        let changed = crate::damage_rects(&before, &after);
        assert!(!changed.is_empty() && changed.iter().all(|r| r.x + r.w <= 64 && r.y + r.h <= 64), "{changed:?}");
    }
}
