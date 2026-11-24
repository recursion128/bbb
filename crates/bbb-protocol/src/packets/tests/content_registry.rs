use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::{
    codec::Encoder,
    content_registry::{
        ContentRegistry, ContentRegistryBuilder, ModBlockRegistration, ModItemRegistration,
        ModParticleRegistration,
    },
    data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
    packets::chunks::{validate_block_id, validate_block_state_id},
    packets::world_effects::{
        particle_command_options_with_registry, ParticleOptions, VanillaParticleCommandOptions,
    },
};

fn build_session_mod_registry() -> ContentRegistry {
    let mut builder = ContentRegistryBuilder::new();
    builder.register_block(ModBlockRegistration {
        resource_id: "modded:crystal_block".to_string(),
        state_count: 2,
        light_emission: 0,
        destroy_speed: 1.5,
        full_cube_collision: true,
        dynamic_light_rgb: None,
    });
    builder.register_item(ModItemRegistration {
        resource_id: "modded:crystal_shard".to_string(),
        max_stack_size: 16,
        is_block_item: false,
    });
    builder.register_particle(ModParticleRegistration {
        resource_id: "modded:crystal_spark".to_string(),
        override_limiter: false,
    });
    builder.register_sound_event("modded:crystal_chime");
    let wire = builder.freeze_for_session().unwrap().encode_wire();
    ContentRegistry::decode_wire(&wire).unwrap()
}

fn write_empty_patch(out: &mut Encoder) {
    out.write_var_i32(0);
    out.write_var_i32(0);
}

#[test]
fn play_clientbound_blocks_round_trip_with_content_registry_and_reject_on_vanilla() {
    let registry = build_session_mod_registry();
    let block = registry
        .block_by_resource_id("modded:crystal_block")
        .unwrap();
    let first_state = block.first_state_id;
    let second_state = first_state + 1;
    let invalid_state = first_state + 2;

    assert_eq!(
        validate_block_id(block.block_id, &registry).unwrap(),
        block.block_id
    );
    assert!(validate_block_id(block.block_id, ContentRegistry::vanilla()).is_err());
    assert_eq!(
        validate_block_state_id(first_state, &registry).unwrap(),
        first_state
    );
    assert_eq!(
        validate_block_state_id(second_state, &registry).unwrap(),
        second_state
    );
    assert!(validate_block_state_id(invalid_state, &registry).is_err());
    assert!(validate_block_state_id(first_state, ContentRegistry::vanilla()).is_err());

    let pos = BlockPos { x: 4, y: 65, z: -3 };
    let block_update = BlockUpdate {
        pos,
        block_state_id: second_state,
    };
    let (id, payload) =
        encode_play_clientbound_block_update_with_content_registry(block_update, &registry)
            .unwrap();
    assert_eq!(
        decode_play_clientbound_with_content_registry(id, &payload, &registry).unwrap(),
        PlayClientbound::BlockUpdate(block_update)
    );
    assert!(decode_play_clientbound(id, &payload).is_err());

    let section_update = SectionBlocksUpdate {
        section_x: 0,
        section_y: 4,
        section_z: -1,
        updates: vec![BlockUpdate {
            pos: BlockPos { x: 1, y: 66, z: -2 },
            block_state_id: first_state,
        }],
    };
    let (id, payload) = encode_play_clientbound_section_blocks_update_with_content_registry(
        &section_update,
        &registry,
    )
    .unwrap();
    assert_eq!(
        decode_play_clientbound_with_content_registry(id, &payload, &registry).unwrap(),
        PlayClientbound::SectionBlocksUpdate(section_update)
    );
    assert!(decode_play_clientbound(id, &payload).is_err());

    let block_event = BlockEvent {
        pos,
        b0: 1,
        b1: 2,
        block_id: block.block_id,
    };
    let (id, payload) =
        encode_play_clientbound_block_event_with_content_registry(block_event, &registry).unwrap();
    assert_eq!(
        decode_play_clientbound_with_content_registry(id, &payload, &registry).unwrap(),
        PlayClientbound::BlockEvent(block_event)
    );
    assert!(decode_play_clientbound(id, &payload).is_err());
}

