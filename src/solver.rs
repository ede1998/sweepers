use std::collections::{BTreeSet, HashSet};

use crate::core::{Location, Minefield, State};

trait Without<T> {
    fn without(&self, element: &T) -> Self;
}

impl<T> Without<T> for BTreeSet<T>
where
    T: Ord + Clone,
{
    fn without(&self, element: &T) -> Self {
        let mut result = (*self).clone();
        result.remove(element);
        result
    }
}

trait Rule {
    fn derive(repo: &Repository) -> Vec<Fact>;
}

struct MinAllToExact;

impl Rule for MinAllToExact {
    fn derive(repo: &Repository) -> Vec<Fact> {
        repo.iter()
            .filter(|f| f.is_min() && f.cardinality() == f.count)
            .map(|f| f.derive_kind(Constraint::Exact))
            .collect()
    }
}

struct MaxZeroToExact;

impl Rule for MaxZeroToExact {
    fn derive(repo: &Repository) -> Vec<Fact> {
        repo.iter()
            .filter(|f| f.is_max() && f.count == 0)
            .map(|f| f.derive_kind(Constraint::Exact))
            .collect()
    }
}
struct ExactToMin;

impl Rule for ExactToMin {
    fn derive(repo: &Repository) -> Vec<Fact> {
        repo.iter()
            .filter(|f| f.is_exact())
            .map(|f| f.derive_kind(Constraint::Min))
            .collect()
    }
}

struct ExactToMax;

impl Rule for ExactToMax {
    fn derive(repo: &Repository) -> Vec<Fact> {
        repo.iter()
            .filter(|f| f.is_exact())
            .map(|f| f.derive_kind(Constraint::Max))
            .collect()
    }
}

struct MaxRemoveLocations;

impl Rule for MaxRemoveLocations {
    fn derive(repo: &Repository) -> Vec<Fact> {
        repo.iter()
            .filter(|f| f.is_max())
            .flat_map(|f| {
                f.proximity
                    .iter()
                    .map(move |l| f.derive_proximity(f.proximity.without(l)))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Constraint {
    Min,
    Exact,
    Max,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct Fact {
    pub kind: Constraint,
    pub count: usize,
    pub proximity: BTreeSet<Location>,
}

impl Fact {
    fn new(kind: Constraint, count: usize, proximity: BTreeSet<Location>) -> Self {
        Self {
            kind,
            count,
            proximity,
        }
    }

    fn derive_proximity(&self, proximity: BTreeSet<Location>) -> Self {
        Self { proximity, ..*self }
    }

    fn derive_count(&self, count: usize) -> Self {
        Self {
            kind: self.kind,
            count,
            proximity: self.proximity.clone(),
        }
    }

    fn derive_kind(&self, kind: Constraint) -> Self {
        Self {
            kind,
            count: self.count,
            proximity: self.proximity.clone(),
        }
    }

    fn cardinality(&self) -> usize {
        self.proximity.len()
    }

    fn is_min(&self) -> bool {
        matches!(self.kind, Constraint::Min)
    }

    fn is_max(&self) -> bool {
        matches!(self.kind, Constraint::Max)
    }

    fn is_exact(&self) -> bool {
        matches!(self.kind, Constraint::Exact)
    }
}

struct Repository {
    facts: HashSet<Fact>,
}

impl Repository {
    fn seed(mf: &Minefield) -> Self {
        let make_proximity = |l: Location| {
            l.neighbours()
                .filter(|&l| mf.fog().get(l).map(State::is_hidden).unwrap_or(false))
                .collect()
        };

        let facts = mf
            .fog()
            .loc_iter()
            .filter_map(|(l, s)| Some((l, *s.as_revealed()?)))
            .map(|(l, s)| Fact::new(Constraint::Exact, s, make_proximity(l)))
            .collect();
        Self { facts }
    }

    fn iter(&self) -> impl Iterator<Item = &Fact> {
        self.facts.iter()
    }
}

#[cfg(test)]
mod test_super {
    use super::*;

    #[test]
    fn test_() {}
}
