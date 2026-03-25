//! Complex number arithmetic — cache-friendly, SIMD-ready layout.
//!
//! `#[repr(C)]` ensures (re, im) are contiguous for vectorization.
//! All operations are branchless for pipeline friendliness.

/// A complex number with SIMD-friendly layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const ZERO: Complex = Complex { re: 0.0, im: 0.0 };
    pub const ONE: Complex = Complex { re: 1.0, im: 0.0 };
    pub const I: Complex = Complex { re: 0.0, im: 1.0 };

    pub fn new(re: f64, im: f64) -> Self { Self { re, im } }
    pub fn from_polar(r: f64, theta: f64) -> Self {
        Self { re: r * theta.cos(), im: r * theta.sin() }
    }

    pub fn add(self, other: Complex) -> Complex {
        Complex { re: self.re + other.re, im: self.im + other.im }
    }
    pub fn sub(self, other: Complex) -> Complex {
        Complex { re: self.re - other.re, im: self.im - other.im }
    }
    /// (a+bi)(c+di) = (ac-bd) + (ad+bc)i
    pub fn mul(self, other: Complex) -> Complex {
        Complex {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
    /// (a+bi)/(c+di) = ((ac+bd) + (bc-ad)i) / (c²+d²)
    pub fn div(self, other: Complex) -> Complex {
        let denom = other.re * other.re + other.im * other.im;
        Complex {
            re: (self.re * other.re + self.im * other.im) / denom,
            im: (self.im * other.re - self.re * other.im) / denom,
        }
    }
    pub fn conjugate(self) -> Complex { Complex { re: self.re, im: -self.im } }
    pub fn magnitude(self) -> f64 { (self.re * self.re + self.im * self.im).sqrt() }
    pub fn magnitude_squared(self) -> f64 { self.re * self.re + self.im * self.im }
    pub fn phase(self) -> f64 { self.im.atan2(self.re) }
    pub fn neg(self) -> Complex { Complex { re: -self.re, im: -self.im } }

    /// e^(a+bi) = e^a * (cos(b) + i*sin(b))
    pub fn exp(self) -> Complex {
        let ea = self.re.exp();
        Complex { re: ea * self.im.cos(), im: ea * self.im.sin() }
    }
    /// ln(z) = ln|z| + i*arg(z)
    pub fn ln(self) -> Complex {
        Complex { re: self.magnitude().ln(), im: self.phase() }
    }
    /// z^n (integer power, by repeated squaring)
    pub fn powi(self, n: i32) -> Complex {
        if n == 0 { return Complex::ONE; }
        let mut result = Complex::ONE;
        let mut base = if n > 0 { self } else { Complex::ONE.div(self) };
        let mut exp = n.unsigned_abs();
        while exp > 0 {
            if exp & 1 == 1 { result = result.mul(base); }
            base = base.mul(base);
            exp >>= 1;
        }
        result
    }
    /// sqrt(z) — principal square root
    pub fn sqrt(self) -> Complex {
        let r = self.magnitude();
        let phase = self.phase() / 2.0;
        Complex::from_polar(r.sqrt(), phase)
    }
}

/// Quaternion — for 3D rotations and spatial computing.
/// Layout: w + xi + yj + zk
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quaternion {
    pub const IDENTITY: Quaternion = Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };

    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self { Self { w, x, y, z } }

    /// Create from axis-angle rotation.
    pub fn from_axis_angle(axis: [f64; 3], angle: f64) -> Self {
        let half = angle / 2.0;
        let s = half.sin();
        let norm = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        Self {
            w: half.cos(),
            x: axis[0] / norm * s,
            y: axis[1] / norm * s,
            z: axis[2] / norm * s,
        }
    }

    pub fn add(self, other: Quaternion) -> Quaternion {
        Quaternion { w: self.w + other.w, x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }

    /// Hamilton product.
    pub fn mul(self, other: Quaternion) -> Quaternion {
        Quaternion {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    pub fn conjugate(self) -> Quaternion {
        Quaternion { w: self.w, x: -self.x, y: -self.y, z: -self.z }
    }

    pub fn magnitude(self) -> f64 {
        (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(self) -> Quaternion {
        let m = self.magnitude();
        Quaternion { w: self.w / m, x: self.x / m, y: self.y / m, z: self.z / m }
    }

    /// Rotate a 3D point by this quaternion: q * p * q⁻¹
    pub fn rotate_point(self, point: [f64; 3]) -> [f64; 3] {
        let p = Quaternion::new(0.0, point[0], point[1], point[2]);
        let result = self.mul(p).mul(self.conjugate());
        [result.x, result.y, result.z]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complex_add() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, 4.0);
        assert_eq!(a.add(b), Complex::new(4.0, 6.0));
    }

    #[test]
    fn test_complex_mul() {
        // (1+2i)(3+4i) = (3-8) + (4+6)i = -5+10i
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, 4.0);
        assert_eq!(a.mul(b), Complex::new(-5.0, 10.0));
    }

    #[test]
    fn test_complex_div() {
        let a = Complex::new(1.0, 0.0);
        let b = Complex::new(0.0, 1.0);
        let r = a.div(b);
        assert!((r.re - 0.0).abs() < 1e-10);
        assert!((r.im - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_complex_magnitude() {
        assert!((Complex::new(3.0, 4.0).magnitude() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_complex_exp() {
        // e^(iπ) = -1
        let z = Complex::new(0.0, std::f64::consts::PI);
        let result = z.exp();
        assert!((result.re - (-1.0)).abs() < 1e-10);
        assert!(result.im.abs() < 1e-10);
    }

    #[test]
    fn test_complex_sqrt() {
        // sqrt(-1) = i
        let z = Complex::new(-1.0, 0.0);
        let r = z.sqrt();
        assert!(r.re.abs() < 1e-10);
        assert!((r.im - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_complex_powi() {
        let z = Complex::new(0.0, 1.0); // i
        let r = z.powi(4); // i^4 = 1
        assert!((r.re - 1.0).abs() < 1e-10);
        assert!(r.im.abs() < 1e-10);
    }

    #[test]
    fn test_quaternion_identity() {
        let q = Quaternion::IDENTITY;
        let p = [1.0, 2.0, 3.0];
        let rotated = q.rotate_point(p);
        assert!((rotated[0] - 1.0).abs() < 1e-10);
        assert!((rotated[1] - 2.0).abs() < 1e-10);
        assert!((rotated[2] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_quaternion_90deg_rotation() {
        // 90° rotation around Z axis: (1,0,0) → (0,1,0)
        let q = Quaternion::from_axis_angle([0.0, 0.0, 1.0], std::f64::consts::FRAC_PI_2);
        let p = q.rotate_point([1.0, 0.0, 0.0]);
        assert!((p[0]).abs() < 1e-10);
        assert!((p[1] - 1.0).abs() < 1e-10);
        assert!((p[2]).abs() < 1e-10);
    }

    #[test]
    fn test_quaternion_mul_identity() {
        let q = Quaternion::from_axis_angle([1.0, 0.0, 0.0], 1.0);
        let r = q.mul(Quaternion::IDENTITY);
        assert!((r.w - q.w).abs() < 1e-10);
        assert!((r.x - q.x).abs() < 1e-10);
    }
}
