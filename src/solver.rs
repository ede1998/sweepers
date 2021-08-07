use custom_debug_derive::Debug;

use std::{
    collections::{BTreeSet, HashSet},
    fmt::Display,
    io::{LineWriter, Write},
    path::Path,
    str,
};

use crate::core::{Location, Minefield, State};

trait Rule: std::fmt::Debug {
    fn derive(&self, repo: &Solver) -> Vec<Fact>;
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

/// If a set of N location has at least N mines, it has exactly N mines.
#[derive(Debug)]
struct MinAllToExact;

impl Rule for MinAllToExact {
    fn derive(&self, repo: &Solver) -> Vec<Fact> {
        repo.iter_previous_iteration()
            .filter(|f| f.is_min() && f.cardinality() == f.count)
            .map(|f| f.derive_kind(Constraint::Exact, repo.iteration, self, f))
            .collect()
    }
}

/// If a set of location has at most 0 mines, it has exactly 0 mines.
#[derive(Debug)]
struct MaxZeroToExact;

impl Rule for MaxZeroToExact {
    fn derive(&self, repo: &Solver) -> Vec<Fact> {
        repo.iter_previous_iteration()
            .filter(|f| f.is_max() && f.count == 0)
            .map(|f| f.derive_kind(Constraint::Exact, repo.iteration, self, f))
            .collect()
    }
}
/// If a set of locations has exactly N mines, it has at least N mines.
#[derive(Debug)]
struct ExactToMin;

impl Rule for ExactToMin {
    fn derive(&self, repo: &Solver) -> Vec<Fact> {
        repo.iter_previous_iteration()
            .filter(|f| f.is_exact())
            .map(|f| f.derive_kind(Constraint::Min, repo.iteration, self, f))
            .collect()
    }
}

/// If a set of locations has exactly N mines, it has at most N mines.
#[derive(Debug)]
struct ExactToMax;

impl Rule for ExactToMax {
    fn derive(&self, repo: &Solver) -> Vec<Fact> {
        repo.iter_previous_iteration()
            .filter(|f| f.is_exact())
            .map(|f| f.derive_kind(Constraint::Max, repo.iteration, self, f))
            .collect()
    }
}

/// If a min proximity is a true subset of a max proximity and the max proximity has more or equal number of mines,
/// then the remaining proximity max without min has at most the remaining mines of max - min.
#[derive(Debug)]
struct MinWithinMaxCombinator;

impl Rule for MinWithinMaxCombinator {
    fn derive(&self, repo: &Solver) -> Vec<Fact> {
        repo.iter_new_with_old()
            .filter_map(|(l, r)| match (l.kind, r.kind) {
                (Constraint::Min, Constraint::Max) => Some((l, r)),
                (Constraint::Max, Constraint::Min) => Some((r, l)),
                _ => None,
            })
            .filter(|(min, max)| {
                max.count >= min.count
                    && min.proximity.len() < max.proximity.len()
                    && min.proximity.is_subset(&max.proximity)
            })
            .map(|(min, max)| {
                Fact::new(
                    Constraint::Max,
                    max.count - min.count,
                    &max.proximity - &min.proximity,
                    repo.iteration,
                    FactDebug::derived_two(self, min, max),
                )
            })
            .collect()
    }
}

/// If a max proximity is intersecting a min proximity and the min proximity has more or equal number of mines,
/// then the remaining proximity min without max has at least the remaining mines of min - max.
#[derive(Debug)]
struct MaxIntersectsMinCombinator;

impl Rule for MaxIntersectsMinCombinator {
    fn derive(&self, repo: &Solver) -> Vec<Fact> {
        repo.iter_new_with_old()
            .filter_map(|(l, r)| match (l.kind, r.kind) {
                (Constraint::Min, Constraint::Max) => Some((l, r)),
                (Constraint::Max, Constraint::Min) => Some((r, l)),
                _ => None,
            })
            .filter_map(|(min, max)| {
                let intersection = &min.proximity & &max.proximity;
                if intersection.is_empty() {
                    // if min is disjoint to max, the two fact do not overlap and
                    // therefore nothing can be derived.
                    return None;
                }

                let max_mines_in_intersection = max.count.min(intersection.len());
                if min.count <= max_mines_in_intersection {
                    // if min has less mines in total than maximum in intersection, all mines
                    // could be in intersection and therefore no meaningful fact can be derived.
                    return None;
                }

                Some(Fact::new(
                    Constraint::Min,
                    min.count - max_mines_in_intersection,
                    &min.proximity - &max.proximity,
                    repo.iteration,
                    FactDebug::derived_two(self, min, max),
                ))
            })
            .collect()
    }
}

#[derive(Debug)]
struct Seeder;

impl Rule for Seeder {
    fn derive(&self, _: &Solver) -> Vec<Fact> {
        vec![]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Constraint {
    Min,
    Exact,
    Max,
}

impl Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let str = match self {
            Constraint::Min => "Min",
            Constraint::Exact => "Exact",
            Constraint::Max => "Max",
        };

        write!(f, "{}", str)
    }
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

#[derive(Debug, Clone)]
struct FactDebug {
    #[debug(with = "opt_fmt")]
    pub base_location: Option<Location>,
    pub produced_by: &'static str,
    #[cfg(feature = "derived_from")]
    pub derived_from: Vec<Fact>,
}

impl FactDebug {
    fn base<O: Into<Option<Location>>>(base_location: O, produced_by: &dyn Rule) -> Self {
        Self {
            base_location: base_location.into(),
            produced_by: produced_by.name(),
            #[cfg(feature = "derived_from")]
            derived_from: Vec::new(),
        }
    }

