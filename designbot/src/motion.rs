//! Motion helpers for animation scripts: easing curves, interpolation,
//! and timeline staging. All functions take and return `f64` in the
//! 0.0..=1.0 range (except `lerp`, which maps onto any span).
//!
//! The staging model: a script computes one global `t` per frame
//! (`frame / total_frames`), then carves it into phases with [`seg`] and
//! per-item offsets with [`stagger`], feeding the result through an
//! easing curve:
//!
//! ```ignore
//! let t = i as f64 / frames as f64;          // global timeline 0..1
//! let draw_in = ease_in_out(seg(t, 0.10, 0.35));  // phase from 10%..35%
//! let pop = ease_out_back(stagger(seg(t, 0.35, 0.60), k, n, 0.5));
//! ```

use std::f64::consts::PI;

/// Clamp to the unit interval.
pub fn clamp01(t: f64) -> f64 {
    t.clamp(0.0, 1.0)
}

/// Linear interpolation from `a` to `b` by `t` (unclamped by design).
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Hermite smoothstep — gentle ease-in-out.
pub fn smoothstep(t: f64) -> f64 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}

/// Cubic ease in (slow start).
pub fn ease_in(t: f64) -> f64 {
    let t = clamp01(t);
    t * t * t
}

/// Cubic ease out (slow end).
pub fn ease_out(t: f64) -> f64 {
    let t = 1.0 - clamp01(t);
    1.0 - t * t * t
}

/// Cubic ease in-out — the workhorse for UI-feeling motion.
pub fn ease_in_out(t: f64) -> f64 {
    let t = clamp01(t);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Quartic ease in-out — snappier than cubic.
pub fn ease_in_out_quart(t: f64) -> f64 {
    let t = clamp01(t);
    if t < 0.5 {
        8.0 * t * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
    }
}

/// Exponential ease out — fast arrival, long settle.
pub fn ease_out_expo(t: f64) -> f64 {
    let t = clamp01(t);
    if t >= 1.0 { 1.0 } else { 1.0 - (2.0f64).powf(-10.0 * t) }
}

/// Ease out with a small overshoot past 1.0 before settling (a "pop").
pub fn ease_out_back(t: f64) -> f64 {
    let t = clamp01(t);
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Sine ease in-out — the softest curve.
pub fn ease_in_out_sine(t: f64) -> f64 {
    let t = clamp01(t);
    -( (PI * t).cos() - 1.0) / 2.0
}

/// Normalized progress of `t` through the stage `[t0, t1]`:
/// 0 before it, 1 after it, linear inside. Feed the result to an easing.
pub fn seg(t: f64, t0: f64, t1: f64) -> f64 {
    if t1 <= t0 {
        return if t >= t1 { 1.0 } else { 0.0 };
    }
    clamp01((t - t0) / (t1 - t0))
}

/// Stagger a stage across `n` items: item `i`'s local progress, where each
/// item's window is delayed and `overlap` (0..1) controls how much the
/// windows share (1.0 = all together, 0.0 = strictly one after another).
pub fn stagger(t: f64, i: usize, n: usize, overlap: f64) -> f64 {
    if n <= 1 {
        return clamp01(t);
    }
    let overlap = overlap.clamp(0.0, 1.0);
    let window = overlap + (1.0 - overlap) / n as f64;
    let start = (1.0 - window) * i as f64 / (n - 1) as f64;
    seg(t, start, start + window)
}

/// Ping-pong: 0 -> 1 -> 0 across the unit interval (for there-and-back
/// morphs that loop cleanly).
pub fn ping_pong(t: f64) -> f64 {
    let t = clamp01(t);
    1.0 - (2.0 * t - 1.0).abs()
}
