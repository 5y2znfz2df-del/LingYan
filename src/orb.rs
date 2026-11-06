//! Siri-style black liquid-glass orb renderer (software, premultiplied ARGB).

pub fn render(pixels: &mut [u32], width: usize, height: usize, t: f32, active: bool) {
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.5;
    let base = (width.min(height) as f32) * 0.355;
    let pulse = if active {
        1.0 + 0.025 * (t * 3.2).sin()
    } else {
        1.0 + 0.010 * (t * 1.8).sin()
    };
    let radius = (base * pulse).max(1.0);
    let inv_r = 1.0 / radius;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let n = (dx * dx + dy * dy).sqrt() * inv_r;
            let mut color = [0.0f32; 3];
            let mut alpha = 0.0f32;

            if n <= 1.0 {
                let nx = dx * inv_r;
                let ny = dy * inv_r;

                let v = ((ny + 1.0) * 0.5).clamp(0.0, 1.0);
                let grad = v.powf(0.85);
                let top = [0.115f32, 0.125, 0.165];
                let bottom = [0.006f32, 0.007, 0.012];
                color = [
                    top[0] + (bottom[0] - top[0]) * grad,
                    top[1] + (bottom[1] - top[1]) * grad,
                    top[2] + (bottom[2] - top[2]) * grad,
                ];

                let lx = nx + 0.62;
                let ly = ny + 0.62;
                let sheen = (-(lx * lx + ly * ly) * 1.15).exp() * 0.10;
                color = blend_add(color, [0.62, 0.72, 0.95], sheen);

                let hx = nx + 0.46;
                let hy = ny + 0.46;
                let highlight = (-(hx * hx) * 5.2 - (hy * hy) * 8.5).exp();
                if highlight > 0.01 {
                    color = blend_add(color, [0.90, 0.95, 1.0], highlight * 0.62);
                }

                let sx = nx - 0.55;
                let sy = ny + 0.10;
                let right = (-(sx * sx) * 7.0 - (sy * sy) * 3.0).exp() * 0.08;
                color = blend_add(color, [0.55, 0.62, 0.85], right);

                let rim_dark = smoothstep(0.70, 1.0, n) * 0.55;
                color = [
                    color[0] * (1.0 - rim_dark),
                    color[1] * (1.0 - rim_dark),
                    color[2] * (1.0 - rim_dark * 0.85),
                ];

                let bezel = smoothstep(0.84, 0.995, n);
                if bezel > 0.001 {
                    let hue = (t * 38.0 + nx * 75.0 - ny * 55.0).rem_euclid(360.0);
                    let (hr, hg, hb) = hue_rgb(hue);
                    let lower = smoothstep(-0.75, 0.35, ny);
                    let strength = bezel * (0.16 + 0.28 * lower);
                    color = blend_add(color, [hr, hg, hb], strength);
                }

                let edge = smoothstep(0.955, 0.985, n) * (1.0 - smoothstep(0.985, 1.0, n));
                if edge > 0.001 {
                    color = blend_add(color, [0.78, 0.86, 1.0], edge * 0.50);
                }

                let caustic_y = ny - 0.62;
                let caustic = (-(caustic_y * caustic_y) * 18.0).exp()
                    * (-(nx * nx) * 2.2).exp()
                    * smoothstep(0.35, 0.75, n)
                    * 0.10;
                if caustic > 0.001 {
                    color = blend_add(color, [0.50, 0.60, 0.90], caustic);
                }

                alpha = 0.985;
                if n > 0.965 {
                    alpha *= ((1.0 - n) / 0.035).clamp(0.0, 1.0);
                }
            }

            if (1.0..1.12).contains(&n) {
                let strength = (1.0 - (n - 1.0) / 0.12).max(0.0).powi(3);
                color = blend_add(color, [0.35, 0.42, 0.65], strength * 0.10);
                alpha = (alpha + strength * 0.05).min(1.0);
            }

            let a = (alpha * 255.0) as u32;
            let r = (color[0].clamp(0.0, 1.0) * alpha * 255.0) as u32;
            let g = (color[1].clamp(0.0, 1.0) * alpha * 255.0) as u32;
            let b = (color[2].clamp(0.0, 1.0) * alpha * 255.0) as u32;
            pixels[y * width + x] = (a << 24) | (r << 16) | (g << 8) | b;
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn blend_add(base: [f32; 3], add: [f32; 3], weight: f32) -> [f32; 3] {
    [
        base[0] + add[0] * weight,
        base[1] + add[1] * weight,
        base[2] + add[2] * weight,
    ]
}

fn hue_rgb(hue: f32) -> (f32, f32, f32) {
    let h = hue.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    match h as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    }
}
