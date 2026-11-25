//! Native spatial/temporal denoising, restored from the interrupted DW2 work.
//! Neighbours are decoded one at a time. Filtering uses straight linear f32,
//! preserves original alpha and never clamps HDR to display white.
use super::{chv, fv};
use crate::{Image, effects::FxCtx};
use filmcraft_project::EffectInstance;
use rayon::prelude::*;

#[derive(Clone, Copy)]
struct Settings {
    spatial: f32,
    temporal: f32,
    detail: f32,
    quality: u32,
    scale: f32,
}

fn amount(e: &EffectInstance, name: &str, cx: &FxCtx) -> f32 {
    let n = fv(e, name, cx);
    if n.is_finite() { n.clamp(0.0, 100.0) / 100.0 } else { 0.0 }
}
fn straight(p: [f32; 4]) -> [f32; 3] {
    if p[3] > 1e-6 && p.iter().all(|v| v.is_finite()) { [p[0] / p[3], p[1] / p[3], p[2] / p[3]] } else { [0.0; 3] }
}
fn luma(c: [f32; 3]) -> f32 {
    c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
}
fn components(c: [f32; 3]) -> [f32; 3] {
    let y = luma(c);
    [y, c[0] - y, c[2] - y]
}
fn rgb(c: [f32; 3]) -> [f32; 3] {
    [c[0] + c[1], c[0] - (0.2126 * c[1] + 0.0722 * c[2]) / 0.7152, c[0] + c[2]]
}
/// Robust high-pass MAD; texture outliers do not raise the threshold everywhere.
fn noise_sigma(img: &Image) -> f32 {
    let stride = ((img.w * img.h / 16_384) as f64).sqrt().ceil().max(1.0) as usize;
    let mut residuals = Vec::new();
    for y in (1..img.h.saturating_sub(1)).step_by(stride) {
        for x in (1..img.w.saturating_sub(1)).step_by(stride) {
            let p = img.get(x, y);
            if p[3] < 0.99 {
                continue;
            }
            let c = luma(straight(p));
            let n = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)].map(|(xx, yy)| luma(straight(img.get(xx, yy))));
            let v = (4.0 * c - n.iter().sum::<f32>()).abs() / 4.472_136;
            if v.is_finite() {
                residuals.push(v);
            }
        }
    }
    if residuals.is_empty() {
        return 0.004;
    }
    let middle = residuals.len() / 2;
    let (_, value, _) = residuals.select_nth_unstable_by(middle, f32::total_cmp);
    (*value / 0.6745).clamp(0.001, 0.15)
}
fn texture(img: &Image, x: usize, y: usize, sigma: f32, detail: f32) -> f32 {
    let c = luma(straight(img.get(x, y)));
    let dx = (luma(straight(img.get_clamped(x as isize + 1, y as isize))) - luma(straight(img.get_clamped(x as isize - 1, y as isize)))).abs();
    let dy = (luma(straight(img.get_clamped(x as isize, y as isize + 1))) - luma(straight(img.get_clamped(x as isize, y as isize - 1)))).abs();
    let edge = (dx.max(dy) - sigma * 2.0).max(0.0);
    1.0 / (1.0 + detail * edge / (sigma * 2.0 + c.abs() * 0.01 + 0.001))
}
#[derive(Clone)]
struct Plane {
    w: usize,
    h: usize,
    values: Vec<f32>,
    scale: f32,
}
impl Plane {
    fn from_image(img: &Image) -> Self {
        let step = img.w.max(img.h).div_ceil(768).max(1);
        let (w, h) = (img.w.div_ceil(step), img.h.div_ceil(step));
        let mut values = vec![0.0; w * h];
        values.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            for (x, out) in row.iter_mut().enumerate() {
                let mut sum = 0.0;
                let mut count = 0.0;
                for yy in y * step..((y + 1) * step).min(img.h) {
                    for xx in x * step..((x + 1) * step).min(img.w) {
                        sum += luma(straight(img.get(xx, yy)));
                        count += 1.0;
                    }
                }
                let v = sum / count;
                *out = v.signum() * v.abs().sqrt();
            }
        });
        Self { w, h, values, scale: step as f32 }
    }
    fn half(&self) -> Self {
        let (w, h) = (self.w.div_ceil(2), self.h.div_ceil(2));
        let mut values = vec![0.0; w * h];
        for y in 0..h {
            for x in 0..w {
                values[y * w + x] = (self.at((x * 2) as f32, (y * 2) as f32)
                    + self.at((x * 2 + 1) as f32, (y * 2) as f32)
                    + self.at((x * 2) as f32, (y * 2 + 1) as f32)
                    + self.at((x * 2 + 1) as f32, (y * 2 + 1) as f32))
                    * 0.25;
            }
        }
        Self { w, h, values, scale: self.scale * 2.0 }
    }
    fn at(&self, x: f32, y: f32) -> f32 {
        let (x, y) = (x.clamp(0.0, (self.w - 1) as f32), y.clamp(0.0, (self.h - 1) as f32));
        let (xx, yy) = (x as usize, y as usize);
        let (nx, ny) = ((xx + 1).min(self.w - 1), (yy + 1).min(self.h - 1));
        let (fx, fy) = (x - xx as f32, y - yy as f32);
        let a = self.values[yy * self.w + xx] * (1.0 - fx) + self.values[yy * self.w + nx] * fx;
        let b = self.values[ny * self.w + xx] * (1.0 - fx) + self.values[ny * self.w + nx] * fx;
        a * (1.0 - fy) + b * fy
    }
}
#[derive(Clone, Copy, Default)]
struct Motion {
    dx: f32,
    dy: f32,
    confidence: f32,
}
fn patch_error(a: &Plane, b: &Plane, centre: [f32; 2], offset: [f32; 2]) -> f32 {
    let mut sum = 0.0;
    for y in -2..=2 {
        for x in -2..=2 {
            let (px, py) = (centre[0] + x as f32, centre[1] + y as f32);
            sum += (a.at(px, py) - b.at(px + offset[0], py + offset[1])).abs().min(0.3);
        }
    }
    sum / 25.0
}
fn search(a: &Plane, b: &Plane, centre: [f32; 2], initial: [f32; 2], radius: i32, step: f32) -> ([f32; 2], f32) {
    let mut best = initial;
    let mut error = patch_error(a, b, centre, initial) + 0.0001 * (initial[0].abs() + initial[1].abs());
    for y in -radius..=radius {
        for x in -radius..=radius {
            let d = [initial[0] + x as f32 * step, initial[1] + y as f32 * step];
            let e = patch_error(a, b, centre, d) + 0.0001 * (d[0].abs() + d[1].abs());
            if e < error {
                error = e;
                best = d;
            }
        }
    }
    (best, error)
}
struct MotionField {
    w: usize,
    h: usize,
    block: f32,
    motions: Vec<Motion>,
}
impl MotionField {
    fn estimate(a: &Plane, b: &Plane, maximum: bool) -> Option<Self> {
        let (ac, bc) = (a.half(), b.half());
        let block = 8usize;
        let (w, h) = (a.w.div_ceil(block), a.h.div_ceil(block));
        let mut motions = vec![Motion::default(); w * h];
        let cancel = filmcraft_media::cancel::Context::current();
        motions.par_chunks_mut(w).enumerate().for_each(|(y, row)| {
            cancel.clone().run(|| {
                for (x, out) in row.iter_mut().enumerate() {
                    if filmcraft_media::cancel::cancelled() {
                        break;
                    }
                    let c = [(x * block + block / 2).min(a.w - 1) as f32, (y * block + block / 2).min(a.h - 1) as f32];
                    let (coarse, _) = search(&ac, &bc, [c[0] * 0.5, c[1] * 0.5], [0.0; 2], if maximum { 6 } else { 3 }, 1.0);
                    let (fine, _) = search(a, b, c, [coarse[0] * 2.0, coarse[1] * 2.0], 2, 1.0);
                    let (fine, error) = search(a, b, c, fine, 1, 0.5);
                    let (back, _) = search(b, a, [c[0] + fine[0], c[1] + fine[1]], [-fine[0], -fine[1]], 2, 1.0);
                    let consistency = (fine[0] + back[0]).hypot(fine[1] + back[1]);
                    let confidence = (1.0 - error / 0.11).clamp(0.0, 1.0) * (1.0 - consistency / 2.0).clamp(0.0, 1.0);
                    *out = Motion { dx: fine[0] * a.scale, dy: fine[1] * a.scale, confidence };
                }
            })
        });
        let valid = motions.iter().filter(|m| m.confidence > 0.2).count();
        if valid * 3 < motions.len() || filmcraft_media::cancel::cancelled() {
            return None;
        }
        Some(Self { w, h, block: block as f32 * a.scale, motions })
    }
    fn at(&self, x: usize, y: usize) -> Motion {
        let fx = (x as f32 / self.block - 0.5).clamp(0.0, (self.w - 1) as f32);
        let fy = (y as f32 / self.block - 0.5).clamp(0.0, (self.h - 1) as f32);
        let (x0, y0) = (fx as usize, fy as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (sx, sy) = (fx - x0 as f32, fy - y0 as f32);
        let mut out = Motion::default();
        for (xx, yy, w) in [(x0, y0, (1.0 - sx) * (1.0 - sy)), (x1, y0, sx * (1.0 - sy)), (x0, y1, (1.0 - sx) * sy), (x1, y1, sx * sy)] {
            let m = self.motions[yy * self.w + xx];
            out.dx += m.dx * w;
            out.dy += m.dy * w;
            out.confidence += m.confidence * w;
        }
        out
    }
}
struct TemporalSample<'a> {
    input: &'a Image,
    other: &'a Image,
    motion: &'a MotionField,
    sigma: f32,
    strength: f32,
    settings: Settings,
}
fn accumulate(sum: &mut Image, weights: &mut [f32], sample: TemporalSample<'_>) {
    let TemporalSample { input, other, motion, sigma, strength, settings } = sample;
    let cancel = filmcraft_media::cancel::Context::current();
    sum.px.par_chunks_mut(input.w * 4).zip(weights.par_chunks_mut(input.w)).enumerate().for_each(|(y, (row, ws))| {
        cancel.clone().run(|| {
            if filmcraft_media::cancel::cancelled() {
                return;
            }
            for (x, (p, weight)) in row.chunks_exact_mut(4).zip(ws).enumerate() {
                let orig = input.get(x, y);
                let m = motion.at(x, y);
                let (xx, yy) = (x as f32 + m.dx, y as f32 + m.dy);
                if orig[3] < 1e-6 || xx < 0.0 || yy < 0.0 || xx > (other.w - 1) as f32 || yy > (other.h - 1) as f32 {
                    continue;
                }
                let next = other.sample_bilinear_clamped(xx + 0.5, yy + 0.5);
                if (next[3] - orig[3]).abs() > 0.02 {
                    continue;
                }
                let (a, b) = (components(straight(orig)), components(straight(next)));
                let dy = (a[0] - b[0]).abs();
                let dc = (a[1] - b[1]).abs().max((a[2] - b[2]).abs());
                let threshold = sigma * 4.0 + 0.006;
                let agreement = (1.0 - (dy / threshold).powi(2)).max(0.0) * (1.0 - (dc / (threshold * 2.0)).powi(2)).max(0.0);
                let w = strength * m.confidence * agreement * texture(input, x, y, sigma, settings.detail);
                let c = straight(next);
                for k in 0..3 {
                    p[k] += c[k] * orig[3] * w;
                }
                *weight += w;
            }
        })
    });
}
fn temporal(img: &mut Image, input: &Image, e: &EffectInstance, cx: &FxCtx, settings: Settings, sigma: f32) {
    let Some(env) = cx.env else { return };
    let fps = env.frame_rate();
    if !fps.is_finite() || fps <= 0.0 {
        return;
    }
    let analysis = Plane::from_image(input);
    let mut weights = vec![1.0; input.w * input.h];
    let radius = if settings.quality == 2 { 2 } else { 1 };
    for direction in [-1, 1] {
        for distance in 1..=radius {
            if filmcraft_media::cancel::cancelled() {
                return;
            }
            let dt = (direction * distance) as f64 / fps;
            if env.clip_offset() + dt < -1e-9 || env.clip_offset() + dt >= env.clip_seconds() {
                break;
            }
            let Some(other) = env.frame(dt, e) else { break };
            if (other.w, other.h) != (input.w, input.h) {
                break;
            }
            let other_analysis = Plane::from_image(&other);
            let Some(motion) = MotionField::estimate(&analysis, &other_analysis, settings.quality == 2) else { break };
            accumulate(
                img,
                &mut weights,
                TemporalSample { input, other: &other, motion: &motion, sigma, strength: settings.temporal * 1.8 / distance as f32, settings },
            );
        }
    }
    img.px.par_chunks_exact_mut(4).zip(weights).for_each(|(p, w)| {
        for c in &mut p[..3] {
            *c /= w;
        }
    });
}
fn filtered_pixel(input: &Image, x: usize, y: usize, settings: Settings, sigma: f32) -> [f32; 4] {
    let original = input.get(x, y);
    if original[3] <= 1e-6 || original.iter().any(|v| !v.is_finite()) {
        return original;
    }
    let centre = components(straight(original));
    let range = sigma * (1.5 + settings.spatial * 3.0) + 0.002;
    let radius = ((if settings.quality == 0 { 1.0 } else { 2.0 }) * settings.scale).ceil().clamp(1.0, 2.0) as isize;
    let mut sum = centre;
    let mut weights = [1.0; 3];
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let p = input.get_clamped(x as isize + dx, y as isize + dy);
            if (dx == 0 && dy == 0) || (p[3] - original[3]).abs() > 0.02 {
                continue;
            }
            let c = components(straight(p));
            let spatial = 1.0 / (1.0 + (dx * dx + dy * dy) as f32);
            let edge = 1.0 / (1.0 + ((c[0] - centre[0]) / range).powi(4));
            for k in 0..3 {
                let limit = range * if k == 0 { 1.0 } else { 2.5 };
                let w = spatial * edge / (1.0 + ((c[k] - centre[k]) / limit).powi(2));
                sum[k] += c[k] * w;
                weights[k] += w;
            }
        }
    }
    let protect = texture(input, x, y, sigma, settings.detail);
    for k in 0..3 {
        let blend = (settings.spatial * if k == 0 { 1.5 } else { 2.5 }).min(1.0) * protect;
        sum[k] = centre[k] + (sum[k] / weights[k] - centre[k]) * blend;
    }
    let c = rgb(sum);
    [c[0] * original[3], c[1] * original[3], c[2] * original[3], original[3]]
}
pub(super) fn apply(img: &mut Image, e: &EffectInstance, cx: &FxCtx) {
    let settings = Settings {
        spatial: amount(e, "spatial", cx),
        temporal: amount(e, "temporal", cx),
        detail: amount(e, "detail_protection", cx),
        quality: chv(e, "quality").min(2),
        scale: if cx.px_scale.is_finite() { cx.px_scale.clamp(1.0 / 64.0, 1.0) } else { 1.0 },
    };
    if (settings.spatial == 0.0 && settings.temporal == 0.0) || img.w == 0 || img.h == 0 || filmcraft_media::cancel::cancelled() {
        return;
    }
    let sigma = noise_sigma(img);
    if settings.temporal > 0.0 && cx.env.is_some() {
        let input = img.clone();
        temporal(img, &input, e, cx, settings, sigma);
        if filmcraft_media::cancel::cancelled() {
            *img = input;
            return;
        }
    }
    if settings.spatial > 0.0 {
        let input = img.clone();
        let cancel = filmcraft_media::cancel::Context::current();
        img.px.par_chunks_mut(input.w * 4).enumerate().for_each(|(y, row)| {
            cancel.clone().run(|| {
                if filmcraft_media::cancel::cancelled() {
                    return;
                }
                for (x, p) in row.chunks_exact_mut(4).enumerate() {
                    p.copy_from_slice(&filtered_pixel(&input, x, y, settings, sigma));
                }
            })
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfx::FxEnv;
    use filmcraft_geom::Affine;
    use filmcraft_project::{ParamValue, find_effect};
    use filmcraft_time::Tick;
    use std::sync::Mutex;
    fn context() -> FxCtx<'static> {
        FxCtx {
            t: Tick::ZERO,
            px_scale: 1.0,
            mask_scale: 1.0,
            seconds: 0.0,
            timecode: "",
            clip_name: "",
            project: None,
            env: None,
            frame_rate: 24.0,
            working: filmcraft_color::WorkingSpace::Rec709,
        }
    }
    fn effect() -> EffectInstance {
        find_effect("dw_video_denoise").expect("registered effect").instance()
    }
    fn set(e: &mut EffectInstance, key: &str, value: f64) {
        e.params.get_mut(key).expect("parameter").value = ParamValue::Float(value);
    }
    fn noisy(w: usize, h: usize, seed: u32) -> Image {
        let mut img = Image::filled(w, h, [0.25, 0.25, 0.25, 1.0]);
        for (i, p) in img.px.chunks_exact_mut(4).enumerate() {
            for (k, v) in p[..3].iter_mut().enumerate() {
                let mut n = (i as u32).wrapping_mul(0x9e37_79b9) ^ seed.wrapping_mul(0x85eb_ca6b) ^ (k as u32).wrapping_mul(0xc2b2_ae35);
                n ^= n >> 16;
                n = n.wrapping_mul(0x7feb_352d);
                n ^= n >> 15;
                *v += ((n & 65535) as f32 / 65535.0 - 0.5) * 0.08;
            }
        }
        img
    }
    fn error(img: &Image) -> f32 {
        img.px.chunks_exact(4).flat_map(|p| p[..3].iter()).map(|v| (v - 0.25).powi(2)).sum::<f32>() / (img.w * img.h * 3) as f32
    }
    #[test]
    fn zero_strength_is_bit_exact_and_preserves_hdr_alpha() {
        let mut e = effect();
        set(&mut e, "spatial", 0.0);
        set(&mut e, "temporal", 0.0);
        let original = Image::filled(8, 7, [1.8, -0.1, 0.6, 0.4]);
        let mut output = original.clone();
        apply(&mut output, &e, &context());
        assert_eq!(output, original);
    }
    #[test]
    fn spatial_reduces_noise_without_clipping_or_changing_alpha() {
        let mut e = effect();
        set(&mut e, "temporal", 0.0);
        let input = noisy(64, 48, 1);
        let mut output = input.clone();
        apply(&mut output, &e, &context());
        assert!(error(&output) < error(&input) * 0.9);
        let mut hdr = Image::filled(12, 10, [1.8, -0.05, 0.6, 0.5]);
        apply(&mut hdr, &e, &context());
        for p in hdr.px.chunks_exact(4) {
            assert_eq!(p[3], 0.5);
            assert!(p[0] > 1.0 && p[1] < 0.0);
        }
    }
    #[test]
    fn spatial_preserves_a_sharp_edge() {
        let mut e = effect();
        set(&mut e, "temporal", 0.0);
        let mut input = Image::filled(40, 20, [0.05, 0.05, 0.05, 1.0]);
        for y in 0..20 {
            for x in 20..40 {
                input.px[(y * 40 + x) * 4..(y * 40 + x) * 4 + 3].fill(0.9);
            }
        }
        apply(&mut input, &e, &context());
        assert!(input.get(19, 10)[0] < 0.06 && input.get(20, 10)[0] > 0.89);
    }
    struct Env {
        seen: Mutex<Vec<f64>>,
        offset: f64,
        cut: bool,
        size: (usize, usize),
    }
    impl FxEnv for Env {
        fn frame(&self, dt: f64, _: &EffectInstance) -> Option<Image> {
            self.seen.lock().expect("lock").push(dt);
            Some(if self.cut {
                Image::filled(self.size.0, self.size.1, [0.95, 0.95, 0.95, 1.0])
            } else {
                noisy(self.size.0, self.size.1, ((dt * 24.0) as i32 + 8) as u32)
            })
        }
        fn source_frame(&self, _: f64, _: f32) -> Option<Image> {
            None
        }
        fn track(&self, _: usize) -> Option<Image> {
            None
        }
        fn layer_to_output(&self) -> Affine {
            Affine::default()
        }
        fn clip_seconds(&self) -> f64 {
            2.0
        }
        fn clip_offset(&self) -> f64 {
            self.offset
        }
        fn frame_rate(&self) -> f64 {
            24.0
        }
        fn media_timecode(&self) -> String {
            String::new()
        }
        fn file_name(&self) -> String {
            String::new()
        }
        fn sequence_name(&self) -> String {
            String::new()
        }
        fn sequence_size(&self) -> (u32, u32) {
            (self.size.0 as u32, self.size.1 as u32)
        }
        fn source_size(&self) -> (u32, u32) {
            (self.size.0 as u32, self.size.1 as u32)
        }
        fn clip_key(&self) -> u64 {
            1
        }
    }
    #[test]
    fn maximum_uses_four_neighbours_and_reduces_noise() {
        let env = Env { seen: Mutex::new(Vec::new()), offset: 1.0, cut: false, size: (64, 48) };
        let mut e = effect();
        set(&mut e, "spatial", 0.0);
        let input = noisy(64, 48, 1);
        let mut output = input.clone();
        apply(&mut output, &e, &FxCtx { env: Some(&env), ..context() });
        assert_eq!(env.seen.lock().expect("lock").len(), 4);
        assert!(error(&output) < error(&input) * 0.85);
    }
    #[test]
    fn scene_cuts_and_clip_boundaries_are_rejected() {
        let env = Env { seen: Mutex::new(Vec::new()), offset: 0.0, cut: true, size: (64, 48) };
        let mut e = effect();
        set(&mut e, "spatial", 0.0);
        let input = noisy(64, 48, 1);
        let mut output = input.clone();
        apply(&mut output, &e, &FxCtx { env: Some(&env), ..context() });
        assert_eq!(output, input);
        let seen = env.seen.lock().expect("lock");
        assert_eq!(seen.len(), 1);
        assert!(seen.iter().all(|dt| *dt >= 0.0));
    }
    #[test]
    fn motion_estimation_finds_translation() {
        let a = noisy(96, 64, 1);
        let mut b = Image::filled(96, 64, [0.0, 0.0, 0.0, 1.0]);
        for y in 0..64 {
            for x in 4..96 {
                b.px[(y * 96 + x) * 4..(y * 96 + x) * 4 + 4].copy_from_slice(&a.get(x - 4, y));
            }
        }
        let field = MotionField::estimate(&Plane::from_image(&a), &Plane::from_image(&b), true).expect("motion");
        let m = field.at(48, 32);
        assert!((m.dx - 4.0).abs() < 1.0 && m.dy.abs() < 1.0, "translation {}, {}", m.dx, m.dy);
    }
    #[test]
    fn maximum_processes_five_4k_frames_with_default_settings() {
        let env = Env { seen: Mutex::new(Vec::new()), offset: 1.0, cut: false, size: (3840, 2160) };
        let mut output = noisy(3840, 2160, 1);
        let before = error(&output);
        let started = std::time::Instant::now();
        apply(&mut output, &effect(), &FxCtx { env: Some(&env), ..context() });
        eprintln!("4K Maximum: {:.2}s, noise MSE ratio {:.3}", started.elapsed().as_secs_f64(), error(&output) / before);
        assert!(error(&output) < before * 0.9);
        assert_eq!(env.seen.lock().expect("lock").len(), 4);
        assert!(output.px.chunks_exact(4).all(|p| p[3] == 1.0 && p.iter().all(|v| v.is_finite())));
    }
    #[test]
    fn cancellation_leaves_the_input_unchanged() {
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let input = noisy(64, 48, 2);
        let mut output = input.clone();
        filmcraft_media::cancel::with_cancel(&flag, || apply(&mut output, &effect(), &context()));
        assert_eq!(input, output);
    }
}