    fn derived_one(produced_by: &dyn Rule, _parent_fact: &Fact) -> Self {
        Self {
            base_location: None,
            produced_by: produced_by.name(),
            #[cfg(feature = "derived_from")]
            derived_from: vec![_parent_fact.clone()],
        }
    }

    fn derived_two(produced_by: &dyn Rule, _parent_fact_1: &Fact, _parent_fact_2: &Fact) -> Self {
        Self {
            base_location: None,
            produced_by: produced_by.name(),
            #[cfg(feature = "derived_from")]
            derived_from: vec![_parent_fact_1.clone(), _parent_fact_2.clone()],
        }
    }
}

#[derive(Debug, Clone)]
struct Fact {
    pub kind: Constraint,
    pub count: usize,
    #[debug(with = "set_fmt")]
    pub proximity: BTreeSet<Location>,
    pub iteration: usize,
    pub debug: FactDebug,
}

impl Eq for Fact {}

impl PartialEq for Fact {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.count == other.count && self.proximity == other.proximity
    }
}

impl Ord for Fact {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        if let ord @ (Ordering::Greater | Ordering::Less) = self.kind.cmp(&other.kind) {
            return ord;
        }

        if let ord @ (Ordering::Greater | Ordering::Less) = self.count.cmp(&other.count) {
            return ord;
        }

        self.proximity.cmp(&other.proximity)
    }
}

impl PartialOrd for Fact {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for Fact {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.count.hash(state);
        self.proximity.hash(state);
    }
}

impl Fact {
    fn new(
        kind: Constraint,
        count: usize,
        proximity: BTreeSet<Location>,
        iteration: usize,
        debug: FactDebug,
    ) -> Self {
        Self {
            kind,
            count,
            proximity,
            iteration,
            debug,
        }
    }

    fn seeded<L>(count: usize, proximity: BTreeSet<Location>, base_location: L) -> Self
    where
        L: Into<Option<Location>>,
    {
        Self {
            kind: Constraint::Exact,
            count,
            proximity,
            iteration: 0,
            debug: FactDebug::base(base_location, &Seeder),
        }
    }

