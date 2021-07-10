use custom_debug_derive::Debug;
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

fn opt_fmt<T: std::fmt::Display>(t: &Option<T>, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    match t {
        Some(t) => write!(f, "{}", t),
        None => write!(f, "None"),
    }
}

fn set_fmt<S, T>(ts: &S, f: &mut std::fmt::Formatter) -> std::fmt::Result
where
    for<'a> &'a S: IntoIterator<Item = &'a T>,
    T: std::fmt::Display,
{
    write!(f, "{{ ")?;
    for t in ts.into_iter() {
        write!(f, "{}, ", t)?;
    }
    write!(f, "}} ")
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Ord, PartialOrd)]
struct Fact {
    #[debug(with = "opt_fmt")]
    pub base_location: Option<Location>,
    pub kind: Constraint,
    pub count: usize,
    #[debug(with = "set_fmt")]
    pub proximity: BTreeSet<Location>,
}

impl Fact {
    fn new(
        kind: Constraint,
        count: usize,
        proximity: BTreeSet<Location>,
        base_location: Option<Location>,
    ) -> Self {
        Self {
            kind,
            count,
            proximity,
            base_location,
        }
    }

    fn derive_proximity(&self, proximity: BTreeSet<Location>) -> Self {
        Self {
            proximity,
            kind: self.kind,
            count: self.count,
            base_location: None,
        }
    }

    fn derive_count(&self, count: usize) -> Self {
        Self {
            kind: self.kind,
            count,
            proximity: self.proximity.clone(),
            base_location: None,
        }
    }

    fn derive_kind(&self, kind: Constraint) -> Self {
        Self {
            kind,
            count: self.count,
            proximity: self.proximity.clone(),
            base_location: None,
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
            .map(|(l, s)| Fact::new(Constraint::Exact, s, make_proximity(l), Some(l)))
            .collect();
        Self { facts }
    }

    fn iter(&self) -> impl Iterator<Item = &Fact> {
        self.facts.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed() {
        let grid = "13m3m211
                         m4m423m1
                         m5e3m21e
                         mmem3321
                         eeee3mm2
                         1m2m223m
                        ";
        let mf = Minefield::new_active_game(&grid);
        let repo = Repository::seed(&mf);

        let expected = vec![
            // row 0
            fact((0, 0), 1, [(0, 1)]),
            fact((1, 0), 3, [(2, 0), (2, 1), (0, 1)]),
            fact((3, 0), 3, [(2, 0), (4, 0), (2, 1)]),
            fact((5, 0), 2, [(4, 0), (6, 1)]),
            fact((6, 0), 1, [(6, 1)]),
            fact((7, 0), 1, [(6, 1)]),
            // row 1
            fact((1, 1), 4, [(2, 0), (0, 1), (2, 1), (0, 2), (2, 2)]),
            fact((3, 1), 4, [(2, 0), (4, 0), (2, 1), (4, 2), (2, 2)]),
            fact((4, 1), 2, [(4, 2), (4, 0)]),
            fact((5, 1), 3, [(4, 0), (4, 2), (6, 1)]),
            fact((7, 1), 1, [(6, 1), (7, 2)]),
            // row 2
            fact(
                (1, 2),
                5,
                [(0, 1), (2, 1), (0, 2), (2, 2), (0, 3), (1, 3), (2, 3)],
            ),
            fact((3, 2), 3, [(2, 1), (2, 2), (4, 2), (2, 3), (3, 3)]),
            fact((5, 2), 2, [(6, 1), (4, 2)]),
            fact((6, 2), 1, [(6, 1), (7, 2)]),
            // row 3
            fact((4, 3), 3, [(4, 2), (3, 3), (3, 4), (5, 4)]),
            fact((5, 3), 3, [(4, 2), (5, 4), (6, 4)]),
            fact((6, 3), 2, [(7, 2), (5, 4), (6, 4)]),
            fact((7, 3), 1, [(7, 2), (6, 4)]),
            // row 4
            fact((4, 4), 3, [(3, 3), (3, 4), (5, 4), (3, 5)]),
            fact((7, 4), 2, [(6, 4), (7, 5)]),
            // row 5
            fact((0, 5), 1, [(0, 4), (1, 4), (1, 5)]),
            fact((2, 5), 2, [(1, 4), (2, 4), (3, 4), (1, 5), (3, 5)]),
            fact((4, 5), 2, [(3, 4), (5, 4), (3, 5)]),
            fact((5, 5), 2, [(5, 4), (6, 4)]),
            fact((6, 5), 3, [(5, 4), (6, 4), (7, 5)]),
        ];

        let actual = repo.facts.into_iter().collect();
        check_facts(expected, actual);
    }

    fn fact<const N: usize>(
        l: (usize, usize),
        mine_count: usize,
        proximity: [(usize, usize); N],
    ) -> Fact {
        let proximity = std::array::IntoIter::new(proximity)
            .map(Into::into)
            .collect();
        Fact::new(Constraint::Exact, mine_count, proximity, Some(l.into()))
    }

    fn check_facts(mut expected: Vec<Fact>, mut actual: Vec<Fact>) {
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(expected.len(), actual.len(), "Different number of facts!");
        for (e, a) in expected.into_iter().zip(actual.into_iter()) {
            assert_eq!(e, a);
        }
    }
}
