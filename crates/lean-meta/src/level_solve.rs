use crate::level_codec::DecodedLevel;

impl DecodedLevel {
    pub fn occurs(&self, v: &DecodedLevel) -> bool {
        if self == v {
            return true;
        }
        match v {
            DecodedLevel::Succ(a) => self.occurs(a),
            DecodedLevel::Max(a, b) | DecodedLevel::IMax(a, b) => self.occurs(a) || self.occurs(b),
            _ => false,
        }
    }

    pub fn strict_occurs_max(&self, l: &DecodedLevel) -> bool {
        match l {
            DecodedLevel::Max(u, v) => {
                self.strict_occurs_max_visit(u) || self.strict_occurs_max_visit(v)
            }
            _ => false,
        }
    }

    fn strict_occurs_max_visit(&self, u: &DecodedLevel) -> bool {
        match u {
            DecodedLevel::Max(a, b) => {
                self.strict_occurs_max_visit(a) || self.strict_occurs_max_visit(b)
            }
            _ => u != self && self.occurs(u),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::level_codec::{DecodedLevel, DecodedName};

    fn zero() -> DecodedLevel {
        DecodedLevel::Zero
    }
    fn succ(u: DecodedLevel) -> DecodedLevel {
        DecodedLevel::Succ(Box::new(u))
    }
    fn max(a: DecodedLevel, b: DecodedLevel) -> DecodedLevel {
        DecodedLevel::Max(Box::new(a), Box::new(b))
    }
    fn param(s: &[u8]) -> DecodedLevel {
        DecodedLevel::Param(DecodedName::Str(
            Box::new(DecodedName::Anonymous),
            s.to_vec(),
        ))
    }

    #[test]
    fn occurs_reflexive_and_nested() {
        assert!(param(b"u").occurs(&param(b"u")));
        assert!(param(b"u").occurs(&succ(succ(param(b"u")))));
        assert!(param(b"u").occurs(&max(zero(), param(b"u"))));
        assert!(!param(b"u").occurs(&max(zero(), param(b"v"))));
        assert!(!param(b"u").occurs(&succ(param(b"v"))));
    }

    #[test]
    fn strict_occurs_max_proper_subterm() {
        assert!(param(b"u").strict_occurs_max(&max(succ(param(b"u")), param(b"v"))));
        assert!(!param(b"u").strict_occurs_max(&max(param(b"u"), param(b"v"))));
        assert!(!param(b"u").strict_occurs_max(&succ(param(b"u"))));
        assert!(!param(b"u").strict_occurs_max(&param(b"u")));
        assert!(param(b"u").strict_occurs_max(&max(param(b"v"), succ(param(b"u")))));
    }
}
