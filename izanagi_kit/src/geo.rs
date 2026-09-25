//! Geodesy primitives in `Fixed` degrees — the geographic sibling of
//! the [`crate::snoise`]/[`crate::worley`] field family.
//!
//! Distance is returned in **kilometers** (the only unit whose magnitudes
//! fit Q16.16: Earth's radius `6371.0088 km` is raw `~4.18e8` — meters
//! would overflow `i32`). Two models:
//!
//! - [`haversine`]: spherical, exact for a sphere of radius
//!   [`R_KM`].
//! - [`lambert`]: Lambert–Andoyer flattening correction (~10 m accuracy
//!   up to 12,000 km per the Survey Review literature) — the closed-form
//!   answer where Vincenty's iteration would be used, and the safer
//!   choice in `Fixed` since it never iterates.
//!
//! Plus the usual closed forms: initial [`bearing`], [`dest`] point
//! (great-circle), [`midpoint`], and [`norm_lon`] wrapping to
//! `(-180°, 180°]`.
//!
//! ```
//! use izanagi_kit::geo::{haversine, P};
//! use izanagi_kit::fixed::Fixed;
//!
//! // Tokyo → Osaka ≈ 403 km on a sphere.
//! let tokyo = P::deg(Fixed::from_ratio(3_568_120, 100_000), Fixed::from_ratio(13_976_710, 100_000));
//! let osaka = P::deg(Fixed::from_ratio(3_469_370, 100_000), Fixed::from_ratio(13_550_230, 100_000));
//! let d = haversine(tokyo, osaka);
//! assert!(d > Fixed::from_int(350) && d < Fixed::from_int(450));
//! ```

use crate::fixed::Fixed;

/// Earth mean radius in kilometers (IUGG R1 = 6,371.0088 km).
pub const R_KM: Fixed = Fixed::from_raw(417_530_433);

/// Degrees → radians.
fn rad(deg: Fixed) -> Fixed {
    deg.mul(Fixed::PI).div(Fixed::from_int(180))
}

/// Radians → degrees.
fn deg(r: Fixed) -> Fixed {
    r.mul(Fixed::from_int(180)).div(Fixed::PI)
}

fn asin(x: Fixed) -> Fixed {
    if x >= Fixed::ONE {
        return Fixed::HALF_PI;
    }
    if x <= Fixed::ZERO - Fixed::ONE {
        return Fixed::ZERO - Fixed::HALF_PI;
    }
    let one_minus = Fixed::ONE - x.mul(x);
    let den = if one_minus < Fixed::ZERO {
        Fixed::ZERO
    } else {
        one_minus
    }
    .sqrt();
    Fixed::atan2(x, den)
}

fn acos(x: Fixed) -> Fixed {
    if x >= Fixed::ONE {
        return Fixed::ZERO;
    }
    if x <= Fixed::ZERO - Fixed::ONE {
        return Fixed::PI;
    }
    let one_minus = Fixed::ONE - x.mul(x);
    let num = if one_minus < Fixed::ZERO {
        Fixed::ZERO
    } else {
        one_minus
    }
    .sqrt();
    Fixed::atan2(num, x)
}

/// A geographic point in degrees (`lat` ∈ `[-90, 90]`, `lon` wrapped
/// to `(-180, 180]`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct P {
    /// Latitude in degrees.
    pub lat: Fixed,
    /// Longitude in degrees.
    pub lon: Fixed,
}

impl P {
    /// Build with validation-free clamping: `lat` clamps into ±90°,
    /// `lon` wraps via [`norm_lon`].
    pub fn deg(lat: Fixed, lon: Fixed) -> P {
        let n90 = Fixed::from_int(90);
        let lat = if lat > n90 {
            n90
        } else if lat < Fixed::ZERO - n90 {
            Fixed::ZERO - n90
        } else {
            lat
        };
        P {
            lat,
            lon: norm_lon(lon),
        }
    }
}

/// Wrap longitude into `(-180°, 180°]` deterministically (adds of
/// `360°` until in range — no `%` on `Fixed`).
pub fn norm_lon(lon: Fixed) -> Fixed {
    let e = Fixed::from_int(360);
    let h = Fixed::from_int(180);
    let mut x = lon;
    while x <= Fixed::ZERO - h {
        x = x + e;
    }
    while x > h {
        x = x - e;
    }
    x
}

