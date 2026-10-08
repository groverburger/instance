//! 4×4 matrices (row-major, acting on column vectors: p' = M·p).

pub type M4 = [[f64; 4]; 4];

pub const IDENT: M4 = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

pub fn mul(a: &M4, b: &M4) -> M4 {
    let mut r = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j] + a[i][3] * b[3][j];
        }
    }
    r
}

#[inline]
pub fn apply(m: &M4, p: [f64; 4]) -> [f64; 4] {
    let mut r = [0.0; 4];
    for (i, row) in m.iter().enumerate() {
        r[i] = row[0] * p[0] + row[1] * p[1] + row[2] * p[2] + row[3] * p[3];
    }
    r
}

/// Transforms a point (w = 1) and returns xyz (ignores the projective row).
#[inline]
pub fn point(m: &M4, x: f64, y: f64, z: f64) -> [f64; 3] {
    let r = apply(m, [x, y, z, 1.0]);
    [r[0], r[1], r[2]]
}

/// Transforms a direction (w = 0).
#[inline]
pub fn dir(m: &M4, x: f64, y: f64, z: f64) -> [f64; 3] {
    let r = apply(m, [x, y, z, 0.0]);
    [r[0], r[1], r[2]]
}

pub fn translate(x: f64, y: f64, z: f64) -> M4 {
    [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z], [0.0, 0.0, 0.0, 1.0]]
}

pub fn scale(x: f64, y: f64, z: f64) -> M4 {
    [[x, 0.0, 0.0, 0.0], [0.0, y, 0.0, 0.0], [0.0, 0.0, z, 0.0], [0.0, 0.0, 0.0, 1.0]]
}

/// Rotation by `deg` degrees about the axis (x, y, z) (right-handed formula in
/// the coordinate system's own handedness).
pub fn rotate_axis(x: f64, y: f64, z: f64, deg: f64) -> M4 {
    let len = (x * x + y * y + z * z).sqrt();
    if len < 1e-12 {
        return IDENT;
    }
    let (x, y, z) = (x / len, y / len, z / len);
    let a = deg.to_radians();
    let (c, s) = (a.cos(), a.sin());
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y, 0.0],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x, 0.0],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// General inverse (None if singular).
pub fn invert(m: &M4) -> Option<M4> {
    let mut a = *m;
    let mut inv = IDENT;
    for col in 0..4 {
        let mut piv = col;
        for r in col + 1..4 {
            if a[r][col].abs() > a[piv][col].abs() {
                piv = r;
            }
        }
        if a[piv][col].abs() < 1e-14 {
            return None;
        }
        a.swap(col, piv);
        inv.swap(col, piv);
        let d = a[col][col];
        for j in 0..4 {
            a[col][j] /= d;
            inv[col][j] /= d;
        }
        for r in 0..4 {
            if r != col {
                let f = a[r][col];
                if f != 0.0 {
                    for j in 0..4 {
                        a[r][j] -= f * a[col][j];
                        inv[r][j] -= f * inv[col][j];
                    }
                }
            }
        }
    }
    Some(inv)
}

pub fn transpose(m: &M4) -> M4 {
    let mut r = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            r[i][j] = m[j][i];
        }
    }
    r
}

pub fn is_identity(m: &M4) -> bool {
    *m == IDENT
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inverse_roundtrip() {
        let m = mul(&translate(1.0, 2.0, 3.0), &mul(&rotate_axis(0.0, 0.0, 1.0, 30.0), &scale(2.0, 3.0, 4.0)));
        let i = invert(&m).unwrap();
        let p = mul(&m, &i);
        for r in 0..4 {
            for c in 0..4 {
                assert!((p[r][c] - IDENT[r][c]).abs() < 1e-9);
            }
        }
    }
}
