//! A villager's day outside trading: its bed, its family, its farm, the
//! golem it calls, and what scares it (`VillagerGoalPackages.java`).

use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_IRON_GOLEM_ID as IRON_GOLEM, VANILLA_ENTITY_TYPE_VILLAGER_ID as VILLAGER,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID as ZOMBIE,
};

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

mod hero;
mod idle;
mod play;
mod reputation;

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

struct Village {
    world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    /// A floor cell three blocks east of the joined player.
    origin: BlockPos,
    feet_y: f64,
    sequence: u64,
}

impl Village {
    fn new() -> Self {
        let mut config = datapack_config();
        config.game_type = GameType::Creative;
        Self::with_config(config)
    }

    fn with_config(config: ServerWorldConfig) -> Self {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "mayor", 0x7111);
        let position = world.publication().players[0].position;
        let mut village = Self {
            world,
            sender,
            receiver,
            origin: BlockPos {
                x: position.x.floor() as i32 + 3,
                y: position.y.floor() as i32,
                z: position.z.floor() as i32,
            },
            feet_y: position.y,
            sequence: 2,
        };
        // No zombie wanders in to cut a test village's night short.
        village.command("gamerule spawn_mobs false");
        village
    }

    fn command(&mut self, command: &str) {
        self.sequence += 1;
        console_command(
            &self.sender,
            &self.receiver,
            &mut self.world,
            self.sequence,
            command,
        );
    }

    fn at(&self, dx: i32, dz: i32) -> BlockPos {
        BlockPos {
            x: self.origin.x + dx,
            y: self.origin.y,
            z: self.origin.z + dz,
        }
    }

    fn set_block(&mut self, pos: BlockPos, state: &str) {
        self.command(&format!("setblock {} {} {} {state}", pos.x, pos.y, pos.z));
    }

    /// A bed whose foot is at `foot` and whose head is one block east.
    fn bed(&mut self, foot: BlockPos) -> BlockPos {
        self.set_block(foot, "minecraft:red_bed[part=foot,facing=east]");
        let head = BlockPos {
            x: foot.x + 1,
            ..foot
        };
        self.set_block(head, "minecraft:red_bed[part=head,facing=east]");
        head
    }

    fn villager(&mut self, pos: BlockPos) -> i32 {
        self.mob(VILLAGER, pos)
    }

    /// Replaces the villager's inventory with `items` through its saved record.
    fn give(&mut self, villager: i32, items: &[(&str, i32)]) {
        let NbtSummaryValue::Compound(mut entries) = self
            .world
            .stored_entity_command_data_for_test(OVERWORLD, villager)
            .expect("the villager has a record")
        else {
            panic!("an entity record is a compound")
        };
        entries.retain(|entry| entry.name != "Inventory");
        let stack = |(id, count): &(&str, i32)| {
            NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::String((*id).into()),
                },
                NbtSummaryEntry {
                    name: "count".into(),
                    value: NbtSummaryValue::Int(*count),
                },
            ])
        };
        entries.push(NbtSummaryEntry {
            name: "Inventory".into(),
            value: NbtSummaryValue::List(items.iter().map(stack).collect()),
        });
        self.world
            .load_stored_entity_record_for_test(OVERWORLD, villager, &entries);
    }

    fn count_item(&self, villager: i32, item: &str) -> i32 {
        self.entity(villager).map_or(0, |entity| {
            crate::entity::villager_life::count_item(entity, item)
        })
    }

    fn villagers(&self) -> Vec<i32> {
        self.world
            .stored_entity_ids_for_test(OVERWORLD)
            .into_iter()
            .filter(|id| {
                self.entity(*id)
                    .is_some_and(|entity| entity.entity_type_id() == VILLAGER)
            })
            .collect()
    }

    fn entity(&self, id: i32) -> Option<&crate::entity::Entity> {
        self.world.stored_entity_for_test(OVERWORLD, id)
    }

    fn tick_until(&mut self, limit: usize, mut done: impl FnMut(&mut Self) -> bool) -> bool {
        for _ in 0..limit {
            let tick = self.world.tick(self.receiver.drain());
            // The joined player answers its keep-alives, so a long wait does
            // not time it out and unload the village.
            for event in &tick.events {
                if let WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::KeepAlive { id },
                } = event
                {
                    enqueue(
                        &self.sender,
                        WorldCommand::Play {
                            connection_id: *connection_id,
                            packet: PlayServerbound::KeepAlive { id: *id },
                        },
                    )
                    .unwrap();
                }
            }
            if done(self) {
                return true;
            }
        }
        false
    }

    fn mob(&mut self, entity_type_id: i32, pos: BlockPos) -> i32 {
        let at = self.center(pos);
        self.world
            .spawn_stored_entity_for_test(
                OVERWORLD,
                entity_type_id,
                at,
                EntityData::GeneralLiving(GeneralLivingData::new(entity_type_id).unwrap()),
            )
            .0
    }

    /// Merges `entries` into the mob's saved record, the way `/data merge
    /// entity` does.
    fn merge(&mut self, entity: i32, entries: Vec<NbtSummaryEntry>) {
        let NbtSummaryValue::Compound(mut record) = self
            .world
            .stored_entity_command_data_for_test(OVERWORLD, entity)
            .expect("the mob has a record")
        else {
            panic!("an entity record is a compound")
        };
        record.retain(|entry| !entries.iter().any(|new| new.name == entry.name));
        record.extend(entries);
        self.world
            .load_stored_entity_record_for_test(OVERWORLD, entity, &record);
    }

    /// A glass pen around the five-by-five floor `at(0..=4, -2..=2)`.
    fn pen(&mut self) {
        let (low, high) = (self.at(-1, -3), self.at(5, 3));
        let (inner_low, inner_high) = (self.at(0, -2), self.at(4, 2));
        self.command(&format!(
            "fill {} {} {} {} {} {} minecraft:glass",
            low.x,
            low.y,
            low.z,
            high.x,
            high.y + 1,
            high.z
        ));
        self.command(&format!(
            "fill {} {} {} {} {} {} minecraft:air",
            inner_low.x,
            inner_low.y,
            inner_low.z,
            inner_high.x,
            inner_high.y + 1,
            inner_high.z
        ));
    }

    fn center(&self, pos: BlockPos) -> Vec3d {
        Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: self.feet_y,
            z: f64::from(pos.z) + 0.5,
        }
    }

    fn count_of(&self, entity_type_id: i32) -> usize {
        self.world
            .stored_entity_ids_for_test(OVERWORLD)
            .into_iter()
            .filter(|id| {
                self.entity(*id)
                    .is_some_and(|entity| entity.entity_type_id() == entity_type_id)
            })
            .count()
    }

    fn distance(&self, entity: i32, to: Vec3d) -> f64 {
        self.entity(entity).map_or(f64::MAX, |entity| {
            let position = entity.position();
            ((position.x - to.x).powi(2) + (position.z - to.z).powi(2)).sqrt()
        })
    }

    fn bed_occupied(&mut self, pos: BlockPos) -> Option<bool> {
        bbb_server_types::block_queries::bool_property(
            self.world.block_state(OVERWORLD, pos)?,
            "occupied",
        )
    }

    fn sleeping_in(&mut self, villager: i32, bed: BlockPos) -> bool {
        self.entity(villager).is_some_and(|entity| {
            entity.sleeping_pos() == Some(bed)
                && entity.pose_id() == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
        }) && self.bed_occupied(bed) == Some(true)
    }
}

