use super::entity_publication_fixtures::{entity_nbt, nbt_entry, uuid_nbt};
use super::*;
use crate::{Java26CreatedPlayer, SaveHolderResolver};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::PlayerAbilities;
use bbb_protocol::NbtSummaryEntry;
use std::fs;

struct Cleanup(PathBuf);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn save_patch(uuid: Uuid) -> Java26PlayerSavePatch {
    Java26PlayerSavePatch {
        stats: bbb_server_types::stats::ServerStatsCounter::new(),
        uuid,
        position: [1.5, 64.0, 2.5],
        motion: [0.0; 3],
        rotation: [90.0, -30.0],
        on_ground: true,
        custom_name: None,
        custom_data: Vec::new(),
        tags: Vec::new(),
        root_vehicle: None,
        ender_pearls: Vec::new(),
        shoulder_entities: Default::default(),
        respawn: None,
        last_death_location: None,
        previous_game_mode: None,
        entered_nether_pos: None,
        raid_omen_position: None,
        warden_spawn_tracker: None,
        created: Java26CreatedPlayer {
            sleep_timer: 0,
            sleeping_pos: None,
            last_hurt_by: Default::default(),
            spawn_extra_particles_on_fall: false,
            seen_credits: false,
            impulse_reset_grace_time: 0,
            impulse_impact_position: None,
            has_visual_fire: false,
            silent: false,
            glowing: false,
            fall_distance: 0.0,
            remaining_fire_ticks: 0,
            air_supply: 300,
            ticks_frozen: 0,
            portal_cooldown: 0,
            dimension: "minecraft:overworld".to_string(),
            game_mode: GameType::Creative,
            entity_invulnerable: true,
            attributes: Vec::new(),
            active_effects: Vec::new(),
            locator_bar_icon: bbb_protocol::packets::WaypointIcon {
                style: "minecraft:default".to_string(),
                color_rgb: None,
            },
            damage: bbb_server_types::player_damage::PlayerDamageState::created(),
            food: bbb_server_types::food::FoodData::created(),
            experience: bbb_server_types::experience::PlayerExperience::created(),
            abilities: PlayerAbilities {
                invulnerable: true,
                flying: true,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            },
            inventory: vec![bbb_protocol::packets::ItemStackSummary::empty(); 43].into(),
            ender_items: vec![bbb_protocol::packets::ItemStackSummary::empty(); 27].into(),
            fall_flying: false,
            selected_item_slot: 3,
            recipe_book_settings: [true, false, false, false, false, false, false, false],
            recipe_book_recipes: Vec::new(),
            recipe_book_to_be_displayed: Vec::new(),
        },
    }
}