#[test]
fn play_clientbound_items_decode_with_content_registry_and_reject_on_vanilla() {
    let registry = build_session_mod_registry();
    let item = registry
        .item_by_resource_id("modded:crystal_shard")
        .unwrap();
    let mod_item_id = item.id;

    let mut slot_payload = Encoder::new();
    slot_payload.write_var_i32(0);
    slot_payload.write_var_i32(1);
    slot_payload.write_i16(3);
    slot_payload.write_var_i32(2);
    slot_payload.write_var_i32(mod_item_id);
    slot_payload.write_var_i32(1);
    slot_payload.write_var_i32(0);
    slot_payload.write_var_i32(VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID);
    slot_payload.write_var_i32(mod_item_id);
    slot_payload.write_var_i32(1);
    write_empty_patch(&mut slot_payload);
    let slot_payload = slot_payload.into_inner();
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_CONTAINER_SET_SLOT, &slot_payload).is_err()
    );
    let PlayClientbound::ContainerSetSlot(decoded_slot) =
        decode_play_clientbound_with_content_registry(
            ids::play::CLIENTBOUND_CONTAINER_SET_SLOT,
            &slot_payload,
            &registry,
        )
        .unwrap()
    else {
        panic!("expected ContainerSetSlot");
    };
    assert_eq!(decoded_slot.item.item_id, Some(mod_item_id));
    assert_eq!(
        decoded_slot
            .item
            .component_patch
            .use_remainder
            .as_ref()
            .unwrap()
            .item_id,
        mod_item_id
    );

    let mut entity_data_payload = Encoder::new();
    entity_data_payload.write_var_i32(99);
    entity_data_payload.write_u8(8);
    entity_data_payload.write_var_i32(VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID);
    entity_data_payload.write_var_i32(1);
    entity_data_payload.write_var_i32(mod_item_id);
    write_empty_patch(&mut entity_data_payload);
    entity_data_payload.write_u8(0xff);
    let entity_data_payload = entity_data_payload.into_inner();
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_SET_ENTITY_DATA, &entity_data_payload)
            .is_err()
    );
    let PlayClientbound::SetEntityData(decoded_entity_data) =
        decode_play_clientbound_with_content_registry(
            ids::play::CLIENTBOUND_SET_ENTITY_DATA,
            &entity_data_payload,
            &registry,
        )
        .unwrap()
    else {
        panic!("expected SetEntityData");
    };
    assert_eq!(
        decoded_entity_data.values[0].value,
        EntityDataValueKind::ItemStack(ItemStackSummary {
            item_id: Some(mod_item_id),
            count: 1,
            component_patch: Default::default(),
        })
    );

    let mut merchant_payload = Encoder::new();
    merchant_payload.write_var_i32(4);
    merchant_payload.write_var_i32(1);
    merchant_payload.write_var_i32(mod_item_id);
    merchant_payload.write_var_i32(2);
    merchant_payload.write_var_i32(0);
    merchant_payload.write_var_i32(1);
    merchant_payload.write_var_i32(mod_item_id);
    write_empty_patch(&mut merchant_payload);
    merchant_payload.write_bool(false);
    merchant_payload.write_bool(false);
    merchant_payload.write_i32(0);
    merchant_payload.write_i32(4);
    merchant_payload.write_i32(5);
    merchant_payload.write_i32(0);
    merchant_payload.write_f32(0.05);
    merchant_payload.write_i32(0);
    merchant_payload.write_var_i32(1);
    merchant_payload.write_var_i32(0);
    merchant_payload.write_bool(true);
    merchant_payload.write_bool(false);
    let merchant_payload = merchant_payload.into_inner();
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_MERCHANT_OFFERS, &merchant_payload).is_err()
    );
    let PlayClientbound::MerchantOffers(decoded_offers) =
        decode_play_clientbound_with_content_registry(
            ids::play::CLIENTBOUND_MERCHANT_OFFERS,
            &merchant_payload,
            &registry,
        )
        .unwrap()
    else {
        panic!("expected MerchantOffers");
    };
    assert_eq!(decoded_offers.offers[0].buy_a.item_id, mod_item_id);
    assert_eq!(decoded_offers.offers[0].sell.item_id, Some(mod_item_id));
}