#[test]
fn villagers_sleep_in_their_bed_at_night_and_a_dead_villagers_bed_is_claimed_again() {
    let mut village = Village::new();
    village.command("time set midnight");
    let bed = village.bed(village.at(2, 0));
    let sleeper = village.villager(village.at(0, 0));

    // At night the REST package walks the villager to the bed it claimed and
    // lays it down there: the sleeping pose and an occupied bed.
    assert!(village.tick_until(1_200, |village| village.sleeping_in(sleeper, bed)));

    // In the morning WakeUp gets it out of bed.
    village.command("time set day");
    assert!(village.tick_until(100, |village| {
        village.entity(sleeper).is_some_and(|entity| {
            entity.sleeping_pos().is_none()
                && entity.pose_id() != bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
        })
    }));
    assert_eq!(village.bed_occupied(bed), Some(false));

    // Its death gives the bed back, so the next villager claims and uses it.
    village.command("kill @e[type=minecraft:villager]");
    assert!(village.tick_until(40, |village| village.entity(sleeper).is_none()));
    let heir = village.villager(village.at(0, 2));
    village.command("time set midnight");
    assert!(village.tick_until(1_200, |village| village.sleeping_in(heir, bed)));
}

/// `Villager.stopSleeping` (`Villager.java:891-894`) records `LAST_WOKEN`
/// whatever wakes it, here damage (`LivingEntity.hurtServer`).
#[test]
fn a_villager_woken_by_damage_remembers_when_it_woke() {
    let mut village = Village::new();
    village.command("time set midnight");
    let bed = village.bed(village.at(2, 0));
    let sleeper = village.villager(village.at(0, 0));
    assert!(village.tick_until(1_200, |village| village.sleeping_in(sleeper, bed)));
    let remembers = |village: &Village| {
        let record = village
            .world
            .stored_entity_command_data_for_test(OVERWORLD, sleeper)
            .expect("the villager has a record");
        format!("{record:?}").contains("minecraft:last_woken")
    };
    assert!(!remembers(&village));

    village.command("damage @e[type=minecraft:villager,limit=1] 1");
    assert!(village.entity(sleeper).unwrap().sleeping_pos().is_none());
    assert!(remembers(&village));
}