    fn derive_kind(
        &self,
        kind: Constraint,
        iteration: usize,
        produced_by: &dyn Rule,
        parent_fact: &Fact,
    ) -> Self {
        Self {
            kind,
            count: self.count,
            proximity: self.proximity.clone(),
            iteration,
            debug: FactDebug::derived_one(produced_by, parent_fact),
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

    fn serialize(&self) -> String {
        let proximity = {
            let mut iter = self.proximity.iter();
            let first = iter
                .next()
                .map(ToString::to_string)
                .unwrap_or_else(Default::default);
            iter.fold(first, |mut elements, x| {
                elements.push(',');
                elements.push_str(&x.to_string());
                elements
            })
        };

        let default = format!(
            "{};{};{};{};{};{}",
            self.debug.base_location.unwrap_or(Location::INVALID),
            self.debug.produced_by,
            self.iteration,
            self.kind,
            self.count,
            proximity,
        );

        #[cfg(feature = "derived_from")]
        {
            return format!("{};{:?}", default, self.debug.derived_from);
        }

        default
    }
}

#[derive(Debug)]
struct Solver<'mf> {
    facts: HashSet<Fact>,
    iteration: usize,
    rules: Vec<Box<dyn Rule>>,
    #[debug(skip)]
    mine_field: &'mf Minefield,
}

impl<'mf> Solver<'mf> {
    fn new(mine_field: &'mf Minefield) -> Self {
        Self {
            facts: HashSet::new(),
            iteration: 0,
            rules: Vec::new(),
            mine_field,
        }
    }

    fn seed_universal_fact(&mut self) {
        let mut universal_fact = Fact::seeded(
            self.mine_field.mine_count(),
            self.mine_field
                .fog()
                .loc_iter()
                .filter(|(_, s)| s.is_hidden() || s.is_marked())
                .map(|(l, _)| l)
                .collect(),
            None,
        );
        universal_fact.iteration = self.iteration;
        self.facts.insert(universal_fact);
    }

    fn seed(&mut self) {
        let fog = self.mine_field.fog();
        let make_proximity = |l: Location| {
            l.neighbours()
                .filter(|&l| fog.get(l).map(State::is_hidden).unwrap_or(false))
                .collect()
        };

        self.facts.extend(
            fog.loc_iter()
                .filter_map(|(l, s)| Some((l, *s.as_revealed()?)))
                .map(|(l, s)| Fact::seeded(s, make_proximity(l), l)),
        );
    }

    fn seed_rules(&mut self) {
        self.rules.push(Box::new(MinAllToExact));
        self.rules.push(Box::new(MaxZeroToExact));
        self.rules.push(Box::new(ExactToMin));
        self.rules.push(Box::new(ExactToMax));
        self.rules.push(Box::new(MinWithinMaxCombinator));
        self.rules.push(Box::new(MaxIntersectsMinCombinator));
    }

    fn iter(&self) -> impl Iterator<Item = &Fact> {
        self.facts.iter()
    }

    fn iter_previous_iteration(&self) -> impl Iterator<Item = &Fact> {
        let previous_iteration = self.iteration - 1;
        self.facts
            .iter()
            .filter(move |f| f.iteration == previous_iteration)
    }

    fn iter_new_with_old(&self) -> impl Iterator<Item = (&Fact, &Fact)> {
        self.iter_previous_iteration()
            .flat_map(move |l| self.facts.iter().map(move |r| (l, r)))
    }

    fn add<I: IntoIterator<Item = Fact>>(&mut self, container: I) -> bool {
        let count = self.facts.len();
        self.facts.extend(container);
        self.facts.len() > count
    }

    fn guaranteed_safe_locations(&self) -> HashSet<Location> {
        self.facts
            .iter()
            .filter(|f| f.is_exact() && f.count == 0)
            .flat_map(|f| f.proximity.iter().copied())
            .collect()
    }

    fn guaranteed_mines(&self) -> HashSet<Location> {
        self.facts
            .iter()
            .filter(|f| f.is_exact() && f.count == f.proximity.len())
            .flat_map(|f| f.proximity.iter().copied())
            .collect()
    }

    fn solve(mf: &Minefield) -> (HashSet<Location>, HashSet<Location>) {
        Solver::solve_dump(mf, None)
    }

    fn run(&mut self) {
        let mut repeat = true;
        while repeat {
            self.print_fact_stats();
            self.iteration += 1;
            #[allow(clippy::needless_collect)]
            // false positive, cannot remove collect or exclusive borrow overlaps shared borrow of self
            let new_facts: Vec<_> = self.rules.iter().map(|r| r.derive(self)).collect();
            repeat = self.add(new_facts.into_iter().flatten());
        }
    }

