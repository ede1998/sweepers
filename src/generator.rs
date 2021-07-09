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

pub struct DummyGenerator;

impl MinefieldGenerator for DummyGenerator {
    fn generate(&mut self, _params: Parameters, _not_a_mine: Location) -> Area<GroundKind> {
        unreachable!()
    }
}