#[test]
fn a_villager_that_loses_its_job_closes_the_open_merchant_menu() {
    let mut village = Village::new();
    village.command("time set noon");
    let lectern = village.at(1, 0);
    village.set_block(lectern, "minecraft:lectern");
    let librarian = village.villager(village.at(0, 0));
    assert!(village.tick_until(600, |village| {
        village.entity(librarian).is_some_and(|entity| {
            matches!(entity.data(), EntityData::GeneralLiving(data)
                if data.villager_profession_and_level()
                    .is_some_and(|(profession, _)| profession == "minecraft:librarian"))
        })
    }));
    play(
        &village.sender,
        &village.receiver,
        &mut village.world,
        1,
        super::feeding::interact_entity_packet(librarian),
    );
    let menu_open = |village: &mut Village| {
        village
            .world
            .player_inventory_for_test(ConnectionId(1))
            .is_merchant_menu()
    };
    assert!(menu_open(&mut village));

    // Without the lectern the librarian that never traded is unemployed again
    // and stops trading, which closes the menu.
    village.set_block(lectern, "minecraft:air");
    assert!(village.tick_until(200, |village| !menu_open(village)));
}

/// AbstractVillager.java:192-195: a teleport stops trading, which closes the
/// open menu even when the villager lands where it stood.
#[test]
fn teleporting_a_trading_villager_closes_the_merchant_menu() {
    let mut village = Village::new();
    village.command("time set noon");
    let lectern = village.at(1, 0);
    village.set_block(lectern, "minecraft:lectern");
    let stand = village.at(0, 0);
    let librarian = village.villager(stand);
    assert!(village.tick_until(600, |village| {
        village.entity(librarian).is_some_and(|entity| {
            matches!(entity.data(), EntityData::GeneralLiving(data)
                if data.villager_profession_and_level()
                    .is_some_and(|(profession, _)| profession == "minecraft:librarian"))
        })
    }));
    village.command("data merge entity @e[type=minecraft:villager,limit=1] {NoAI:1b}");
    play(
        &village.sender,
        &village.receiver,
        &mut village.world,
        1,
        super::feeding::interact_entity_packet(librarian),
    );
    let menu_open = |village: &mut Village| {
        village
            .world
            .player_inventory_for_test(ConnectionId(1))
            .is_merchant_menu()
    };
    assert!(menu_open(&mut village));
    let position = village.entity(librarian).unwrap().position();
    village.command(&format!(
        "tp @e[type=minecraft:villager,limit=1] {} {} {}",
        position.x, position.y, position.z
    ));
    assert!(village.tick_until(2, |village| !menu_open(village)));
    assert_eq!(
        village.entity(librarian).unwrap().merchant_trading_player(),
        None
    );
}