/// Great-circle distance in km on a sphere of radius [`R_KM`].
///
/// `h = sin²(Δφ/2) + cos φ₁ cos φ₂ sin²(Δλ/2)`, `d = 2R·asin(√h)` —
/// numerically stable for all separations (unlike the law of cosines
/// for short arcs).
pub fn haversine(a: P, b: P) -> Fixed {
    let p1 = rad(a.lat);
    let p2 = rad(b.lat);
    let dl = rad(norm_lon(b.lon - a.lon));
    let dp = p2 - p1;
    let half_dp = dp.div(Fixed::from_int(2));
    let half_dl = dl.div(Fixed::from_int(2));
    let sdp = half_dp.sin_cos().0;
    let sdl = half_dl.sin_cos().0;
    let cp1 = p1.sin_cos().1;
    let cp2 = p2.sin_cos().1;
    let h = sdp.mul(sdp) + cp1.mul(cp2).mul(sdl.mul(sdl));
    let h = if h > Fixed::ONE { Fixed::ONE } else { h };
    let c = Fixed::from_int(2).mul(asin(h.sqrt()));
    R_KM.mul(c)
}

/// Lambert–Andoyer ellipsoidal distance in km on WGS84
/// (`a = 6378.137 km`, `f = 1/298.257223563`) — closed-form flattening
/// correction to the spherical central angle over *reduced* latitudes.
/// Accuracy ~10 m at global range; far better than the sphere for long
/// lines while staying iteration-free.
pub fn lambert(a: P, b: P) -> Fixed {
    // Reduced latitudes β = atan((1−f) tan φ).
    let f = Fixed::from_ratio(1000, 298_257); // 1/298.257223563
    let beta = |p: P| {
        let t = rad(p.lat);
        let (s, c) = t.sin_cos();
        if c.raw() == 0 {
            // Poles stay at their reduced pole.
            if s.raw() >= 0 {
                Fixed::HALF_PI
            } else {
                Fixed::ZERO - Fixed::HALF_PI
            }
        } else {
            Fixed::atan2((Fixed::ONE - f).mul(s), c)
        }
    };
    let b1 = beta(a);
    let b2 = beta(b);
    let dl = rad(norm_lon(b.lon - a.lon));
    let (sb1, cb1) = b1.sin_cos();
    let (sb2, cb2) = b2.sin_cos();
    let cdl = dl.sin_cos().1;
    let cos_sig = sb1.mul(sb2) + cb1.mul(cb2).mul(cdl);
    let sig = acos(if cos_sig > Fixed::ONE {
        Fixed::ONE
    } else if cos_sig < Fixed::ZERO - Fixed::ONE {
        Fixed::ZERO - Fixed::ONE
    } else {
        cos_sig
    });
    if sig.raw() == 0 {
        return Fixed::ZERO;
    }
    let pp = (b1 + b2).div(Fixed::from_int(2));
    let q = (b2 - b1).div(Fixed::from_int(2));
    let (sp, cp) = pp.sin_cos();
    let (sq, cq) = q.sin_cos();
    let (shs, chs) = sig.div(Fixed::from_int(2)).sin_cos();
    // Guard the two removable poles of X and Y.
    let x = if chs.raw() == 0 {
        Fixed::ZERO
    } else {
        (sig - sig.sin_cos().0)
            .mul(sp.mul(sp).mul(cq.mul(cq)))
            .div(chs.mul(chs))
    };
    let y = if shs.raw() == 0 {
        Fixed::ZERO
    } else {
        (sig + sig.sin_cos().0)
            .mul(cp.mul(cp).mul(sq.mul(sq)))
            .div(shs.mul(shs))
    };
    let big_a = Fixed::from_int(6378) + Fixed::from_ratio(137, 1000); // a = 6378.137 km
    big_a.mul(sig - f.div(Fixed::from_int(2)).mul(x + y))
}

/// Initial (forward) bearing in degrees `[0, 360)`.
///
/// `θ = atan2(sin Δλ·cos φ₂, cos φ₁·sin φ₂ − sin φ₁·cos φ₂·cos Δλ)`.
pub fn bearing(a: P, b: P) -> Fixed {
    let p1 = rad(a.lat);
    let p2 = rad(b.lat);
    let dl = rad(norm_lon(b.lon - a.lon));
    let (sp1, cp1) = p1.sin_cos();
    let (sp2, cp2) = p2.sin_cos();
    let (sdl, cdl) = dl.sin_cos();
    let y = sdl.mul(cp2);
    let x = cp1.mul(sp2) - sp1.mul(cp2).mul(cdl);
    if x.raw() == 0 && y.raw() == 0 {
        return Fixed::ZERO;
    }
    let mut d = deg(Fixed::atan2(y, x));
    if d < Fixed::ZERO {
        d = d + Fixed::from_int(360);
    }
    d
}

