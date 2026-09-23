//! Ed25519 — RFC 8032 EdDSA signatures over the Edwards curve
//! `−x² + y² = 1 + dx²y²` in `GF(2²⁵⁵−19)`, integer-only by
//! construction. Field arithmetic is the donna64-style radix-51
//! five-limb form: `u128` column sums stay under `2¹¹⁰`, so the
//! schoolbook product can never overflow. Scalars live in a
//! separate `[u64; 4]` type and reduce mod
//! `L = 2²⁵² + 27742317777372353535851937790883648493` by a
//! bit-fold (`acc = 2·acc + bit`, one conditional subtract) —
//! every operation is total and deterministic.
//!
//! ```
//! use izanagi_kit::ed25519::{keypair, sign, verify};
//! let (pk, sk) = keypair(b"01234567890123456789012345678901");
//! let sig = sign(&sk, b"hello");
//! assert!(verify(&pk, b"hello", &sig));
//! ```
//!
//! References: Bernstein et al. (2012) "High-speed high-security
//! signatures"; RFC 8032 §3/§5 (clamping, nonce derivation,
//! check `[S]B = R + [k]A`); Hisil–Wong–Carter–Dawson extended
//! twisted-Edwards addition (complete — no exceptional cases).

use crate::sha512::sha512;

/// Field element: 5 limbs of 51 bits each (little-endian).
type Fe = [u64; 5];
/// Scalar: 4 limbs of 64 bits.
type Sc = [u64; 4];

const MASK51: u64 = (1 << 51) - 1;

/// `p = 2²⁵⁵ − 19` in radix-51 limbs.
const P: Fe = [
    0x7ffffffffffed,
    0x7ffffffffffff,
    0x7ffffffffffff,
    0x7ffffffffffff,
    0x7ffffffffffff,
];

/// `d = −121665·121666⁻¹`.
const D: Fe = [
    0x34dca135978a3,
    0x1a8283b156ebd,
    0x5e7a26001c029,
    0x739c663a03cbb,
    0x52036cee2b6ff,
];

/// `2·d`.
const D2: Fe = [
    0x69b9426b2f159,
    0x35050762add7a,
    0x3cf44c0038052,
    0x6738cc7407977,
    0x2406d9dc56dff,
];

/// `i = 2^{(p−1)/4}`, the fixed `√−1`.
const SQRT_M1: Fe = [
    0x61b274a0ea0b0,
    0xd5a5fc8f189d,
    0x7ef5e9cbd0c60,
    0x78595a6804c9e,
    0x2b8324804fc1d,
];

const BX: Fe = [
    0x62d608f25d51a,
    0x412a4b4f6592a,
    0x75b7171a4b31d,
    0x1ff60527118fe,
    0x216936d3cd6e5,
];
const BY: Fe = [
    0x6666666666658,
    0x4cccccccccccc,
    0x1999999999999,
    0x3333333333333,
    0x6666666666666,
];
const BT: Fe = [
    0x68ab3a5b7dda3,
    0xeea2a5eadbb,
    0x2af8df483c27e,
    0x332b375274732,
    0x67875f0fd78b7,
];

/// `L = 2²⁵² + 27742317777372353535851937790883648493`, 4×64.
const L: Sc = [
    0x5812631a5cf5d3ed,
    0x14def9dea2f79cd6,
    0,
    0x1000000000000000,
];

/// Carry-propagate a 51-bit-limb element twice — enough for any
/// value produced by `fadd`/`fsub`/`fmul`'s folds.
fn norm(mut h: Fe) -> Fe {
    for _ in 0..2 {
        for i in 0..4 {
            let c = h[i] >> 51;
            h[i] &= MASK51;
            h[i + 1] += c;
        }
        let c = h[4] >> 51;
        h[4] &= MASK51;
        h[0] += c * 19;
    }
    h
}

/// Canonical form `< p`: after `norm` the value is `< 2²⁵⁵ < 2p`,
/// so one conditional subtract finishes the reduction. All
/// comparisons and serializations must go through `canon`, not
/// `norm` — a normalized value may still sit in `[p, 2²⁵⁵)`.
fn canon(f: &Fe) -> Fe {
    let h = norm(*f);
    if fge(&h, &P) {
        // limbwise subtract — h ≥ p here so no bias needed
        let mut r = [0u64; 5];
        let mut borrow = 0i64;
        for i in 0..5 {
            let d = h[i] as i64 - P[i] as i64 + borrow;
            r[i] = d as u64 & MASK51;
            borrow = d >> 51;
        }
        r
    } else {
        h
    }
}