#[test]
fn two_fed_villagers_with_free_beds_have_a_baby_that_takes_a_bed() {
    let mut village = Village::new();
    village.command("time set noon");
    let from = village.at(-8, -16);
    let to = village.at(47, 15);
    village.command(&format!(
        "forceload add {} {} {} {}",
        from.x, from.z, to.x, to.z
    ));
    village.command(&format!(
        "fillbiome {} {} {} {} {} {} minecraft:swamp",
        from.x,
        from.y - 8,
        from.z,
        to.x,
        to.y + 8,
        to.z
    ));
    let mut beds: Vec<BlockPos> = [-5, 5]
        .into_iter()
        .map(|dz| village.bed(village.at(4, dz)))
        .collect();
    let sealed = village.bed(village.at(4, 0));
    for dx in -2_i32..=2 {
        for dz in -2_i32..=2 {
            for dy in 0..=2 {
                if dx.abs() == 2 || dz.abs() == 2 || dy == 2 {
                    village.set_block(
                        BlockPos {
                            x: sealed.x + dx,
                            y: sealed.y + dy,
                            z: sealed.z + dz,
                        },
                        "minecraft:stone",
                    );
                }
            }
        }
    }
    let distant = village.bed(village.at(32, 0));
    beds.push(distant);
    let mother = village.villager(village.at(0, 0));
    let father = village.villager(village.at(1, 1));
    for parent in [mother, father] {
        village.merge(
            parent,
            vec![entry(
                "VillagerData",
                NbtSummaryValue::Compound(vec![entry(
                    "type",
                    NbtSummaryValue::String("minecraft:swamp".into()),
                )]),
            )],
        );
    }
    // Three bread are the twelve food points `canBreed` asks of each.
    village.give(mother, &[("minecraft:bread", 3)]);
    village.give(father, &[("minecraft:bread", 3)]);

    assert!(village.tick_until(4_000, |village| village.villagers().len() == 3));
    let baby = *village
        .villagers()
        .iter()
        .find(|id| ![mother, father].contains(id))
        .unwrap();
    assert!(village.entity(baby).is_some_and(|entity| matches!(entity.data(),
        EntityData::GeneralLiving(data) if data.ageable_retained().is_some_and(|age| age.is_baby()))));
    let saved = village
        .world
        .stored_entity_command_data_for_test(OVERWORLD, baby)
        .unwrap();
    let home = ["Brain", "memories", "minecraft:home", "value", "pos"]
        .into_iter()
        .try_fold(&saved, |value, name| match value {
            NbtSummaryValue::Compound(entries) => entries
                .iter()
                .find(|entry| entry.name == name)
                .map(|entry| &entry.value),
            _ => None,
        })
        .expect("the newborn saves its home position");
    assert_eq!(
        home,
        &NbtSummaryValue::IntArray(vec![distant.x, distant.y, distant.z]),
        "the newborn must be assigned the remaining reachable bed"
    );
    let child_type =
        ["VillagerData", "type"]
            .into_iter()
            .try_fold(&saved, |value, name| match value {
                NbtSummaryValue::Compound(entries) => entries
                    .iter()
                    .find(|entry| entry.name == name)
                    .map(|entry| &entry.value),
                _ => None,
            });
    assert_eq!(
        child_type,
        Some(&NbtSummaryValue::String("minecraft:swamp".into()))
    );
    // Both parents ate their bread.
    assert_eq!(village.count_item(mother, "minecraft:bread"), 0);
    assert_eq!(village.count_item(father, "minecraft:bread"), 0);

    // The baby was given a bed of its own and sleeps there at night.
    village.command("time set midnight");
    assert!(village.tick_until(1_500, |village| {
        beds.iter().any(|bed| village.sleeping_in(baby, *bed))
    }));
}

#[test]
fn a_farmer_harvests_ripe_wheat_replants_it_and_bakes_bread_at_its_composter() {
    let mut village = Village::new();
    village.command("gamerule random_tick_speed 0");
    village.command("gamerule block_drops false");
    village.command("time set 3000");
    let composter = village.at(6, -2);
    village.set_block(composter, "minecraft:composter");
    let crop = village.at(2, 0);
    village.set_block(
        BlockPos {
            y: crop.y - 1,
            ..crop
        },
        "minecraft:farmland",
    );
    village.set_block(crop, "minecraft:wheat[age=7]");
    let mut position = village.center(village.at(2, 0));
    position.x -= 0.3;
    let farmer = village
        .world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            VILLAGER,
            position,
            EntityData::GeneralLiving(GeneralLivingData::new(VILLAGER).unwrap()),
        )
        .0;
    village.give(
        farmer,
        &[("minecraft:wheat_seeds", 4), ("minecraft:wheat", 6)],
    );
    let job_site = NbtSummaryValue::Compound(vec![
        entry(
            "dimension",
            NbtSummaryValue::String("minecraft:overworld".into()),
        ),
        entry(
            "pos",
            NbtSummaryValue::IntArray(vec![composter.x, composter.y, composter.z]),
        ),
    ]);
    let memories = NbtSummaryValue::Compound(vec![entry(
        "minecraft:job_site",
        NbtSummaryValue::Compound(vec![entry("value", job_site)]),
    )]);
    village.merge(
        farmer,
        vec![
            entry(
                "VillagerData",
                NbtSummaryValue::Compound(vec![
                    entry("type", NbtSummaryValue::String("minecraft:plains".into())),
                    entry(
                        "profession",
                        NbtSummaryValue::String("minecraft:farmer".into()),
                    ),
                    entry("level", NbtSummaryValue::Int(2)),
                ]),
            ),
            entry(
                "Brain",
                NbtSummaryValue::Compound(vec![entry("memories", memories)]),
            ),
        ],
    );
    village.command(
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0",
    );

    let age = |village: &mut Village, pos: BlockPos| {
        village
            .world
            .block_state(OVERWORLD, pos)
            .filter(|state| crate::block_behavior::crop::is_crop(*state))
            .and_then(crate::block_behavior::crop::age)
    };
    // Keep the farmer by its crop and suppress drops so collecting an item
    // cannot imitate the farmer looking at the cell it harvests and replants.
    assert!(
        village.tick_until(3_000, |village| {
            village.entity(farmer).is_some_and(|entity| {
                let position = entity.position();
                let yaw = (f64::from(crop.z) + 0.5 - position.z)
                    .atan2(f64::from(crop.x) + 0.5 - position.x)
                    .to_degrees() as f32
                    - 90.0;
                crate::entity::living::wrap_degrees(entity.y_head_rot() - yaw).abs() < 10.0
                    && entity.x_rot() > 20.0
            }) && age(village, crop) == Some(0)
        }),
        "the farmer looks down toward the crop it harvests and replants"
    );
    village.command(
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0.5",
    );
    village.command("gamerule block_drops true");
    // The farmer works at its composter and turns three wheat into a bread.
    assert!(village.tick_until(3_000, |village| {
        village.count_item(farmer, "minecraft:bread") > 0
    }));
}