#[test]
fn play_clientbound_particles_and_sounds_round_trip_with_content_registry_and_reject_on_vanilla() {
    let registry = build_session_mod_registry();
    let particle = registry
        .particle_by_resource_id("modded:crystal_spark")
        .unwrap();
    let sound = registry
        .sound_event_by_resource_id("modded:crystal_chime")
        .unwrap();
    assert!(sound.id >= ContentRegistry::vanilla().total_sound_event_count() as i32);

    assert_eq!(
        particle_command_options_with_registry("modded:crystal_spark", &registry),
        Some(VanillaParticleCommandOptions::Empty)
    );
    assert_eq!(
        particle_command_options_with_registry("modded:crystal_spark", ContentRegistry::vanilla()),
        None
    );

    let payload = ParticleOptions::Simple
        .payload_with_registry(particle.id, &registry)
        .unwrap();
    assert!(payload.raw_options.is_empty());
    assert_eq!(
        ParticleOptions::decode_with_registry(particle.id, &payload.raw_options, &registry)
            .unwrap(),
        ParticleOptions::Simple
    );
    assert!(ParticleOptions::Simple
        .payload_with_registry(particle.id, ContentRegistry::vanilla())
        .is_err());
    assert!(ParticleOptions::decode(particle.id, &[]).is_err());

    let level_particles = LevelParticles {
        override_limiter: true,
        always_show: false,
        position: Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        offset: Vec3d {
            x: 0.25,
            y: 0.5,
            z: 0.75,
        },
        max_speed: 0.1,
        count: 5,
        particle: payload.clone(),
    };
    let (id, bytes) = encode_play_clientbound_level_particles(level_particles.clone());
    assert_eq!(
        decode_play_clientbound_with_content_registry(id, &bytes, &registry).unwrap(),
        PlayClientbound::LevelParticles(level_particles)
    );
    assert!(decode_play_clientbound(id, &bytes).is_err());

    let explosion = Explosion {
        center: Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        radius: 2.0,
        block_count: 1,
        player_knockback: None,
        explosion_particle: payload.clone(),
        explosion_sound: SoundEventHolder::Reference {
            registry_id: sound.id,
        },
        block_particles: vec![WeightedExplosionParticle {
            particle: payload,
            scaling: 1.0,
            speed: 0.5,
            weight: 1,
        }],
    };
    let (id, bytes) = encode_play_clientbound_explode(explosion.clone()).unwrap();
    assert_eq!(
        decode_play_clientbound_with_content_registry(id, &bytes, &registry).unwrap(),
        PlayClientbound::Explosion(explosion)
    );
    assert!(decode_play_clientbound(id, &bytes).is_err());

    let sound_packet = SoundEvent {
        sound: SoundEventHolder::Reference {
            registry_id: sound.id,
        },
        source: SoundSource::Blocks,
        position: Vec3d {
            x: 10.0,
            y: 64.0,
            z: -5.0,
        },
        volume: 1.0,
        pitch: 1.0,
        seed: 7,
    };
    let (id, bytes) = encode_play_clientbound_sound(&sound_packet).unwrap();
    assert_eq!(
        decode_play_clientbound_with_content_registry(id, &bytes, &registry).unwrap(),
        PlayClientbound::Sound(sound_packet)
    );
    assert!(decode_play_clientbound(id, &bytes).is_err());

    let sound_entity_packet = SoundEntityEvent {
        sound: SoundEventHolder::Reference {
            registry_id: sound.id,
        },
        source: SoundSource::Neutral,
        entity_id: 55,
        volume: 0.8,
        pitch: 1.2,
        seed: 19,
    };
    let (id, bytes) = encode_play_clientbound_sound_entity(&sound_entity_packet).unwrap();
    assert_eq!(
        decode_play_clientbound_with_content_registry(id, &bytes, &registry).unwrap(),
        PlayClientbound::SoundEntity(sound_entity_packet)
    );
    assert!(decode_play_clientbound(id, &bytes).is_err());
}

