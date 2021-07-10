use std::collections::BTreeSet;

use rand::seq::index::sample as rand_sample;

use crate::core::*;

pub struct SimpleGenerator;

impl MinefieldGenerator for SimpleGenerator {
    fn generate(&mut self, params: Parameters, not_a_mine: Location) -> Area<GroundKind> {
        let Parameters {
            width,
            height,
            mine_count,
        } = params;
        loop {
            let mut a = Area::new(width, height);
            let result = rand_sample(&mut rand::thread_rng(), width * height, mine_count);
            for index in result {
                let mine_location = Location::from_index(index, width);
                a[mine_location] = GroundKind::Mine;
            }

            let is_zero = not_a_mine
                .neighbours()
                .filter_map(|l| a.get(l))
                .all(|g| g.is_dirt());
            let is_ground = a.get(not_a_mine).unwrap_or(&GroundKind::Dirt).is_dirt();
            if is_ground && is_zero {
                break a;
            }
        }
    }
}

pub struct ImprovedGenerator;

impl ImprovedGenerator {
    fn build_safe_location_skipper(not_a_mine: Location, width: usize) -> impl Fn(usize) -> usize {
        let safe_indices: BTreeSet<_> = {
            let mut safe_area = vec![not_a_mine];
            safe_area.extend(not_a_mine.neighbours());
            safe_area
                .into_iter()
                .filter_map(|l| l.to_index(width))
                .collect()
        };
        eprintln!("bomb_free_indices: {:?}", safe_indices);

        move |index| {
            let mut adjusted_index = index;
            loop {
                // count number of skipped safe indices
                let adjustment = safe_indices
                    .iter()
                    .position(|&p| adjusted_index < p)
                    .unwrap_or_else(|| safe_indices.len());

                // if index didn't get a new adjustment, we are done
                match index + adjustment == adjusted_index {
                    true => {
                        eprintln!("original index: {}, adjustment: {}", index, adjustment);
                        break adjusted_index;
                    }
                    false => adjusted_index = index + adjustment,
                }
            }
        }
    }
}

impl MinefieldGenerator for ImprovedGenerator {
    fn generate(&mut self, params: Parameters, not_a_mine: Location) -> Area<GroundKind> {
        let Parameters {
            width,
            height,
            mine_count,
        } = params;
        let mut a = Area::new(width, height);
        const MIN_DIFFERENCE_FOR_SAFE_AREA: usize = 9;
        let mut result = rand_sample(
            &mut rand::thread_rng(),
            width * height - MIN_DIFFERENCE_FOR_SAFE_AREA,
            mine_count,
        )
        .into_vec();
        result.sort_unstable();

        let skip_safe_indices = Self::build_safe_location_skipper(not_a_mine, width);
        for index in result {
            let adjusted_index = skip_safe_indices(index);
            let mine_location = Location::from_index(adjusted_index, width);
            a[mine_location] = GroundKind::Mine;
        }
        a
    }
}

pub struct DummyGenerator;

impl MinefieldGenerator for DummyGenerator {
    fn generate(&mut self, _params: Parameters, _not_a_mine: Location) -> Area<GroundKind> {
        unreachable!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_skipper_maps_indices_correctly() {
        // Original indices:                Original mine locations:
        //  00 01 02 03 04 05 06 07 08 09    __ __ __ 03 __ __ __ __ __ __
        //  10 11 12 13 14+15-16-17+18 19    __ __ __ __ 14+15-__-__+__ __
        //  20 21 22 23 24|25 26 27|28 29    20 __ __ __ __|__ __ __|__ 29
        //  30 31 32 33 34+35-36-37+38 39    __ __ __ 33 __+__-__-__+__ 39
        //  40 41 42 43 44 45 46 47 48 49    __ __ __ __ __ __ __ __ __ __
        // Adjusted indices:                Adjusted mine locations:
        //  00 01 02 03 04 05 06 07 08 09    __ __ __ 03 __ __ __ __ __ __
        //  10 11 12 13 14+--------+15 16    __ __ __ __ 14+--------+15 __
        //  17 18 19 20 21|        |22 23    __ __ __ 20 __|        |__ __
        //  24 25 26 27 28+--------+29 30    __ __ __ __ __+--------+29 __
        //  31 32 33 34 35 36 37 38 39 40    __ __ 33 __ __ __ __ __ 39 __
        let width = 10;
        let not_a_mine = Location::from_index(26, width);
        let skipper = ImprovedGenerator::build_safe_location_skipper(not_a_mine, width);

        let check = |input, expected_result, msg: &str| {
            assert_eq!(skipper(input), expected_result, "{}", msg);
        };
        check(03, 03, "Invalid adjustment before any safe location.");
        check(14, 14, "Invalid adjustment before 1st safe location.");
        check(15, 18, "Invalid adjustment at 1st safe location.");
        check(20, 23, "Invalid adjustment after 1st safe location block.");
        check(29, 38, "Invalid adjustment after 2nd safe location block.");
        check(33, 42, "Invalid adjustment before 3rd safe location block.");
        check(39, 48, "Invalid adjustment after 3rd safe location block.");
    }
}