#[test]
fn a_villager_runs_from_a_zombie_that_comes_close() {
    let mut village = Village::new();
    village.command("time set midnight");
    let villager = village.villager(village.at(2, 0));
    let zombie_pos = village.at(6, 0);
    let zombie = village.mob(ZOMBIE, zombie_pos);
    village.merge(zombie, vec![entry("NoAI", NbtSummaryValue::Byte(1))]);
    let zombie_at = village.center(zombie_pos);

    // Four blocks from a zombie the villager panics and walks out of the
    // eight blocks that scare it.
    assert!(village.tick_until(200, |village| village.distance(villager, zombie_at) > 8.0));
}

#[test]
fn penned_villagers_that_slept_lately_call_one_iron_golem_against_a_zombie() {
    let mut village = Village::new();
    village.command("time set midnight");
    // A pen that keeps them within the zombie's eight blocks.
    village.pen();
    // Three villagers that slept a moment ago and have seen no golem.
    let slept = village.world.game_time();
    for row in 0..3 {
        let villager = village.villager(village.at(0, row - 1));
        let memory = NbtSummaryValue::Compound(vec![entry("value", NbtSummaryValue::Long(slept))]);
        let memories = NbtSummaryValue::Compound(vec![entry("minecraft:last_slept", memory)]);
        let brain = NbtSummaryValue::Compound(vec![entry("memories", memories)]);
        village.merge(villager, vec![entry("Brain", brain)]);
    }
    let zombie = village.mob(ZOMBIE, village.at(4, 2));
    village.merge(zombie, vec![entry("NoAI", NbtSummaryValue::Byte(1))]);

    // The panic's golem call finds three villagers that agree.
    assert!(village.tick_until(400, |village| village.count_of(IRON_GOLEM) == 1));
    // Having seen that golem, they call no second one.
    village.tick_until(600, |_| false);
    assert_eq!(village.count_of(IRON_GOLEM), 1);
}

#[test]
fn a_villager_that_hears_a_bell_hides_in_a_bed() {
    for redstone in [false, true] {
        let mut village = Village::new();
        village.command("time set noon");
        let bed = village.bed(village.at(12, 0));
        let villager = village.villager(village.at(1, 0));
        let bell = village.at(-2, 1);
        village.set_block(bell, "minecraft:bell[attachment=floor,facing=north]");
        let bed_center = village.center(bed);
        if redstone {
            village.set_block(
                BlockPos {
                    x: bell.x + 1,
                    ..bell
                },
                "minecraft:redstone_block",
            );
        } else {
            play(
                &village.sender,
                &village.receiver,
                &mut village.world,
                1,
                PlayServerbound::UseItemOn(UseItemOn {
                    hand: InteractionHand::MainHand,
                    hit: BlockHitResult {
                        pos: bell,
                        direction: Direction::North,
                        cursor_x: 0.5,
                        cursor_y: 0.5,
                        cursor_z: 0.0,
                        inside: false,
                        world_border_hit: false,
                    },
                    sequence: 1,
                }),
            );
        }

        // The bell sends it to the nearest home it can find, which is the bed.
        assert!(village.tick_until(300, |village| village.distance(villager, bed_center) < 3.0));
    }
}

fn gossip_entry(target: &[i32; 4], kind: &str, value: i32) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("Target", NbtSummaryValue::IntArray(target.to_vec())),
        entry("Type", NbtSummaryValue::String(kind.into())),
        entry("Value", NbtSummaryValue::Int(value)),
    ])
}