#[test]
fn play_serverbound_items_round_trip_with_content_registry_and_reject_on_vanilla() {
    let registry = build_session_mod_registry();
    let mod_item_id = registry
        .item_by_resource_id("modded:crystal_shard")
        .unwrap()
        .id;
    let dummy_component_id = VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID;

    let click = ContainerClick {
        container_id: 0,
        state_id: 2,
        slot_num: 9,
        button_num: 0,
        input: ContainerInput::Pickup,
        changed_slots: BTreeMap::from([(
            9,
            HashedStack::Item(HashedItemStack {
                item_id: mod_item_id,
                count: 3,
                components: HashedComponentPatch {
                    added_components: BTreeMap::from([(0, 12345)]),
                    removed_components: BTreeSet::from([dummy_component_id]),
                },
            }),
        )]),
        carried_item: HashedStack::Item(HashedItemStack {
            item_id: mod_item_id,
            count: 1,
            components: HashedComponentPatch::default(),
        }),
    };
    let (id, bytes) =
        encode_play_container_click_with_content_registry(click.clone(), &registry).unwrap();
    assert_eq!(
        decode_play_serverbound_with_content_registry(id, &bytes, &registry).unwrap(),
        PlayServerbound::ContainerClick(click)
    );
    assert!(decode_play_serverbound(id, &bytes).is_err());

    let creative = SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(mod_item_id),
            count: 4,
            component_patch: Default::default(),
        },
    };
    let (id, bytes) =
        encode_play_set_creative_mode_slot_with_content_registry(&creative, &registry).unwrap();
    assert_eq!(
        decode_play_serverbound_with_content_registry(id, &bytes, &registry).unwrap(),
        PlayServerbound::SetCreativeModeSlot(creative)
    );
    assert!(decode_play_serverbound(id, &bytes).is_err());
}

