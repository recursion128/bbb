use super::*;

// NaturalSpawner.java:170: at most three groups per category and position.
const GROUP_ATTEMPTS: usize = 3;

impl NaturalSpawner<'_> {
    pub(super) fn spawn_category_for_position(
        &mut self,
        category: VanillaEntityMobCategory,
        start: BlockPos,
        random: &GameRandom,
    ) -> Result<(), ()> {
        if !self.initial_position_allows_spawning(start) {
            return Ok(());
        }

        let mut cluster_size: i32 = 0;
        for _ in 0..GROUP_ATTEMPTS {
            let mut x = start.x;
            let mut z = start.z;
            let mut current_spawn_data = None;
            let mut group_data = None;
            let mut max = (random.next_float() * 4.0).ceil() as i32;
            let mut attempt = 0;

            while attempt < max {
                x = x
                    .wrapping_add(random.next_int(6))
                    .wrapping_sub(random.next_int(6));
                z = z
                    .wrapping_add(random.next_int(6))
                    .wrapping_sub(random.next_int(6));
                let position = BlockPos { x, y: start.y, z };

                if self.position_is_eligible(category, position) {
                    if current_spawn_data.is_none() {
                        let Some(selection) = self.select_spawn_data(category, position, random)
                        else {
                            break;
                        };
                        if selection.min_count < 1 || selection.max_count < selection.min_count {
                            return Err(());
                        }
                        let range = selection
                            .max_count
                            .checked_sub(selection.min_count)
                            .and_then(|difference| difference.checked_add(1))
                            .ok_or(())?;
                        max = selection
                            .min_count
                            .checked_add(random.next_int(range))
                            .ok_or(())?;
                        current_spawn_data = Some(selection);
                    }

                    let spawn_data = current_spawn_data
                        .as_ref()
                        .expect("eligible candidates retain their selected spawn data");
                    if self.is_valid_spawn_data(category, spawn_data, position, random) {
                        let Some(mob) = self.create_mob(spawn_data) else {
                            return Ok(());
                        };
                        let y_rot = random.next_float() * 360.0;
                        match self.finish_mob(mob, position, y_rot, group_data.as_ref(), random) {
                            None => {}
                            Some(published) => {
                                group_data = published.group_data;
                                cluster_size = cluster_size.wrapping_add(1);
                                if cluster_size >= published.max_spawn_cluster_size {
                                    return Ok(());
                                }
                                if published.max_group_size_reached {
                                    break;
                                }
                            }
                        }
                    }
                }
                attempt += 1;
            }
        }

        Ok(())
    }
}