impl Village {
    /// The villager's saved gossips as `(type, value)`.
    fn gossips(&self, villager: i32) -> Vec<(String, i32)> {
        let Some(NbtSummaryValue::Compound(record)) = self
            .world
            .stored_entity_command_data_for_test(OVERWORLD, villager)
        else {
            return Vec::new();
        };
        let Some(NbtSummaryValue::List(gossips)) = record
            .into_iter()
            .rev()
            .find(|entry| entry.name == "Gossips")
            .map(|entry| entry.value)
        else {
            return Vec::new();
        };
        gossips
            .into_iter()
            .filter_map(|gossip| {
                let NbtSummaryValue::Compound(fields) = gossip else {
                    return None;
                };
                let field = |name: &str| fields.iter().find(|f| f.name == name).map(|f| &f.value);
                match (field("Type"), field("Value")) {
                    (Some(NbtSummaryValue::String(kind)), Some(NbtSummaryValue::Int(value))) => {
                        Some((kind.try_to_string().unwrap(), *value))
                    }
                    _ => None,
                }
            })
            .collect()
    }
}

#[test]
fn villagers_gather_at_the_bell_in_the_afternoon() {
    let mut village = Village::new();
    village.command("time set 9500");
    let bell = village.at(24, 0);
    village.set_block(bell, "minecraft:bell[attachment=floor,facing=north]");
    let villager = village.villager(village.at(0, 0));
    let bell_at = village.center(bell);

    // The MEET hours walk it to the meeting point it claimed.
    assert!(village.tick_until(600, |village| village.distance(villager, bell_at) < 7.0));
}

#[test]
fn villagers_that_meet_pass_on_gossip_and_gossip_fades_with_the_days() {
    let mut village = Village::new();
    village.command("time set 1000");
    village.pen();
    let player = [1, 2, 3, 4];
    let teller = village.villager(village.at(0, 0));
    let listener = village.villager(village.at(1, 1));
    // The teller stands still, so the listener is the one that walks up,
    // talks and takes the gossip.
    village.merge(
        teller,
        vec![
            entry(
                "Gossips",
                NbtSummaryValue::List(vec![gossip_entry(&player, "minor_positive", 25)]),
            ),
            entry("NoAI", NbtSummaryValue::Byte(1)),
        ],
    );

    // Talking villagers trade gossip at most every 1200 ticks: the listener
    // hears the teller's 25 less the five a retelling loses.
    assert!(village.tick_until(4_000, |village| {
        village.gossips(listener) == vec![("minor_positive".to_owned(), 20)]
    }));

    // A day after the last decay each gossip loses its daily amount, and one
    // that falls under two is forgotten.
    village.merge(
        teller,
        vec![
            entry(
                "Gossips",
                NbtSummaryValue::List(vec![
                    gossip_entry(&player, "trading", 10),
                    gossip_entry(&player, "minor_negative", 20),
                ]),
            ),
            entry("LastGossipDecay", NbtSummaryValue::Long(-30_000)),
        ],
    );
    village.tick_until(1, |_| false);
    assert_eq!(village.gossips(teller), vec![("trading".to_owned(), 8)]);
}

#[test]
fn a_baby_villager_shakes_its_head_at_a_player_for_two_seconds() {
    let mut village = Village::new();
    let baby = village.villager(village.at(0, 0));
    village.merge(baby, vec![entry("Age", NbtSummaryValue::Int(-24_000))]);
    let unhappy = |village: &Village| {
        village
            .entity(baby)
            .map_or(0, |entity| match entity.data() {
                EntityData::GeneralLiving(data) => data
                    .abstract_villager_retained()
                    .map_or(0, |merchant| merchant.unhappy_counter),
                _ => 0,
            })
    };
    play(
        &village.sender,
        &village.receiver,
        &mut village.world,
        1,
        super::feeding::interact_entity_packet(baby),
    );
    assert!(unhappy(&village) > 30);
    village.tick_until(45, |_| false);
    assert_eq!(unhappy(&village), 0);
}

