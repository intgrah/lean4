use std::cmp::Ordering;

use crate::level_codec::{DecodedLevel, DecodedName};

impl DecodedName {
    fn cmp_name(&self, other: &DecodedName) -> Ordering {
        match (self, other) {
            (DecodedName::Anonymous, DecodedName::Anonymous) => Ordering::Equal,
            (DecodedName::Anonymous, _) => Ordering::Less,
            (_, DecodedName::Anonymous) => Ordering::Greater,
            (DecodedName::Num(p1, i1), DecodedName::Num(p2, i2)) => match p1.cmp_name(p2) {
                Ordering::Equal => i1.cmp(i2),
                ord => ord,
            },
            (DecodedName::Num(_, _), DecodedName::Str(_, _)) => Ordering::Less,
            (DecodedName::Str(_, _), DecodedName::Num(_, _)) => Ordering::Greater,
            (DecodedName::Str(p1, n1), DecodedName::Str(p2, n2)) => match p1.cmp_name(p2) {
                Ordering::Equal => n1.cmp(n2),
                ord => ord,
            },
        }
    }

    fn name_lt(&self, other: &DecodedName) -> bool {
        self.cmp_name(other) == Ordering::Less
    }
}

impl DecodedLevel {
    pub fn normalize(&self) -> DecodedLevel {
        if self.is_already_normalized_cheap() {
            return self.clone();
        }
        let k = self.get_offset();
        match self.get_level_offset() {
            DecodedLevel::Max(l1, l2) => {
                let mut lvls = Vec::new();
                Self::get_max_args_aux(l1, false, &mut lvls);
                Self::get_max_args_aux(l2, false, &mut lvls);
                lvls.sort_by(|a, b| {
                    if Self::norm_lt(a, b) {
                        Ordering::Less
                    } else if Self::norm_lt(b, a) {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                });
                let first_non_explicit = Self::skip_explicit(&lvls, 0);
                let i = if Self::is_explicit_subsumed(&lvls, first_non_explicit) {
                    first_non_explicit
                } else {
                    first_non_explicit.saturating_sub(1)
                };
                let prev = lvls[i].get_level_offset().clone();
                let prev_k = lvls[i].get_offset();
                Self::mk_max_aux(&lvls, k, i + 1, prev, prev_k, DecodedLevel::Zero)
            }
            DecodedLevel::IMax(l1, l2) => {
                if l2.is_never_zero() {
                    let merged = DecodedLevel::Max(l1.clone(), l2.clone());
                    merged.normalize().add_offset(k)
                } else {
                    let n1 = l1.normalize();
                    let n2 = l2.normalize();
                    Self::mk_imax_aux(n1, n2).add_offset(k)
                }
            }
            _ => unreachable!("normalize: level offset of a non-cheap level is max or imax"),
        }
    }

    fn is_zero(&self) -> bool {
        matches!(self, DecodedLevel::Zero)
    }

    fn ctor_to_nat(&self) -> u8 {
        match self {
            DecodedLevel::Zero => 0,
            DecodedLevel::Param(_) => 1,
            DecodedLevel::MVar(_) => 2,
            DecodedLevel::Succ(_) => 3,
            DecodedLevel::Max(_, _) => 4,
            DecodedLevel::IMax(_, _) => 5,
        }
    }

    fn get_offset(&self) -> usize {
        let mut cur = self;
        let mut n = 0;
        while let DecodedLevel::Succ(u) = cur {
            n += 1;
            cur = u;
        }
        n
    }

    fn get_level_offset(&self) -> &DecodedLevel {
        let mut cur = self;
        while let DecodedLevel::Succ(u) = cur {
            cur = u;
        }
        cur
    }

    fn is_never_zero(&self) -> bool {
        match self {
            DecodedLevel::Zero | DecodedLevel::Param(_) | DecodedLevel::MVar(_) => false,
            DecodedLevel::Succ(_) => true,
            DecodedLevel::Max(l1, l2) => l1.is_never_zero() || l2.is_never_zero(),
            DecodedLevel::IMax(_, l2) => l2.is_never_zero(),
        }
    }

    fn is_already_normalized_cheap(&self) -> bool {
        match self {
            DecodedLevel::Zero | DecodedLevel::Param(_) | DecodedLevel::MVar(_) => true,
            DecodedLevel::Succ(u) => u.is_already_normalized_cheap(),
            _ => false,
        }
    }

    fn add_offset(self, n: usize) -> DecodedLevel {
        let mut u = self;
        for _ in 0..n {
            u = DecodedLevel::Succ(Box::new(u));
        }
        u
    }

    fn mk_imax_aux(u: DecodedLevel, v: DecodedLevel) -> DecodedLevel {
        if v.is_zero() {
            return DecodedLevel::Zero;
        }
        if u.is_zero() {
            return v;
        }
        if let DecodedLevel::Succ(inner) = &u
            && inner.is_zero()
        {
            return v;
        }
        if u == v {
            u
        } else {
            DecodedLevel::IMax(Box::new(u), Box::new(v))
        }
    }

    fn norm_lt(l1: &DecodedLevel, l2: &DecodedLevel) -> bool {
        Self::norm_lt_aux(l1, 0, l2, 0)
    }

    fn norm_lt_aux(l1: &DecodedLevel, k1: usize, l2: &DecodedLevel, k2: usize) -> bool {
        match (l1, l2) {
            (DecodedLevel::Succ(a), _) => Self::norm_lt_aux(a, k1 + 1, l2, k2),
            (_, DecodedLevel::Succ(b)) => Self::norm_lt_aux(l1, k1, b, k2 + 1),
            (DecodedLevel::Max(a1, a2), DecodedLevel::Max(b1, b2)) => {
                if l1 == l2 {
                    k1 < k2
                } else if a1 != b1 {
                    Self::norm_lt_aux(a1, 0, b1, 0)
                } else {
                    Self::norm_lt_aux(a2, 0, b2, 0)
                }
            }
            (DecodedLevel::IMax(a1, a2), DecodedLevel::IMax(b1, b2)) => {
                if l1 == l2 {
                    k1 < k2
                } else if a1 != b1 {
                    Self::norm_lt_aux(a1, 0, b1, 0)
                } else {
                    Self::norm_lt_aux(a2, 0, b2, 0)
                }
            }
            (DecodedLevel::Param(n1), DecodedLevel::Param(n2)) => {
                if n1 == n2 {
                    k1 < k2
                } else {
                    n1.name_lt(n2)
                }
            }
            (DecodedLevel::MVar(n1), DecodedLevel::MVar(n2)) => {
                if n1 == n2 {
                    k1 < k2
                } else {
                    n1.name_lt(n2)
                }
            }
            _ => {
                if l1 == l2 {
                    k1 < k2
                } else {
                    l1.ctor_to_nat() < l2.ctor_to_nat()
                }
            }
        }
    }

    fn get_max_args_aux(l: &DecodedLevel, already_normalized: bool, lvls: &mut Vec<DecodedLevel>) {
        match l {
            DecodedLevel::Max(l1, l2) => {
                Self::get_max_args_aux(l1, already_normalized, lvls);
                Self::get_max_args_aux(l2, already_normalized, lvls);
            }
            _ if !already_normalized => {
                let n = l.normalize();
                Self::get_max_args_aux(&n, true, lvls);
            }
            _ => lvls.push(l.clone()),
        }
    }

    fn acc_max(result: DecodedLevel, prev: DecodedLevel, offset: usize) -> DecodedLevel {
        let shifted = prev.add_offset(offset);
        if result.is_zero() {
            shifted
        } else {
            DecodedLevel::Max(Box::new(result), Box::new(shifted))
        }
    }

    fn mk_max_aux(
        lvls: &[DecodedLevel],
        extra_k: usize,
        start: usize,
        prev: DecodedLevel,
        prev_k: usize,
        result: DecodedLevel,
    ) -> DecodedLevel {
        let mut prev = prev;
        let mut prev_k = prev_k;
        let mut result = result;
        for lvl in &lvls[start.min(lvls.len())..] {
            let curr = lvl.get_level_offset();
            let curr_k = lvl.get_offset();
            if *curr == prev {
                prev_k = curr_k;
            } else {
                result = Self::acc_max(result, prev, extra_k + prev_k);
                prev = curr.clone();
                prev_k = curr_k;
            }
        }
        Self::acc_max(result, prev, extra_k + prev_k)
    }

    fn skip_explicit(lvls: &[DecodedLevel], start: usize) -> usize {
        let mut i = start;
        while i < lvls.len() && lvls[i].get_level_offset().is_zero() {
            i += 1;
        }
        i
    }

    fn is_explicit_subsumed(lvls: &[DecodedLevel], first_non_explicit: usize) -> bool {
        if first_non_explicit == 0 {
            return false;
        }
        let max = lvls[first_non_explicit - 1].get_offset();
        Self::is_explicit_subsumed_aux(lvls, max, first_non_explicit)
    }

    fn is_explicit_subsumed_aux(lvls: &[DecodedLevel], max_explicit: usize, start: usize) -> bool {
        let mut i = start;
        while i < lvls.len() {
            if lvls[i].get_offset() >= max_explicit {
                return true;
            }
            i += 1;
        }
        false
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

    fn imax(a: DecodedLevel, b: DecodedLevel) -> DecodedLevel {
        DecodedLevel::IMax(Box::new(a), Box::new(b))
    }

    fn param(s: &[u8]) -> DecodedLevel {
        DecodedLevel::Param(DecodedName::Str(
            Box::new(DecodedName::Anonymous),
            s.to_vec(),
        ))
    }

    fn nat(n: usize) -> DecodedLevel {
        let mut l = zero();
        for _ in 0..n {
            l = succ(l);
        }
        l
    }

    #[test]
    fn cheap_levels_are_unchanged() {
        assert_eq!(zero().normalize(), zero());
        assert_eq!(nat(3).normalize(), nat(3));
        assert_eq!(param(b"u").normalize(), param(b"u"));
        assert_eq!(succ(param(b"u")).normalize(), succ(param(b"u")));
    }

    #[test]
    fn max_of_equal_collapses() {
        assert_eq!(max(param(b"u"), param(b"u")).normalize(), param(b"u"));
    }

    #[test]
    fn max_with_zero_drops_zero() {
        assert_eq!(max(zero(), param(b"u")).normalize(), param(b"u"));
        assert_eq!(max(param(b"u"), zero()).normalize(), param(b"u"));
    }

    #[test]
    fn max_keeps_larger_offset() {
        assert_eq!(
            max(param(b"u"), succ(param(b"u"))).normalize(),
            succ(param(b"u"))
        );
        assert_eq!(
            max(succ(param(b"u")), param(b"u")).normalize(),
            succ(param(b"u"))
        );
    }

    #[test]
    fn max_of_explicit_numerals() {
        assert_eq!(max(nat(1), nat(3)).normalize(), nat(3));
        assert_eq!(max(nat(3), nat(1)).normalize(), nat(3));
    }

    #[test]
    fn imax_with_zero_rhs_is_zero() {
        assert_eq!(imax(param(b"u"), zero()).normalize(), zero());
    }

    #[test]
    fn imax_with_never_zero_rhs_becomes_max() {
        assert_eq!(
            imax(param(b"u"), succ(zero())).normalize(),
            max(succ(zero()), param(b"u")).normalize()
        );
    }

    #[test]
    fn normalize_is_idempotent() {
        let samples = [
            max(param(b"v"), max(param(b"u"), param(b"v"))),
            imax(param(b"u"), max(param(b"v"), succ(zero()))),
            max(succ(max(param(b"a"), param(b"b"))), param(b"a")),
            max(param(b"b"), param(b"a")),
        ];
        for s in samples {
            let n = s.normalize();
            assert_eq!(n.normalize(), n, "not idempotent on {s:?}");
        }
    }

    #[test]
    fn param_order_is_lexicographic() {
        assert_eq!(
            max(param(b"b"), param(b"a")).normalize(),
            max(param(b"a"), param(b"b"))
        );
    }
}