fn fadd(a: &Fe, b: &Fe) -> Fe {
    let mut h = [0u64; 5];
    for i in 0..5 {
        h[i] = a[i] + b[i];
    }
    norm(h)
}

fn fsub(a: &Fe, b: &Fe) -> Fe {
    // bias by 2p so no limb goes negative
    let mut h = [0u64; 5];
    for i in 0..5 {
        h[i] = a[i] + 2 * P[i] - b[i];
    }
    norm(h)
}

fn fmul(a: &Fe, b: &Fe) -> Fe {
    let mut t = [0u128; 9];
    for i in 0..5 {
        for j in 0..5 {
            t[i + j] += a[i] as u128 * b[j] as u128;
        }
    }
    // 2²⁵⁵ ≡ 19 — fold the four high columns down
    for i in 0..4 {
        t[i] += t[i + 5] * 19;
    }
    let mut h = [0u64; 5];
    let mut c = 0u128;
    for i in 0..5 {
        let v = t[i] + c;
        h[i] = (v as u64) & MASK51;
        c = v >> 51;
    }
    // The final carry can reach 2^59 — fold ×19 back through the
    // 128-bit lane; `h[0] += c*19` in u64 would wrap.
    let v = h[0] as u128 + c * 19;
    h[0] = (v as u64) & MASK51;
    let mut c2 = v >> 51;
    for h_i in h.iter_mut().skip(1) {
        let v = *h_i as u128 + c2;
        *h_i = (v as u64) & MASK51;
        c2 = v >> 51;
    }
    h[0] += (c2 as u64) * 19;
    norm(h)
}

fn fsqr(a: &Fe) -> Fe {
    fmul(a, a)
}

/// `a^e` for a 256-bit exponent, MSB-first square-multiply.
fn fpow(a: &Fe, e: &Sc) -> Fe {
    let mut acc: Fe = [1, 0, 0, 0, 0];
    for i in (0..256).rev() {
        acc = fsqr(&acc);
        if (e[i / 64] >> (i % 64)) & 1 == 1 {
            acc = fmul(&acc, a);
        }
    }
    acc
}

/// `a⁻¹ = a^{p−2}` (Fermat).
fn finv(a: &Fe) -> Fe {
    const PM2: Sc = [0xffffffffffffffeb, !0, !0, 0x7fffffffffffffff];
    fpow(a, &PM2)
}

/// `√v` when `v` is a quadratic residue, `None` otherwise.
/// `p ≡ 5 (mod 8)`: `x = v^{(p+3)/8}`; if `x² = −v` then `x·i`.
fn fsqrt(v: &Fe) -> Option<Fe> {
    const E: Sc = [
        0xfffffffffffffffe,
        0xffffffffffffffff,
        0xffffffffffffffff,
        0x0fffffffffffffff,
    ];
    let mut x = fpow(v, &E);
    if canon(&fsqr(&x)) == canon(v) {
        return Some(x);
    }
    x = fmul(&x, &SQRT_M1);
    if canon(&fsqr(&x)) == canon(v) {
        Some(x)
    } else {
        None
    }
}

/// Extended twisted-Edwards point `(X:Y:Z:T)` with `xy = zt`.
#[derive(Clone, Copy, Debug)]
struct Pt {
    x: Fe,
    y: Fe,
    z: Fe,
    t: Fe,
}

const ID: Pt = Pt {
    x: [0, 0, 0, 0, 0],
    y: [1, 0, 0, 0, 0],
    z: [1, 0, 0, 0, 0],
    t: [0, 0, 0, 0, 0],
};

const BASE: Pt = Pt {
    x: BX,
    y: BY,
    z: [1, 0, 0, 0, 0],
    t: BT,
};

/// Hisil et al. complete addition for `a = −1`.
fn padd(p: &Pt, q: &Pt) -> Pt {
    let a = fmul(&fsub(&p.y, &p.x), &fsub(&q.y, &q.x));
    let b = fmul(&fadd(&p.y, &p.x), &fadd(&q.y, &q.x));
    let c = fmul(&fmul(&p.t, &q.t), &D2);
    // EFD add-2008-hwcd-3: D = 2·Z1·Z2
    let zz = fmul(&p.z, &q.z);
    let dd = fadd(&zz, &zz);
    let e = fsub(&b, &a);
    let f = fsub(&dd, &c);
    let g = fadd(&dd, &c);
    let h = fadd(&b, &a);
    Pt {
        x: fmul(&e, &f),
        y: fmul(&g, &h),
        z: fmul(&f, &g),
        t: fmul(&e, &h),
    }
}