#[test]
fn of_two_villagers_holding_one_workstation_the_less_experienced_gives_it_up() {
    let mut village = Village::new();
    village.command("time set noon");
    let lectern = village.at(2, 0);
    village.set_block(lectern, "minecraft:lectern");
    let job_site = NbtSummaryValue::Compound(vec![
        entry(
            "dimension",
            NbtSummaryValue::String("minecraft:overworld".into()),
        ),
        entry(
            "pos",
            NbtSummaryValue::IntArray(vec![lectern.x, lectern.y, lectern.z]),
        ),
    ]);
    let librarian = |village: &mut Village, pos: BlockPos, xp: i32| {
        let villager = village.villager(pos);
        let memories = NbtSummaryValue::Compound(vec![entry(
            "minecraft:job_site",
            NbtSummaryValue::Compound(vec![entry("value", job_site.clone())]),
        )]);
        village.merge(
            villager,
            vec![
                entry(
                    "VillagerData",
                    NbtSummaryValue::Compound(vec![
                        entry("level", NbtSummaryValue::Int(1)),
                        entry(
                            "profession",
                            NbtSummaryValue::String("minecraft:librarian".into()),
                        ),
                        entry("type", NbtSummaryValue::String("minecraft:plains".into())),
                    ]),
                ),
                entry("Xp", NbtSummaryValue::Int(xp)),
                entry(
                    "Brain",
                    NbtSummaryValue::Compound(vec![entry("memories", memories)]),
                ),
            ],
        );
        villager
    };
    let (novice_at, expert_at) = (village.at(0, 0), village.at(0, 2));
    let novice = librarian(&mut village, novice_at, 0);
    let expert = librarian(&mut village, expert_at, 10);
    let profession = |village: &Village, villager: i32| {
        village
            .entity(villager)
            .and_then(|entity| match entity.data() {
                EntityData::GeneralLiving(data) => data
                    .villager_profession_and_level()
                    .map(|(profession, _)| profession.to_owned()),
                _ => None,
            })
    };

    // The scan takes the workstation from the villager with less trade
    // experience, who never traded and so loses the profession too.
    assert!(village.tick_until(200, |village| {
        profession(village, novice).as_deref() == Some("minecraft:none")
    }));
    assert_eq!(
        profession(&village, expert).as_deref(),
        Some("minecraft:librarian")
    );
}

#[test]
fn a_villager_that_used_up_its_restocks_restocks_again_on_the_next_day() {
    let mut village = Village::new();
    // The start of the second day's work: day 0 never counts as a day passed.
    village.command("time set 26000");
    village.set_block(village.at(2, 0), "minecraft:lectern");
    let librarian = village.villager(village.at(0, 0));
    let game_time = village.world.game_time();
    let stack = |id: &str| {
        NbtSummaryValue::Compound(vec![
            entry("id", NbtSummaryValue::String(id.into())),
            entry("count", NbtSummaryValue::Int(1)),
        ])
    };
    village.merge(
        librarian,
        vec![
            entry(
                "VillagerData",
                NbtSummaryValue::Compound(vec![
                    entry("level", NbtSummaryValue::Int(1)),
                    entry(
                        "profession",
                        NbtSummaryValue::String("minecraft:librarian".into()),
                    ),
                    entry("type", NbtSummaryValue::String("minecraft:plains".into())),
                ]),
            ),
            entry("Xp", NbtSummaryValue::Int(1)),
            entry("RestocksToday", NbtSummaryValue::Int(2)),
            entry("LastRestock", NbtSummaryValue::Long(game_time)),
            // One sold-out offer of paper for an emerald.
            entry(
                "Offers",
                NbtSummaryValue::Compound(vec![entry(
                    "Recipes",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("buy", stack("minecraft:paper")),
                        entry("sell", stack("minecraft:emerald")),
                        entry("uses", NbtSummaryValue::Int(1)),
                        entry("maxUses", NbtSummaryValue::Int(1)),
                    ])]),
                )]),
            ),
        ],
    );
    let out_of_stock = |village: &Village| {
        village
            .entity(librarian)
            .and_then(crate::entity::Entity::merchant_offers)
            .is_some_and(|offers| offers.offers()[0].is_out_of_stock())
    };

    // `Villager.shouldRestock` (`Villager.java:410-428`) only sees a new day
    // after it checked on an earlier one, and it checks only when the walk
    // brings the villager to the lectern; both of today's restocks are spent,
    // so that check leaves the offer sold out.
    let checked = |village: &Village| {
        village
            .entity(librarian)
            .and_then(|entity| match entity.data() {
                EntityData::GeneralLiving(data) => data.villager_work(),
                _ => None,
            })
            .is_some_and(|work| work.last_restock_check_day > 0)
    };
    assert!(village.tick_until(3_000, |village| checked(village)));
    assert!(out_of_stock(&village));

    // The next morning counts as a new day however little game time passed.
    village.command("time add 24000");
    // Strolls around the job site come between work attempts; work ends at 9000.
    assert!(village.tick_until(3_900, |village| !out_of_stock(village)));
}

