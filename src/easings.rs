//! Complete 1:1 Rust port of `com.holdmylua.source.scripting.script_wrappers.Easings`.
//!
//! All 31 easing functions preserve the exact mathematical definitions from HMI 5.1.1.

use std::f64::consts::PI;

#[inline]
pub fn ease_in_out_back(x: f64) -> f64 {
    let c1 = 1.70158_f32 as f64;
    let c2 = c1 * (1.525_f32 as f64);
    if x < 0.5 {
        ((2.0 * x).powi(2) * ((c2 + 1.0) * 2.0 * x - c2)) / 2.0
    } else {
        ((2.0 * x - 2.0).powi(2) * ((c2 + 1.0) * (x * 2.0 - 2.0) + c2) + 2.0) / 2.0
    }
}

#[inline]
pub fn ease_in_sine(x: f64) -> f64 {
    1.0 - (x * PI / 2.0).cos()
}

#[inline]
pub fn ease_out_sine(x: f64) -> f64 {
    (x * PI / 2.0).sin()
}

#[inline]
pub fn ease_in_out_sine(x: f64) -> f64 {
    -((PI * x).cos() - 1.0) / 2.0
}

#[inline]
pub fn ease_in_quad(x: f64) -> f64 {
    x * x
}

#[inline]
pub fn ease_out_quad(x: f64) -> f64 {
    1.0 - (1.0 - x) * (1.0 - x)
}

#[inline]
pub fn ease_in_out_quad(x: f64) -> f64 {
    if x < 0.5 {
        2.0 * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(2) / 2.0
    }
}

#[inline]
pub fn ease_in_cubic(x: f64) -> f64 {
    x * x * x
}

#[inline]
pub fn ease_out_cubic(x: f64) -> f64 {
    1.0 - (1.0 - x).powi(3)
}

#[inline]
pub fn ease_in_out_cubic(x: f64) -> f64 {
    if x < 0.5 {
        4.0 * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(3) / 2.0
    }
}

#[inline]
pub fn ease_in_quart(x: f64) -> f64 {
    x * x * x * x
}

#[inline]
pub fn ease_out_quart(x: f64) -> f64 {
    1.0 - (1.0 - x).powi(4)
}

#[inline]
pub fn ease_in_out_quart(x: f64) -> f64 {
    if x < 0.5 {
        8.0 * x * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(4) / 2.0
    }
}

#[inline]
pub fn ease_in_quint(x: f64) -> f64 {
    x * x * x * x * x
}

#[inline]
pub fn ease_out_quint(x: f64) -> f64 {
    1.0 - (1.0 - x).powi(5)
}

#[inline]
pub fn ease_in_out_quint(x: f64) -> f64 {
    if x < 0.5 {
        16.0 * x * x * x * x * x
    } else {
        1.0 - (-2.0 * x + 2.0).powi(5) / 2.0
    }
}

#[inline]
pub fn ease_in_expo(x: f64) -> f64 {
    if x == 0.0 {
        0.0
    } else {
        2.0_f64.powf(10.0 * x - 10.0)
    }
}

#[inline]
pub fn ease_out_expo(x: f64) -> f64 {
    if x == 1.0 {
        1.0
    } else {
        1.0 - 2.0_f64.powf(-10.0 * x)
    }
}

#[inline]
pub fn ease_in_out_expo(x: f64) -> f64 {
    if x == 0.0 {
        0.0
    } else if x == 1.0 {
        1.0
    } else if x < 0.5 {
        2.0_f64.powf(20.0 * x - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f64.powf(-20.0 * x + 10.0)) / 2.0
    }
}

#[inline]
pub fn ease_in_circ(x: f64) -> f64 {
    1.0 - (1.0 - x * x).max(0.0).sqrt()
}

#[inline]
pub fn ease_out_circ(x: f64) -> f64 {
    (1.0 - (x - 1.0).powi(2)).max(0.0).sqrt()
}

#[inline]
pub fn ease_in_out_circ(x: f64) -> f64 {
    if x < 0.5 {
        (1.0 - (1.0 - (2.0 * x).powi(2)).max(0.0).sqrt()) / 2.0
    } else {
        ((1.0 - (-2.0 * x + 2.0).powi(2)).max(0.0).sqrt() + 1.0) / 2.0
    }
}

#[inline]
pub fn ease_in_back(x: f64) -> f64 {
    let c1 = 1.70158_f32 as f64;
    let c3 = c1 + 1.0;
    c3 * x * x * x - c1 * x * x
}

#[inline]
pub fn ease_out_back(x: f64) -> f64 {
    let c1 = 1.70158_f32 as f64;
    let c3 = c1 + 1.0;
    1.0 + c3 * (x - 1.0).powi(3) + c1 * (x - 1.0).powi(2)
}

#[inline]
pub fn ease_in_elastic(x: f64) -> f64 {
    let c4 = (2.0 * PI) / 3.0;
    if x == 0.0 {
        0.0
    } else if x == 1.0 {
        1.0
    } else {
        -2.0_f64.powf(10.0 * x - 10.0) * ((x * 10.0 - 10.75) * c4).sin()
    }
}

#[inline]
pub fn ease_out_elastic(x: f64) -> f64 {
    let c4 = (2.0 * PI) / 3.0;
    if x == 0.0 {
        0.0
    } else if x == 1.0 {
        1.0
    } else {
        2.0_f64.powf(-10.0 * x) * ((x * 10.0 - 0.75) * c4).sin() + 1.0
    }
}

#[inline]
pub fn ease_in_out_elastic(x: f64) -> f64 {
    let c5 = (2.0 * PI) / 4.5;
    if x == 0.0 {
        0.0
    } else if x == 1.0 {
        1.0
    } else if x < 0.5 {
        -(2.0_f64.powf(20.0 * x - 10.0) * ((20.0 * x - 11.125) * c5).sin()) / 2.0
    } else {
        (2.0_f64.powf(-20.0 * x + 10.0) * ((20.0 * x - 11.125) * c5).sin()) / 2.0 + 1.0
    }
}

#[inline]
pub fn ease_out_bounce(x: f64) -> f64 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if x < 1.0 / d1 {
        n1 * x * x
    } else if x < 2.0 / d1 {
        let t = x - 1.5 / d1;
        n1 * t * t + 0.75
    } else if x < 2.5 / d1 {
        let t = x - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = x - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

#[inline]
pub fn ease_in_bounce(x: f64) -> f64 {
    1.0 - ease_out_bounce(1.0 - x)
}

#[inline]
pub fn ease_in_out_bounce(x: f64) -> f64 {
    if x < 0.5 {
        (1.0 - ease_out_bounce(1.0 - 2.0 * x)) / 2.0
    } else {
        (1.0 + ease_out_bounce(2.0 * x - 1.0)) / 2.0
    }
}

#[inline]
pub fn cubic_ease(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}
