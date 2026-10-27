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

/// Gray to the panel's 16 levels (0x00, 0x11 ... 0xFF), dithered *between* those levels so a gradient stays smooth instead of banding. The threshold for a
/// pixel is noise that depends only on its position (interleaved gradient noise), so a pixel's result is a pure function of its own gray value and place:
/// nothing spreads to neighbours, nothing is re-rolled when something elsewhere changes, and a repaint of unchanged content changes not one pixel.
/// Pure black and pure white always stay themselves, so text and rules are untouched.
pub fn levels16(gray: &[u8], w: usize, h: usize) -> crate::Frame {
    let px = gray
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let (x, y) = ((i % w) as f32, (i / w) as f32);
            let t = (52.982_92 * ((0.067_110_56 * x + 0.005_837_15 * y).fract())).fract();
            let level = ((v as f32 / 17.0) + t).floor().min(15.0) as u8;
            level * 17
        })
        .collect();
    crate::Frame { w, h, px }
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
    fn sixteen_levels_keep_black_and_white_and_dither_between_the_rest() {
        let f = levels16(&[0, 255, 0, 255], 4, 1);
        assert_eq!(f.px, vec![0, 255, 0, 255]);
        // a flat patch of a level between two of the panel's: only those two neighbours appear, in about the right proportion
        let (w, h) = (128, 128);
        let g = levels16(&vec![136u8; w * h], w, h); // 136 = 8.0 levels: exactly level 8 (0x88)
        assert!(g.px.iter().all(|&p| p == 0x88));
        let g = levels16(&vec![144u8; w * h], w, h); // 144 / 17 = 8.47: between 0x88 and 0x99
        assert!(g.px.iter().all(|&p| p == 0x88 || p == 0x99));
        let mean = g.px.iter().map(|&p| p as f64).sum::<f64>() / g.px.len() as f64;
        assert!((mean - 144.0).abs() < 2.0, "the dither keeps the average: {mean}");
    }

    #[test]
    fn a_pixel_depends_only_on_its_own_value_and_place() {
        let (w, h) = (64, 64);
        let mut gray: Vec<u8> = (0..w * h).map(|i| ((i % w) * 255 / w) as u8).collect();
        let before = levels16(&gray, w, h);
        gray[30 * w + 30] = 255;
        let after = levels16(&gray, w, h);
        let differing: Vec<usize> = (0..w * h).filter(|&i| before.px[i] != after.px[i]).collect();
        assert_eq!(differing, vec![30 * w + 30]);
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