fn employ_farmer(village: &mut Village, villager: i32, job_site: BlockPos) {
    let job_site = NbtSummaryValue::Compound(vec![
        entry(
            "dimension",
            NbtSummaryValue::String("minecraft:overworld".into()),
        ),
        entry(
            "pos",
            NbtSummaryValue::IntArray(vec![job_site.x, job_site.y, job_site.z]),
        ),
    ]);
    let memories = NbtSummaryValue::Compound(vec![entry(
        "minecraft:job_site",
        NbtSummaryValue::Compound(vec![entry("value", job_site)]),
    )]);
    village.merge(
        villager,
        vec![
            entry(
                "VillagerData",
                NbtSummaryValue::Compound(vec![
                    entry("type", NbtSummaryValue::String("minecraft:plains".into())),
                    entry(
                        "profession",
                        NbtSummaryValue::String("minecraft:farmer".into()),
                    ),
                    entry("level", NbtSummaryValue::Int(2)),
                ]),
            ),
            entry(
                "Brain",
                NbtSummaryValue::Compound(vec![entry("memories", memories)]),
            ),
        ],
    );
}

#[test]
fn a_farmer_bonemeals_its_growing_wheat() {
    let mut village = Village::new();
    village.command("gamerule random_tick_speed 0");
    village.command("time set 3000");
    let composter = village.at(6, -2);
    village.set_block(composter, "minecraft:composter");
    let crop = village.at(2, 0);
    village.set_block(
        BlockPos {
            y: crop.y - 1,
            ..crop
        },
        "minecraft:farmland",
    );
    village.set_block(crop, "minecraft:wheat[age=0]");
    let farmer = village.villager(crop);
    village.give(farmer, &[("minecraft:bone_meal", 8)]);
    employ_farmer(&mut village, farmer, composter);
    // Keep the farmer by its crop rather than at its composter.
    village.command(
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0",
    );

    let age = |village: &mut Village| {
        village
            .world
            .block_state(OVERWORLD, crop)
            .and_then(crate::block_behavior::crop::age)
    };
    assert!(
        village.tick_until(6_000, |village| age(village).is_some_and(|age| age > 0)),
        "the farmer bonemeals the wheat"
    );
    assert!(village.count_item(farmer, "minecraft:bone_meal") < 8);
}

#[test]
fn a_villager_shows_a_player_the_trades_its_held_item_buys() {
    // Working at its job site, and idle without one.
    for employed in [true, false] {
        shows_a_player_the_trades_its_held_item_buys(employed);
    }
}

fn shows_a_player_the_trades_its_held_item_buys(employed: bool) {
    let mut village = Village::new();
    village.command("time set 3000");
    let villager = village.villager(village.at(0, 0));
    if employed {
        let job_site = village.at(1, 1);
        village.set_block(job_site, "minecraft:composter");
        employ_farmer(&mut village, villager, job_site);
    }
    let offer = |cost: &str, result: &str| {
        crate::entity::MerchantOffer::new(
            crate::entity::MerchantItemCost::new(cost.into(), 20, Vec::new()).unwrap(),
            None,
            crate::entity::item::plain_stack(result, 1).unwrap(),
            16,
            2,
            0.05,
        )
    };
    assert!(village
        .world
        .stored_entity_mut_for_test(OVERWORLD, villager)
        .unwrap()
        .initialize_merchant_offers(crate::entity::MerchantOffers::new(vec![
            offer("minecraft:wheat", "minecraft:emerald"),
            offer("minecraft:pumpkin", "minecraft:bread"),
        ])));
    village.command(
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0",
    );
    village.world.stock_menu_slot_for_test(
        ConnectionId(1),
        36,
        bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WHEAT_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );

    let held = |village: &Village| {
        village.entity(villager).and_then(|entity| {
            entity
                .living()?
                .equipment()
                .get(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
                .and_then(|stack| stack.item_resource_id().map(str::to_owned))
        })
    };
    assert!(
        village.tick_until(1_200, |village| held(village).as_deref()
            == Some("minecraft:emerald")),
        "the villager holds up the emerald the player's wheat buys (employed: {employed})"
    );
}

#[test]
fn a_bedless_villager_at_rest_walks_to_the_closest_bed() {
    let mut village = Village::new();
    village.command("time set midnight");
    village.command("gamerule advance_time false");
    let bed = village.bed(village.at(0, 12));
    let resident = village.villager(village.at(3, 12));
    assert!(village.tick_until(1_200, |village| village.sleeping_in(resident, bed)));
    village.merge(resident, vec![entry("NoAI", NbtSummaryValue::Byte(1))]);
    let bedless = village.villager(village.at(0, -10));
    let home = village.center(bed);
    assert!(
        village.tick_until(1_200, |village| village.distance(bedless, home) < 3.0),
        "the bedless villager heads for the occupied bed"
    );
}
