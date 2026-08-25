//! Easing functions.
//!
//! All take `t` in `[0, 1]` and return a value in `[0, 1]` (mostly).
//! Use these as the curve argument to a tween: `lerp(a, b, ease(t))`.

/// Identity.
pub fn linear(t: f32) -> f32 {
    t
}

/// Decelerating start, smooth end.
pub fn quad_in(t: f32) -> f32 {
    t * t
}
/// Smooth start, decelerating end.
pub fn quad_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(2)
}
/// Smooth ends, fast middle.
pub fn quad_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - 2.0 * (1.0 - t).powi(2)
    }
}

/// Cubic accelerating.
pub fn cubic_in(t: f32) -> f32 {
    t * t * t
}
/// Cubic decelerating.
pub fn cubic_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
/// Cubic accelerate then decelerate.
pub fn cubic_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - 4.0 * (1.0 - t).powi(3)
    }
}

/// Bounces past 1 then settles.
pub fn back_out(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Spring-like overshoot.
pub fn elastic_out(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else if t == 1.0 {
        1.0
    } else {
        let c4 = (2.0 * std::f32::consts::PI) / 3.0;
        2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
    }
}

/// Bounce-decay landing on 1.0.
pub fn bounce_out(t: f32) -> f32 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Linear interpolation.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Map `x` from `[a, b]` to `[c, d]` with optional clamping.
pub fn remap(x: f32, a: f32, b: f32, c: f32, d: f32) -> f32 {
    if (b - a).abs() < f32::EPSILON {
        return c;
    }
    c + (d - c) * (x - a) / (b - a)
}

/// Smoothstep (Hermite interpolation).
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn endpoints_are_zero_and_one() {
        for f in [
            linear,
            quad_in,
            quad_out,
            quad_in_out,
            cubic_in,
            cubic_out,
            cubic_in_out,
            smoothstep,
            bounce_out,
        ] {
            assert!(near(f(0.0), 0.0), "f(0) failed");
            assert!(near(f(1.0), 1.0), "f(1) failed");
        }
    }

    #[test]
    fn back_out_overshoots() {
        // Should exceed 1.0 somewhere in (0.5, 1.0).
        let mut max = 0.0_f32;
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            max = max.max(back_out(t));
        }
        assert!(max > 1.0);
    }

    #[test]
    fn quad_in_is_monotonic() {
        let mut prev = -1.0;
        for i in 0..=100 {
            let v = quad_in(i as f32 / 100.0);
            assert!(v >= prev, "non-monotonic at {i}");
            prev = v;
        }
    }

    #[test]
    fn lerp_is_linear() {
        assert!(near(lerp(0.0, 10.0, 0.0), 0.0));
        assert!(near(lerp(0.0, 10.0, 1.0), 10.0));
        assert!(near(lerp(0.0, 10.0, 0.5), 5.0));
    }

    #[test]
    fn remap_handles_degenerate() {
        assert_eq!(remap(5.0, 0.0, 0.0, 1.0, 2.0), 1.0);
        assert!(near(remap(5.0, 0.0, 10.0, 0.0, 100.0), 50.0));
    }

    #[test]
    fn smoothstep_clamps() {
        assert_eq!(smoothstep(-0.5), 0.0);
        assert_eq!(smoothstep(1.5), 1.0);
    }
}