    fn solve_dump(
        mf: &Minefield,
        dump_path: Option<&Path>,
    ) -> (HashSet<Location>, HashSet<Location>) {
        let mut solver = Solver::new(mf);
        solver.seed_rules();
        solver.seed();
        println!("Base Facts: {:#?}", solver);

        solver.run();

        let remaining_mines = solver.mine_field.mine_count() - solver.guaranteed_mines().len();
        if solver.mine_field.unobserved_count() <= remaining_mines {
            // skip one iteration to mark adding a fact
            solver.seed_universal_fact();
            solver.run();
        }

        println!("Final Facts: {:#?}", solver);
        if let Some(path) = dump_path {
            solver.dump(path).expect("Failed to dump facts to file.");
        }

        let safe_locations = solver.guaranteed_safe_locations();
        let mines = solver.guaranteed_mines();
        (safe_locations, mines)
    }

    fn print_fact_stats(&self) {
        use std::collections::HashMap;
        println!(
            "{} facts after iteration {}.",
            self.facts.len(),
            self.iteration
        );

        let facts_per_rule: HashMap<&str, usize> =
            self.facts
                .iter()
                .fold(HashMap::new(), |mut facts_per_rule, fact| {
                    *facts_per_rule.entry(fact.debug.produced_by).or_default() += 1;
                    facts_per_rule
                });

        println!("{:#?}", facts_per_rule);
    }