#[test]
fn content_registry_plumbing_covers_entity_data_bees_lock_particles_slot_display_text_and_selectors(
) {
    use crate::component::{
        is_valid_entity_selector, is_valid_entity_selector_with_registry, RichTextComponent,
        RichTextStyle,
    };
    use crate::content_registry::ModEntityRegistration;
    use crate::data_component_types::{
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID, VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
    };
    use crate::packets::data_components::{
        BeeOccupantSummary, DataComponentPatchSummary, HolderSetSummary, ItemStackTemplateSummary,
        PersistentItemPredicateSummary,
    };
    use crate::packets::{
        encode_play_clientbound_set_action_bar_text, encode_play_clientbound_set_title_text,
        ParticleOptions, SetActionBarText, SetTitleText, SlotDisplaySummary, SlotDisplayValue,
    };
    use crate::{NbtSummaryEntry, NbtSummaryValue};

    let mut builder = ContentRegistryBuilder::new();
    builder.register_item(ModItemRegistration {
        resource_id: "demo:ruby".to_string(),
        max_stack_size: 16,
        is_block_item: false,
    });
    builder.register_entity(ModEntityRegistration {
        resource_id: "demo:golem".to_string(),
        width: 1.0,
        height: 2.0,
        ..ModEntityRegistration::default()
    });
    let registry =
        ContentRegistry::decode_wire(&builder.freeze_for_session().unwrap().encode_wire()).unwrap();
    let mod_item_id = registry.item_by_resource_id("demo:ruby").unwrap().id;
    let mod_entity_id = registry.entity_by_resource_id("demo:golem").unwrap().id;

    // R2-54: entity selectors accept custom registered entity types only with ContentRegistry.
    assert!(!is_valid_entity_selector("@e[type=demo:golem]"));
    assert!(is_valid_entity_selector_with_registry(
        "@e[type=demo:golem]",
        &registry
    ));

    // R1-04: ENTITY_DATA, BEES, and LOCK round-trip custom mod entity/item ids.
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![
            VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        ],
        entity_data_entity_type_id: Some(mod_entity_id),
        entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
        bees_present: true,
        bees: vec![BeeOccupantSummary {
            entity_type_id: mod_entity_id,
            entity_data_tag: Some(NbtSummaryValue::Compound(Vec::new())),
            ticks_in_hive: 10,
            min_ticks_in_hive: 100,
        }],
        lock_predicate: Some(PersistentItemPredicateSummary {
            items: Some(HolderSetSummary {
                registry_ids: vec![mod_item_id],
                tag: None,
            }),
            ..PersistentItemPredicateSummary::default()
        }),
        ..DataComponentPatchSummary::default()
    };
    let slot_pkt = ContainerSetSlot {
        container_id: 0,
        state_id: 1,
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(mod_item_id),
            count: 1,
            component_patch: patch.clone().into(),
        },
    };
    let (id, bytes) =
        encode_play_clientbound_container_set_slot_with_content_registry(&slot_pkt, &registry)
            .unwrap();
    assert_eq!(
        decode_play_clientbound_with_content_registry(id, &bytes, &registry).unwrap(),
        PlayClientbound::ContainerSetSlot(slot_pkt)
    );
    assert!(decode_play_clientbound(id, &bytes).is_err());

    // R1-04: ParticleOptions::Item passes ContentRegistry when encoding nested patch.
    let item_particle_id =
        crate::particle_types::vanilla_particle_type_for_resource_id("minecraft:item")
            .unwrap()
            .id;
    let payload = ParticleOptions::Item(Box::new(ItemStackTemplateSummary {
        item_id: mod_item_id,
        count: 1,
        component_patch: patch.into(),
    }))
    .payload_with_registry(item_particle_id, &registry)
    .unwrap();
    assert_eq!(payload.particle_type_id, item_particle_id);

    // R1-04: SlotDisplaySummary::decode_value_arc_with_content_registry decodes custom mod items.
    let display = SlotDisplaySummary::from_value_with_content_registry(
        SlotDisplayValue::Item {
            stack: Box::new(ItemStackSummary {
                item_id: Some(mod_item_id),
                count: 1,
                component_patch: Default::default(),
            }),
        },
        &registry,
    )
    .unwrap();
    assert!(display.decode_value_arc().is_err());
    assert!(display
        .decode_value_arc_with_content_registry(&registry)
        .is_ok());

    // R1-04: Play text packets & RichTextStyle accept hover_event referencing custom mod item/entity.
    let hover_style_nbt = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "hover_event".into(),
        value: NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "action".into(),
                value: NbtSummaryValue::string("show_item"),
            },
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("demo:ruby"),
            },
        ]),
    }]);
    assert!(RichTextStyle::try_from_style_nbt(hover_style_nbt.clone()).is_err());
    assert!(RichTextStyle::try_from_style_nbt_with_registry(hover_style_nbt, &registry).is_ok());

    let hover_component_nbt = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::string("ruby"),
        },
        NbtSummaryEntry {
            name: "hover_event".into(),
            value: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "action".into(),
                    value: NbtSummaryValue::string("show_item"),
                },
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::string("demo:ruby"),
                },
            ]),
        },
    ]);
    let comp =
        RichTextComponent::try_from_component_nbt_with_registry(hover_component_nbt, &registry)
            .unwrap();
    let (title_id, title_bytes) = encode_play_clientbound_set_title_text(&SetTitleText {
        content: comp.clone(),
    })
    .unwrap();
    assert!(decode_play_clientbound(title_id, &title_bytes).is_err());
    assert!(
        decode_play_clientbound_with_content_registry(title_id, &title_bytes, &registry).is_ok()
    );

    let (bar_id, bar_bytes) =
        encode_play_clientbound_set_action_bar_text(&SetActionBarText { content: comp }).unwrap();
    assert!(decode_play_clientbound(bar_id, &bar_bytes).is_err());
    assert!(decode_play_clientbound_with_content_registry(bar_id, &bar_bytes, &registry).is_ok());
}
