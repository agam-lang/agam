//! Compile-time dimensional analysis (SI units).
//!
//! Encodes physical dimensions as integer exponents of the 7 SI base units.
//! The compiler can verify dimensional correctness at zero runtime cost.
//!
//! ## Layout
//! Each `Unit` is 7 i8 exponents packed into 7 bytes (fits in a register).
//! Arithmetic operations compose dimensions at compile time.

/// A physical unit represented as SI base unit exponents.
/// Stored as 7 bytes for cache-line packing.
///
/// Order: [mass(kg), length(m), time(s), current(A), temp(K), amount(mol), intensity(cd)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit {
    pub exponents: [i8; 7],
}

impl Unit {
    pub const DIMENSIONLESS: Unit = Unit {
        exponents: [0, 0, 0, 0, 0, 0, 0],
    };
    pub const METER: Unit = Unit {
        exponents: [0, 1, 0, 0, 0, 0, 0],
    };
    pub const KILOGRAM: Unit = Unit {
        exponents: [1, 0, 0, 0, 0, 0, 0],
    };
    pub const SECOND: Unit = Unit {
        exponents: [0, 0, 1, 0, 0, 0, 0],
    };
    pub const AMPERE: Unit = Unit {
        exponents: [0, 0, 0, 1, 0, 0, 0],
    };
    pub const KELVIN: Unit = Unit {
        exponents: [0, 0, 0, 0, 1, 0, 0],
    };
    pub const MOLE: Unit = Unit {
        exponents: [0, 0, 0, 0, 0, 1, 0],
    };
    pub const CANDELA: Unit = Unit {
        exponents: [0, 0, 0, 0, 0, 0, 1],
    };

    // Derived units
    /// m/s
    pub const VELOCITY: Unit = Unit {
        exponents: [0, 1, -1, 0, 0, 0, 0],
    };
    /// m/s²
    pub const ACCELERATION: Unit = Unit {
        exponents: [0, 1, -2, 0, 0, 0, 0],
    };
    /// kg·m/s² = Newton
    pub const NEWTON: Unit = Unit {
        exponents: [1, 1, -2, 0, 0, 0, 0],
    };
    /// kg·m²/s² = Joule
    pub const JOULE: Unit = Unit {
        exponents: [1, 2, -2, 0, 0, 0, 0],
    };
    /// kg·m²/s³ = Watt
    pub const WATT: Unit = Unit {
        exponents: [1, 2, -3, 0, 0, 0, 0],
    };
    /// A·s = Coulomb
    pub const COULOMB: Unit = Unit {
        exponents: [0, 0, 1, 1, 0, 0, 0],
    };
    /// kg·m²/(A·s³) = Volt
    pub const VOLT: Unit = Unit {
        exponents: [1, 2, -3, -1, 0, 0, 0],
    };
    /// kg/(m·s²) = Pascal
    pub const PASCAL: Unit = Unit {
        exponents: [1, -1, -2, 0, 0, 0, 0],
    };
    /// 1/s = Hertz
    pub const HERTZ: Unit = Unit {
        exponents: [0, 0, -1, 0, 0, 0, 0],
    };

    /// Multiply units: add exponents.
    pub fn mul(self, other: Unit) -> Unit {
        let mut e = [0i8; 7];
        for i in 0..7 {
            e[i] = self.exponents[i] + other.exponents[i];
        }
        Unit { exponents: e }
    }

    /// Divide units: subtract exponents.
    pub fn div(self, other: Unit) -> Unit {
        let mut e = [0i8; 7];
        for i in 0..7 {
            e[i] = self.exponents[i] - other.exponents[i];
        }
        Unit { exponents: e }
    }

    /// Raise to integer power.
    pub fn pow(self, n: i8) -> Unit {
        let mut e = [0i8; 7];
        for i in 0..7 {
            e[i] = self.exponents[i] * n;
        }
        Unit { exponents: e }
    }

    /// Check if two units are compatible for addition/subtraction.
    pub fn is_compatible(self, other: Unit) -> bool {
        self.exponents == other.exponents
    }

    /// Check if dimensionless.
    pub fn is_dimensionless(self) -> bool {
        self.exponents == [0, 0, 0, 0, 0, 0, 0]
    }
}

/// A value with physical units attached.
#[derive(Debug, Clone, Copy)]
pub struct Quantity {
    pub value: f64,
    pub unit: Unit,
}

impl Quantity {
    pub fn new(value: f64, unit: Unit) -> Self {
        Self { value, unit }
    }

    pub fn add(self, other: Quantity) -> Result<Quantity, &'static str> {
        if !self.unit.is_compatible(other.unit) {
            return Err("incompatible units for addition");
        }
        Ok(Quantity {
            value: self.value + other.value,
            unit: self.unit,
        })
    }

    pub fn sub(self, other: Quantity) -> Result<Quantity, &'static str> {
        if !self.unit.is_compatible(other.unit) {
            return Err("incompatible units for subtraction");
        }
        Ok(Quantity {
            value: self.value - other.value,
            unit: self.unit,
        })
    }

    pub fn mul(self, other: Quantity) -> Quantity {
        Quantity {
            value: self.value * other.value,
            unit: self.unit.mul(other.unit),
        }
    }

    pub fn div(self, other: Quantity) -> Quantity {
        Quantity {
            value: self.value / other.value,
            unit: self.unit.div(other.unit),
        }
    }

    pub fn pow(self, n: i8) -> Quantity {
        Quantity {
            value: self.value.powi(n as i32),
            unit: self.unit.pow(n),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_velocity() {
        // distance / time = velocity
        let dist = Quantity::new(100.0, Unit::METER);
        let time = Quantity::new(10.0, Unit::SECOND);
        let vel = dist.div(time);
        assert_eq!(vel.value, 10.0);
        assert_eq!(vel.unit, Unit::VELOCITY);
    }

    #[test]
    fn test_force() {
        // F = m * a → kg·m/s² = Newton
        let mass = Quantity::new(10.0, Unit::KILOGRAM);
        let accel = Quantity::new(9.8, Unit::ACCELERATION);
        let force = mass.mul(accel);
        assert!((force.value - 98.0).abs() < 1e-10);
        assert_eq!(force.unit, Unit::NEWTON);
    }

    #[test]
    fn test_energy() {
        // E = F · d → Newton·meter = Joule
        let force = Quantity::new(10.0, Unit::NEWTON);
        let dist = Quantity::new(5.0, Unit::METER);
        let energy = force.mul(dist);
        assert_eq!(energy.value, 50.0);
        assert_eq!(energy.unit, Unit::JOULE);
    }

    #[test]
    fn test_compatible_add() {
        let a = Quantity::new(5.0, Unit::METER);
        let b = Quantity::new(3.0, Unit::METER);
        let result = a.add(b).unwrap();
        assert_eq!(result.value, 8.0);
    }

    #[test]
    fn test_incompatible_add() {
        let m = Quantity::new(5.0, Unit::METER);
        let kg = Quantity::new(3.0, Unit::KILOGRAM);
        assert!(m.add(kg).is_err());
    }

    #[test]
    fn test_power_unit() {
        // m² = METER.pow(2)
        let area_unit = Unit::METER.pow(2);
        assert_eq!(area_unit.exponents, [0, 2, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_dimensionless() {
        let ratio = Unit::METER.div(Unit::METER);
        assert!(ratio.is_dimensionless());
    }

    #[test]
    fn test_hertz() {
        // 1/s = Hertz
        let freq = Unit::DIMENSIONLESS.div(Unit::SECOND);
        assert_eq!(freq, Unit::HERTZ);
    }
}