fn pdbl(p: &Pt) -> Pt {
    let a = fsqr(&p.x);
    let b = fsqr(&p.y);
    let zz = fsqr(&p.z);
    let c = fadd(&zz, &zz);
    let d = fsub(&[0, 0, 0, 0, 0], &a);
    let e = fsub(&fsub(&fsqr(&fadd(&p.x, &p.y)), &a), &b);
    let g = fadd(&d, &b);
    let f = fsub(&g, &c);
    let h = fsub(&d, &b);
    Pt {
        x: fmul(&e, &f),
        y: fmul(&g, &h),
        z: fmul(&f, &g),
        t: fmul(&e, &h),
    }
}

/// `[s]·P` — MSB-first double-and-add over the 256-bit scalar.
fn pmul(p: &Pt, s: &Sc) -> Pt {
    let mut acc = ID;
    for i in (0..256).rev() {
        acc = pdbl(&acc);
        if (s[i / 64] >> (i % 64)) & 1 == 1 {
            acc = padd(&acc, p);
        }
    }
    acc
}

/// Serialize a normalized `Fe` to 32 little-endian bytes.
fn fe_to_bytes(f: &Fe) -> [u8; 32] {
    let h = canon(f);
    let mut out = [0u8; 32];
    for i in 0..255 {
        let bit = (h[i / 51] >> (i % 51)) & 1;
        out[i / 8] |= (bit as u8) << (i % 8);
    }
    out
}

/// Parse 32 little-endian bytes into a `Fe` (may be ≥ p — callers
/// compare against `P` themselves for canonicality).
fn fe_from_bytes(b: &[u8; 32]) -> Fe {
    let mut h = [0u64; 5];
    for i in 0..255 {
        let bit = (b[i / 8] >> (i % 8)) & 1;
        h[i / 51] |= (bit as u64) << (i % 51);
    }
    h
}

fn fge(a: &Fe, b: &Fe) -> bool {
    let a = norm(*a);
    let b = norm(*b);
    for i in (0..5).rev() {
        if a[i] != b[i] {
            return a[i] > b[i];
        }
    }
    true
}

/// `y` affine with `x`'s low bit in bit 255 — RFC 8032 §3.1.
fn compress(p: &Pt) -> [u8; 32] {
    let iz = finv(&p.z);
    let y = fmul(&p.y, &iz);
    let x = fmul(&p.x, &iz);
    let mut out = fe_to_bytes(&y);
    out[31] |= ((x[0] & 1) as u8) << 7;
    out
}

/// Point decompression; rejects `y ≥ p` and non-residue `x²`.
fn decompress(bytes: &[u8; 32]) -> Option<Pt> {
    let mut yb = *bytes;
    let x_sign = (yb[31] >> 7) & 1;
    yb[31] &= 0x7f;
    let y = fe_from_bytes(&yb);
    if fge(&y, &P) {
        return None;
    }
    let yy = fsqr(&y);
    let u = fsub(&yy, &[1, 0, 0, 0, 0]);
    let v = fadd(&fmul(&D, &yy), &[1, 0, 0, 0, 0]);
    let xx = fmul(&u, &finv(&v));
    let mut x = fsqrt(&xx)?;
    if (x[0] & 1) as u8 != x_sign {
        x = fsub(&[0, 0, 0, 0, 0], &x);
    }
    // x = 0 with sign bit set is non-canonical
    if canon(&x) == [0, 0, 0, 0, 0] && x_sign == 1 {
        return None;
    }
    Some(Pt {
        x,
        y,
        z: [1, 0, 0, 0, 0],
        t: fmul(&x, &y),
    })
}

fn sc_from_bytes(b: &[u8; 32]) -> Sc {
    let mut r = [0u64; 4];
    for i in 0..4 {
        for j in 0..8 {
            r[i] |= (b[i * 8 + j] as u64) << (j * 8);
        }
    }
    r
}

fn sc_ge(a: &Sc, b: &Sc) -> bool {
    for i in (0..4).rev() {
        if a[i] != b[i] {
            return a[i] > b[i];
        }
    }
    true
}

fn sc_sub(a: &Sc, b: &Sc) -> Sc {
    let mut r = [0u64; 4];
    let mut borrow = 0u64;
    for i in 0..4 {
        let (d1, b1) = a[i].overflowing_sub(b[i]);
        let (d2, b2) = d1.overflowing_sub(borrow);
        r[i] = d2;
        borrow = (b1 || b2) as u64;
    }
    r
}