fn storage(name: &str) -> (Cleanup, Java26WorldStorage) {
    let root =
        std::env::temp_dir().join(format!("bbb-item-root-input-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let paths = WorldSavePaths::new(&root).unwrap();
    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
    let save = crate::world_storage::entity_publication_fixtures::test_created_save(
        &crate::Java26WorldCreation {
            level_name: "item input".into(),
            game_type: bbb_protocol::game_types::VANILLA_GAME_TYPE_CREATIVE_ID,
            difficulty: "easy".into(),
            spawn_position: [0, 65, 0],
            last_played: 1_786_000_000_000,
        },
        &fixture,
        &[],
    )
    .unwrap();
    storage
        .create_world(&save, RegionCompression::Zlib, 1)
        .unwrap();
    (Cleanup(root), storage)
}

fn player_document(uuid: Uuid, entity: Vec<NbtSummaryEntry>) -> NamedCompound {
    let mut document = created_player_document(
        &save_patch(uuid),
        SaveHolderResolver::unavailable().as_dyn(),
    )
    .unwrap();
    document.entries_mut().push(nbt_entry(
        "RootVehicle",
        NbtSummaryValue::Compound(vec![
            nbt_entry("Attach", uuid_nbt(Uuid::from_u128(1))),
            nbt_entry("Entity", NbtSummaryValue::Compound(entity)),
        ]),
    ));
    document
}

fn write_raw_player(storage: &Java26WorldStorage, uuid: Uuid, document: &NamedCompound) -> Vec<u8> {
    let path = storage
        .paths()
        .player_data_dir()
        .join(format!("{}.dat", uuid.hyphenated()));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes =
        crate::level_file::encode_gzip_named_compound(&path, document, WorldSaveLimits::default())
            .unwrap();
    fs::write(path, &bytes).unwrap();
    bytes
}

#[test]
fn item_root_vehicle_input_tests_disk_candidate_preserves_native_payloads_and_passengers() {
    for (index, item) in [
        None,
        Some(NbtSummaryValue::Compound(vec![])),
        Some(NbtSummaryValue::String("bad".into())),
        Some(NbtSummaryValue::Compound(vec![
            nbt_entry("id", NbtSummaryValue::String("minecraft:stone".into())),
            nbt_entry(
                "components",
                NbtSummaryValue::Compound(vec![nbt_entry(
                    "minecraft:damage",
                    NbtSummaryValue::String("Native usable partial".into()),
                )]),
            ),
        ])),
    ]
    .into_iter()
    .enumerate()
    {
        let mut root = entity_nbt("minecraft:item", Uuid::from_u128(1), 1.0);
        root.extend([
            nbt_entry("Health", NbtSummaryValue::Long(70_000)),
            nbt_entry("Age", NbtSummaryValue::String("bad".into())),
            nbt_entry("PickupDelay", NbtSummaryValue::Float(2.5_f32.to_bits())),
            nbt_entry("Owner", NbtSummaryValue::IntArray(vec![1, 2, 3])),
            nbt_entry("Thrower", NbtSummaryValue::String("bad UUID".into())),
            nbt_entry(
                "Passengers",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entity_nbt(
                    "minecraft:item",
                    Uuid::from_u128(2),
                    1.0,
                ))]),
            ),
        ]);
        if let Some(item) = item {
            root.push(nbt_entry("Item", item));
        }
        let uuid = Uuid::from_u128(7);
        let raw = player_document(uuid, root);
        let parsed = Java26PlayerData::parse(raw.clone(), uuid).unwrap();
        assert_eq!(parsed.raw(), &raw);
        let (_cleanup, storage) = storage(&index.to_string());
        let bytes = write_raw_player(&storage, uuid, parsed.raw());
        let candidate = storage.load_candidate().unwrap();
        assert!(candidate.players.is_empty());
        assert_eq!(candidate.stored_players.len(), 1);
        let (_, player) = candidate.load_player(uuid).unwrap().unwrap();
        assert!(!player.has_retained_raw());
        assert!(player.root_vehicle().is_some());
        let path = storage
            .paths()
            .player_data_dir()
            .join(format!("{}.dat", uuid.hyphenated()));
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn item_root_vehicle_input_tests_disk_candidate_with_a_mistyped_common_field_keeps_the_vehicle() {
    let mut root = entity_nbt("minecraft:item", Uuid::from_u128(1), 1.0);
    root.retain(|field| field.name != "Air");
    root.push(nbt_entry(
        "Air",
        NbtSummaryValue::String("bad common field".into()),
    ));
    let uuid = Uuid::from_u128(7);
    let raw = player_document(uuid, root);
    let (_cleanup, storage) = storage("common-field");
    let bytes = write_raw_player(&storage, uuid, &raw);
    let candidate = storage.load_candidate().unwrap();
    let (_, player) = candidate.load_player(uuid).unwrap().unwrap();
    assert!(player.root_vehicle().is_some());
    assert!(!player.has_retained_raw());
    let path = storage
        .paths()
        .player_data_dir()
        .join(format!("{}.dat", uuid.hyphenated()));
    assert_eq!(fs::read(path).unwrap(), bytes);
}

mod native_attach_input;

mod native_arrow_input;