/// Destination point `d` km from `a` on initial bearing `brg` degrees.
pub fn dest(a: P, brg: Fixed, dist_km: Fixed) -> P {
    let p1 = rad(a.lat);
    let t = rad(brg);
    let dr = dist_km.div(R_KM);
    let (sp1, cp1) = p1.sin_cos();
    let (st, ct) = t.sin_cos();
    let (sdr, cdr) = dr.sin_cos();
    let sp2 = sp1.mul(cdr) + cp1.mul(sdr).mul(ct);
    let p2 = asin(if sp2 > Fixed::ONE {
        Fixed::ONE
    } else if sp2 < Fixed::ZERO - Fixed::ONE {
        Fixed::ZERO - Fixed::ONE
    } else {
        sp2
    });
    let lon2 = rad(a.lon) + Fixed::atan2(st.mul(sdr).mul(cp1), cdr - sp1.mul(sp2));
    P {
        lat: deg(p2),
        lon: norm_lon(deg(lon2)),
    }
}

/// Great-circle midpoint (geographic mean direction).
pub fn midpoint(a: P, b: P) -> P {
    let p1 = rad(a.lat);
    let p2 = rad(b.lat);
    let dl = rad(norm_lon(b.lon - a.lon));
    let (sp1, cp1) = p1.sin_cos();
    let (sp2, cp2) = p2.sin_cos();
    let (sdl, cdl) = dl.sin_cos();
    let bx = cp2.mul(cdl);
    let by = cp2.mul(sdl);
    let lat = Fixed::atan2(sp1 + sp2, ((cp1 + bx).mul(cp1 + bx) + by.mul(by)).sqrt());
    let lon = rad(a.lon) + Fixed::atan2(by, cp1 + bx);
    P {
        lat: deg(lat),
        lon: norm_lon(deg(lon)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(lat_num: i32, lon_num: i32) -> P {
        P::deg(
            Fixed::from_ratio(lat_num, 10_000),
            Fixed::from_ratio(lon_num, 10_000),
        )
    }

    fn km(f: Fixed) -> f64 {
        f.raw() as f64 / 65536.0
    }

    #[test]
    fn haversine_known_distances() {
        // LHR → JFK ≈ 5,570 km (spherical).
        let d = km(haversine(p(515_074, -1_278), p(407_128, -740_060)));
        assert!((d - 5570.0).abs() < 40.0, "hav = {d}");
        // Equator quarter: πR/2 ≈ 10,018.75 km.
        let d = km(haversine(p(0, 0), p(0, 900_000)));
        // Q16.16 trig is ~0.1 % — the equator arc tolerates ±15 km.
        assert!((d - 10_018.75).abs() < 15.0, "eq = {d}");
        // Zero distance.
        assert_eq!(haversine(p(100_000, 200_000), p(100_000, 200_000)).raw(), 0);
    }

    #[test]
    fn lambert_flattens() {
        // Lambert–Andoyer LHR→JFK ≈ 5,585 km vs spherical 5,570.
        let d = km(lambert(p(515_074, -1_278), p(407_128, -740_060)));
        assert!((d - 5585.0).abs() < 50.0, "lam = {d}");
        // Equator: 90° arc ≈ 10,018.75 km.
        let d = km(lambert(p(0, 0), p(0, 900_000)));
        assert!((d - 10_018.75).abs() < 20.0, "eq = {d}");
        assert_eq!(lambert(p(0, 0), p(0, 0)).raw(), 0);
    }

    #[test]
    fn bearing_and_dest() {
        // LHR→JFK initial bearing ≈ 288.3°.
        let b = bearing(p(515_074, -1_278), p(407_128, -740_060));
        let bd = km(b);
        assert!((bd - 288.33).abs() < 2.0, "brg = {bd}");
        // Due east on the equator: bearing 90, dest flips nothing.
        let b = bearing(p(0, 0), p(0, 100_000));
        assert!((km(b) - 90.0).abs() < 0.5);
        let d = dest(p(0, 0), Fixed::from_int(90), Fixed::from_int(1000));
        assert!(km(d.lat).abs() < 0.5);
        assert!((km(d.lon) - 8.99).abs() < 0.3);
    }

    #[test]
    fn midpoint_and_wrap() {
        let m = midpoint(p(0, 0), p(0, 900_000));
        assert!((km(m.lon) - 45.0).abs() < 0.5);
        assert!(km(m.lat).abs() < 0.5);
        // Longitude wrapping.
        assert_eq!(norm_lon(Fixed::from_int(190)), Fixed::from_int(-170));
        assert_eq!(norm_lon(Fixed::from_int(-190)), Fixed::from_int(170));
        assert_eq!(norm_lon(Fixed::from_int(180)), Fixed::from_int(180));
    }

    #[test]
    fn clamps_and_degrades() {
        // Lat clamp.
        let x = P::deg(Fixed::from_int(95), Fixed::ZERO);
        assert_eq!(x.lat, Fixed::from_int(90));
        // Antipodal-ish distance stays finite.
        let d = haversine(p(0, 0), p(1_000, -1_795_000));
        assert!(d.raw() > 0);
    }
}