/// Reduce a little-endian `u64`-limb integer mod `L` by bit-fold.
/// `acc = 2·acc + bit` — the injected bit rides the shift chain's
/// incoming carry so a `u64::MAX` low limb can't overflow; `acc`
/// stays `< L`, so one conditional subtract always suffices.
fn mod_l_bits(bits: &[u64]) -> Sc {
    let mut acc: Sc = [0, 0, 0, 0];
    let n = bits.len() * 64;
    for i in (0..n).rev() {
        let mut c = (bits[i / 64] >> (i % 64)) & 1;
        for acc_j in acc.iter_mut() {
            let nv = (*acc_j << 1) | c;
            c = *acc_j >> 63;
            *acc_j = nv;
        }
        if sc_ge(&acc, &L) {
            acc = sc_sub(&acc, &L);
        }
    }
    acc
}

fn add_mod_l(a: &Sc, b: &Sc) -> Sc {
    // a, b < L < 2²⁵³ → sum < 2²⁵⁴, no wrap
    let mut r = [0u64; 4];
    let mut c = 0u64;
    for i in 0..4 {
        let t = a[i] as u128 + b[i] as u128 + c as u128;
        r[i] = t as u64;
        c = (t >> 64) as u64;
    }
    if sc_ge(&r, &L) {
        r = sc_sub(&r, &L);
    }
    r
}

/// `a·b mod L` — double-and-add over `b`'s bits.
fn mul_mod_l(a: &Sc, b: &Sc) -> Sc {
    let mut res: Sc = [0, 0, 0, 0];
    let mut aa = *a;
    for i in 0..256 {
        if (b[i / 64] >> (i % 64)) & 1 == 1 {
            res = add_mod_l(&res, &aa);
        }
        aa = add_mod_l(&aa, &aa);
    }
    res
}

fn hash_to_sc(bytes: &[u8]) -> Sc {
    let d = sha512(bytes);
    let limbs: [u64; 8] = core::array::from_fn(|i| {
        let mut v = 0u64;
        for j in 0..8 {
            v |= (d[i * 8 + j] as u64) << (j * 8);
        }
        v
    });
    mod_l_bits(&limbs)
}

/// Derive `(public_key, secret_key)` from a 32-byte seed —
/// `sk = seed ‖ public_key` per RFC 8032 §5.1.5.
pub fn keypair(seed: &[u8; 32]) -> ([u8; 32], [u8; 64]) {
    let h = sha512(seed);
    let mut a = [0u8; 32];
    a.copy_from_slice(&h[..32]);
    a[0] &= 248;
    a[31] &= 127;
    a[31] |= 64;
    let s = sc_from_bytes(&a);
    let pk = compress(&pmul(&BASE, &s));
    let mut sk = [0u8; 64];
    sk[..32].copy_from_slice(seed);
    sk[32..].copy_from_slice(&pk);
    (pk, sk)
}

/// Sign `msg` with the 64-byte `keypair` secret (seed ‖ pubkey).
pub fn sign(sk: &[u8; 64], msg: &[u8]) -> [u8; 64] {
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&sk[..32]);
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&sk[32..]);
    let h = sha512(&seed);
    let mut a_b = [0u8; 32];
    a_b.copy_from_slice(&h[..32]);
    a_b[0] &= 248;
    a_b[31] &= 127;
    a_b[31] |= 64;
    let a_sc = sc_from_bytes(&a_b);
    // r = H(prefix ‖ msg) mod L
    let mut buf = Vec::with_capacity(32 + msg.len());
    buf.extend_from_slice(&h[32..]);
    buf.extend_from_slice(msg);
    let r = hash_to_sc(&buf);
    let rb = compress(&pmul(&BASE, &r));
    // k = H(R ‖ A ‖ msg) mod L
    let mut buf = Vec::with_capacity(64 + msg.len());
    buf.extend_from_slice(&rb);
    buf.extend_from_slice(&pk);
    buf.extend_from_slice(msg);
    let k = hash_to_sc(&buf);
    // s = (r + k·a) mod L
    let a_mod = mod_l_bits(&a_sc);
    let s = add_mod_l(&r, &mul_mod_l(&k, &a_mod));
    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&rb);
    for i in 0..4 {
        for j in 0..8 {
            sig[32 + i * 8 + j] = (s[i] >> (j * 8)) as u8;
        }
    }
    sig
}