    fn dump(&self, path: &Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut writer = LineWriter::new(file);
        writeln!(
            writer,
            "base location;produced by;iteration;kind;count;proximity;predecessors"
        )?;
        for fact in &self.facts {
            let line = fact.serialize();
            writeln!(writer, "{}", line)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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
        let mut repo = Solver::new(&mf);
        repo.seed();
        repo.seed_universal_fact();

        let expected = vec![
            // row 0
            fact((0, 0), 1, [(0, 1)]),
            fact((1, 0), 3, [(2, 0), (2, 1), (0, 1)]),
            fact((3, 0), 3, [(2, 0), (4, 0), (2, 1)]),
            fact((5, 0), 2, [(4, 0), (6, 1)]),
            fact((6, 0), 1, [(6, 1)]),
            // duplicate fact((7, 0), 1, [(6, 1)]),
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
            // duplicate fact((6, 2), 1, [(6, 1), (7, 2)]),
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
            // all locations
            fact(
                (0, 0),
                15,
                [
                    (0, 1),
                    (0, 2),
                    (0, 3),
                    (0, 4),
                    (1, 3),
                    (1, 4),
                    (1, 5),
                    (2, 0),
                    (2, 1),
                    (2, 2),
                    (2, 3),
                    (2, 4),
                    (3, 3),
                    (3, 4),
                    (3, 5),
                    (4, 0),
                    (4, 2),
                    (5, 4),
                    (6, 1),
                    (6, 4),
                    (7, 2),
                    (7, 5),
                ],
            ),
        ];

        let actual = repo.facts.into_iter().collect();
        check_facts(expected, actual);
    }

    #[test]
    fn one_fact_mine_deduction() {
        let grid = "m1";
        let mf = Minefield::new_active_game(&grid);

        let (safe, mine) = Solver::solve_dump(&mf, dump_facts_path().as_deref());

        assert_eq!(locations([(0, 0)]), mine);
        assert_eq!(locations([]), safe);
    }

    #[test]
    fn two_fact_safe_deduction() {
        let grid = "m1
                         e1
                         ee";
        let mf = Minefield::new_active_game(&grid);

        let (safe, mine) = Solver::solve_dump(&mf, dump_facts_path().as_deref());

        assert_eq!(locations([]), mine);
        assert_eq!(locations([(0, 2), (1, 2)]), safe);
    }

    #[test]
    fn two_fact_mine_and_safe_deduction() {
        let grid = "mmeee
                         2211m";
        let mf = Minefield::new_active_game(&grid);

        let (safe, mine) = Solver::solve_dump(&mf, dump_facts_path().as_deref());

        assert_eq!(locations([(0, 0), (1, 0)]), mine);
        assert_eq!(locations([(2, 0), (3, 0)]), safe);
    }

    #[test]
    fn cross_deduction() {
        let grid = "eeeee
                         em1ee
                         e111m";
        let mf = Minefield::new_active_game(&grid);

        let (safe, mine) = Solver::solve_dump(&mf, dump_facts_path().as_deref());

        assert_eq!(locations([]), mine);
        assert_eq!(locations([(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]), safe);
    }

    #[test]
    fn corner_deduct_case_1() {
        let grid = "12m1
                         em32
                         ee2m";
        let mf = Minefield::new_active_game(&grid);

        let (safe, mine) = Solver::solve_dump(&mf, dump_facts_path().as_deref());

        assert_eq!(locations([(2, 0), (1, 1), (3, 2)]), mine);
        assert_eq!(locations([(0, 1), (0, 2), (1, 2)]), safe);
    }

    #[test]
    fn corner_deduct_case_2() {
        let grid = "12m1
                         me32
                         em2m";
        let mf = Minefield::new_active_game(&grid);

        let (safe, mine) = Solver::solve_dump(&mf, dump_facts_path().as_deref());

        assert_eq!(locations([(2, 0), (3, 2)]), mine);
        assert_eq!(locations([]), safe);
    }

    #[test]
    fn corner_deduct_case_3() {
        let grid = "12m1
                         me32
                         mm2m";
        let mf = Minefield::new_active_game(&grid);

        let (safe, mine) = Solver::solve_dump(&mf, dump_facts_path().as_deref());

        assert_eq!(locations([(2, 0), (0, 1), (0, 2), (1, 2), (3, 2)]), mine);
        assert_eq!(locations([(1, 1)]), safe);
    }

    /// Test case from a generated minefield.
    /// ![complex_example][complex_example]
    /// [complex_example]: pics/complex-example-with-coords.png
    #[test]
    fn real_example_1() {
        let grid = "meeeeeeeem100000000001em
                         eeemeeeeee111101110113me
                         emeemmemeeeem101m212meme
                         emeee3212m11110113meeeem
                         meeem1001110000002meeeee
                         eeme21000000001122eeeemm
                         eemm10000000001memeeeeem";
        let mf = Minefield::new_active_game(&grid);

        let (safe, mine) = Solver::solve_dump(&mf, dump_facts_path().as_deref());

        assert_eq!(
            locations([
                (4, 2),
                (5, 2),
                (7, 2),
                (12, 2),
                (16, 2),
                (20, 2),
                (22, 2),
                (9, 3),
                (18, 3),
                (4, 4),
                (18, 4),
                (15, 6),
                (17, 6),
            ]),
            mine
        );
        assert_eq!(
            locations([
                (6, 2),
                (8, 2),
                (9, 2),
                (10, 2),
                (11, 2),
                (21, 2),
                (4, 3),
                (19, 3),
                (20, 3),
                (3, 4),
                (18, 5),
                (16, 6),
                (18, 6)
            ]),
            safe
        );
    }

    fn locations<const N: usize>(ls: [(usize, usize); N]) -> HashSet<Location> {
        std::array::IntoIter::new(ls).map(Into::into).collect()
    }

    fn fact<const N: usize>(
        _: (usize, usize),
        mine_count: usize,
        proximity: [(usize, usize); N],
    ) -> Fact {
        let proximity = std::array::IntoIter::new(proximity)
            .map(Into::into)
            .collect();
        Fact::new(
            Constraint::Exact,
            mine_count,
            proximity,
            0,
            FactDebug::base(Location::INVALID, &Seeder),
        )
    }

    fn check_facts(mut expected: Vec<Fact>, mut actual: Vec<Fact>) {
        expected.sort_unstable();
        actual.sort_unstable();
        println!("Expected: {:?}", expected);
        println!("Actual: {:?}", actual);
        assert_eq!(expected.len(), actual.len(), "Different number of facts!");
        for (e, a) in expected.into_iter().zip(actual.into_iter()) {
            assert_eq!(e, a);
        }
    }

    fn dump_facts_path() -> Option<PathBuf> {
        std::env::var("DUMP_FACTS").ok().map(Into::into)
    }
}
