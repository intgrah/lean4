#![allow(dead_code)]

use lean_corpus::Record;

pub trait ReplayFn: Send + Sync + 'static {
    fn replay(&self, record: &Record) -> ReplayOutcome;
}

#[derive(Debug, Eq, PartialEq)]
pub enum ReplayOutcome {
    Match,
    Diverged { detail: String },
    Skipped { reason: String },
}

pub struct Registry {
    entries: std::collections::BTreeMap<String, Box<dyn ReplayFn>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            entries: std::collections::BTreeMap::new(),
        }
    }

    pub fn register(&mut self, fqn: &str, f: Box<dyn ReplayFn>) {
        self.entries.insert(fqn.to_string(), f);
    }

    pub fn get(&self, fqn: &str) -> Option<&dyn ReplayFn> {
        self.entries.get(fqn).map(|b| b.as_ref())
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin() -> Registry {
    Registry::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysMatch;
    impl ReplayFn for AlwaysMatch {
        fn replay(&self, _record: &Record) -> ReplayOutcome {
            ReplayOutcome::Match
        }
    }

    #[test]
    fn register_and_lookup() {
        let mut r = Registry::new();
        assert!(r.get("Foo.bar").is_none());
        r.register("Foo.bar", Box::new(AlwaysMatch));
        assert!(r.get("Foo.bar").is_some());
        assert!(r.get("Other").is_none());
        let names: Vec<&str> = r.names().collect();
        assert_eq!(names, vec!["Foo.bar"]);
    }

    #[test]
    fn always_match_outcome() {
        let f = AlwaysMatch;
        let rec = Record::default();
        assert_eq!(f.replay(&rec), ReplayOutcome::Match);
    }

    #[test]
    fn builtin_starts_empty() {
        let r = builtin();
        assert_eq!(r.names().count(), 0);
    }
}