/// Verify `sig` on `msg` under `pk`: `[S]B == R + [k]A`.
/// Rejects `S ≥ L` and undecodable `R`/`A` encodings.
pub fn verify(pk: &[u8; 32], msg: &[u8], sig: &[u8; 64]) -> bool {
    let mut rb = [0u8; 32];
    rb.copy_from_slice(&sig[..32]);
    let mut sb = [0u8; 32];
    sb.copy_from_slice(&sig[32..]);
    let s = sc_from_bytes(&sb);
    if sc_ge(&s, &L) {
        return false;
    }
    let r = match decompress(&rb) {
        Some(p) => p,
        None => return false,
    };
    let a = match decompress(pk) {
        Some(p) => p,
        None => return false,
    };
    let mut buf = Vec::with_capacity(64 + msg.len());
    buf.extend_from_slice(&rb);
    buf.extend_from_slice(pk);
    buf.extend_from_slice(msg);
    let k = hash_to_sc(&buf);
    let lhs = pmul(&BASE, &s);
    let rhs = padd(&r, &pmul(&a, &k));
    compress(&lhs) == compress(&rhs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
            .collect()
    }

    /// RFC 8032 §7.1 TEST 1 — empty message.
    #[test]
    fn rfc8032_vector_1() {
        let seed: [u8; 32] =
            unhex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .try_into()
                .unwrap();
        let (pk, sk) = keypair(&seed);
        assert_eq!(
            unhex("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"),
            pk
        );
        let sig = sign(&sk, b"");
        assert_eq!(
            unhex("e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"),
            sig.to_vec()
        );
        assert!(verify(&pk, b"", &sig));
    }

    /// RFC 8032 §7.1 TEST 2 — message `0x72`.
    #[test]
    fn rfc8032_vector_2() {
        let seed: [u8; 32] =
            unhex("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb")
                .try_into()
                .unwrap();
        let (pk, sk) = keypair(&seed);
        assert_eq!(
            unhex("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"),
            pk
        );
        let sig = sign(&sk, &[0x72]);
        assert_eq!(
            unhex("92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"),
            sig.to_vec()
        );
        assert!(verify(&pk, &[0x72], &sig));
    }

    /// RFC 8032 §7.1 TEST 3 — message `0xaf82`.
    #[test]
    fn rfc8032_vector_3() {
        let seed: [u8; 32] =
            unhex("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7")
                .try_into()
                .unwrap();
        let (pk, sk) = keypair(&seed);
        assert_eq!(
            unhex("fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025"),
            pk
        );
        let sig = sign(&sk, &[0xaf, 0x82]);
        assert_eq!(
            unhex("6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a"),
            sig.to_vec()
        );
        assert!(verify(&pk, &[0xaf, 0x82], &sig));
    }

    #[test]
    fn tamper_rejected() {
        let (pk, sk) = keypair(b"01234567890123456789012345678901");
        let sig = sign(&sk, b"hello world");
        assert!(verify(&pk, b"hello world", &sig));
        assert!(!verify(&pk, b"hello world!", &sig));
        let mut sig2 = sig;
        sig2[10] ^= 1;
        assert!(!verify(&pk, b"hello world", &sig2));
        let mut pk2 = pk;
        pk2[0] ^= 1;
        assert!(!verify(&pk2, b"hello world", &sig));
    }

    #[test]
    fn noncanonical_s_rejected() {
        let (pk, sk) = keypair(b"01234567890123456789012345678901");
        let mut sig = sign(&sk, b"m");
        // S = L exactly → must be rejected
        sig[32..].copy_from_slice(&[
            0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9,
            0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x10,
        ]);
        assert!(!verify(&pk, b"m", &sig));
    }

    #[test]
    fn decompress_rejects_bad_encoding() {
        // y ≥ p rejected: all-FF with cleared sign bit is > p
        let mut bad = [0xffu8; 32];
        bad[31] = 0x7f;
        assert!(decompress(&bad).is_none());
    }

    #[test]
    fn scalar_basepoint_roundtrip() {
        let one: Sc = [1, 0, 0, 0];
        assert_eq!(compress(&pmul(&BASE, &one)), compress(&BASE));
    }

    #[test]
    fn group_law_sanity() {
        let a: Sc = [7, 0, 0, 0];
        let b: Sc = [11, 0, 0, 0];
        let ab: Sc = [18, 0, 0, 0];
        assert_eq!(
            compress(&pmul(&BASE, &ab)),
            compress(&padd(&pmul(&BASE, &a), &pmul(&BASE, &b)))
        );
    }
}
