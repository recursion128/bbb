//! The entity family: attribute get, damage, kill, swing, rotate, spectate, ride and tag.

use bbb_protocol::{
    attribute_types::VANILLA_ATTRIBUTE_TYPES,
    component_literal_nbt,
    entity_anchors::EntityAnchor,
    entity_types::{
        vanilla_entity_type, vanilla_entity_type_client_tracking_range,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    },
    game_types::GameType,
    packets::Vec3d,
    NbtSummaryValue, RichTextComponent,
};
use bbb_server_types::save::Java26LevelKey;

use crate::entity::{AttributeMap, AttributeOperation, EntityData, MAX_ENTITY_TAG_COUNT};
use bbb_server_datapack::damage::GENERIC_DAMAGE_TYPE;

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::types::{
    CommandAttributeTarget, CommandDamageEntityRef, CommandDamageSourceEntity, CommandLookAt,
};
use crate::commands::{
    format_list, team_player_display_name_nbt, translatable, CommandCx, CommandPermission,
    CommandPlayerTarget, CommandSource,
};
use crate::world::{EntityReads, ServerWorld};

use super::teleport::{look_at_target, FacingShape};
use super::{
    coordinates_argument, damage_entity_targets, double_argument, float_argument,
    identifier_argument, resource_argument, rotation_argument, select_damage_entity_states,
    single_damage_entity_argument, single_player_argument, source_entity, source_player_state,
    text_argument, view_of, CommandRun, Outcome,
};
use crate::commands::CommandOrigin;

/// `AttributeCommand.getAttributeValue` (`AttributeCommand.java:252-258`) and
/// `getAttributeBase` (`AttributeCommand.java:261-267`).
pub(super) fn get_attribute_value(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    scaled: bool,
    base: bool,
) -> Outcome {
    world.with_entity_reads(|reads| {
        let target = single_damage_entity_argument(parse, "target", source, at, reads)?;
        let attribute_id = resource_argument(parse, "attribute");
        let scale = if scaled {
            double_argument(parse, "scale")
        } else {
            1.0
        };
        let entry = VANILLA_ATTRIBUTE_TYPES
            .iter()
            .find(|entry| entry.resource_id == attribute_id)
            .expect("the resource argument only resolves catalog ids");
        let description = translatable(entry.description_key, Vec::new());
        let name = target.display_name(reads);
        let Some(map) = living_attribute_map(target) else {
            return Err(translatable("commands.attribute.failed.entity", vec![name]));
        };
        if !map.has_attribute(entry.id) {
            return Err(translatable(
                "commands.attribute.failed.no_attribute",
                vec![name, description],
            ));
        }
        let result = if base {
            map.base_value(entry.id)
        } else {
            map.value(entry.id)
        }
        .expect("hasAttribute already accepted this id");
        let key = if base {
            "commands.attribute.base_value.get.success"
        } else {
            "commands.attribute.value.get.success"
        };
        run.succeed(
            translatable(
                key,
                vec![description, name, NbtSummaryValue::Double(result.to_bits())],
            ),
            false,
        );
        Ok((result * scale) as i32)
    })
}

/// `AttributeCommand.setAttributeBase` (`AttributeCommand.java:294-299`).
pub(super) fn set_attribute_base(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let value = double_argument(parse, "value");
    let (target, attribute, description, name) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let target = single_damage_entity_argument(parse, "target", source, at, reads)?;
        let attribute_id = resource_argument(parse, "attribute");
        let entry = VANILLA_ATTRIBUTE_TYPES
            .iter()
            .find(|entry| entry.resource_id == attribute_id)
            .expect("the resource argument only resolves catalog ids");
        let description = translatable(entry.description_key, Vec::new());
        let name = target.display_name(reads);
        let Some(map) = living_attribute_map(target) else {
            return Err(translatable("commands.attribute.failed.entity", vec![name]));
        };
        // `AttributeCommand.getAttributeInstance` (`AttributeCommand.java:226-232`).
        if !map.has_attribute(entry.id) {
            return Err(translatable(
                "commands.attribute.failed.no_attribute",
                vec![name, description],
            ));
        }
        Ok::<_, NbtSummaryValue>((attribute_target(target), entry.id, description, name))
    })?;
    cx.world.set_attribute_base(&target, attribute, value);
    run.succeed(
        translatable(
            "commands.attribute.base_value.set.success",
            vec![description, name, NbtSummaryValue::Double(value.to_bits())],
        ),
        false,
    );
    Ok(1)
}

/// `AttributeCommand.resetAttributeBase` (`AttributeCommand.java:302-312`).
pub(super) fn reset_attribute_base(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let (target, attribute, description, name, value) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let target = single_damage_entity_argument(parse, "target", source, at, reads)?;
        let attribute_id = resource_argument(parse, "attribute");
        let entry = VANILLA_ATTRIBUTE_TYPES
            .iter()
            .find(|entry| entry.resource_id == attribute_id)
            .expect("the resource argument only resolves catalog ids");
        let description = translatable(entry.description_key, Vec::new());
        let name = target.display_name(reads);
        let Some(map) = living_attribute_map(target) else {
            return Err(translatable("commands.attribute.failed.entity", vec![name]));
        };
        let Some(value) = map.supplier().base_value(entry.id) else {
            return Err(translatable(
                "commands.attribute.failed.no_attribute",
                vec![name, description],
            ));
        };
        Ok::<_, NbtSummaryValue>((attribute_target(target), entry.id, description, name, value))
    })?;
    cx.world.reset_attribute_base(&target, attribute);
    run.succeed(
        translatable(
            "commands.attribute.base_value.reset.success",
            vec![description, name, NbtSummaryValue::Double(value.to_bits())],
        ),
        false,
    );
    Ok(1)
}

/// `AttributeCommand.addModifier` (`AttributeCommand.java:315-336`).
pub(super) fn add_attribute_modifier(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    operation: AttributeOperation,
) -> Outcome {
    let value = double_argument(parse, "value");
    let id = identifier_argument(parse, "id").to_string();
    let (target, attribute, description, name) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let target = single_damage_entity_argument(parse, "target", source, at, reads)?;
        let attribute_id = resource_argument(parse, "attribute");
        let entry = VANILLA_ATTRIBUTE_TYPES
            .iter()
            .find(|entry| entry.resource_id == attribute_id)
            .expect("the resource argument only resolves catalog ids");
        let description = translatable(entry.description_key, Vec::new());
        let name = target.display_name(reads);
        let Some(map) = living_attribute_map(target) else {
            return Err(translatable("commands.attribute.failed.entity", vec![name]));
        };
        if !map.has_attribute(entry.id) {
            return Err(translatable(
                "commands.attribute.failed.no_attribute",
                vec![name, description],
            ));
        }
        if map.has_modifier(entry.id, &id) {
            return Err(translatable(
                "commands.attribute.failed.modifier_already_present",
                vec![component_literal_nbt(&id), description, name],
            ));
        }
        Ok::<_, NbtSummaryValue>((attribute_target(target), entry.id, description, name))
    })?;
    cx.world
        .add_attribute_modifier(&target, attribute, id.clone(), value, operation);
    run.succeed(
        translatable(
            "commands.attribute.modifier.add.success",
            vec![component_literal_nbt(&id), description, name],
        ),
        false,
    );
    Ok(1)
}

/// `AttributeCommand.removeModifier` (`AttributeCommand.java:339-351`).
pub(super) fn remove_attribute_modifier(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let id = identifier_argument(parse, "id").to_string();
    let (target, attribute, description, name) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let target = single_damage_entity_argument(parse, "target", source, at, reads)?;
        let attribute_id = resource_argument(parse, "attribute");
        let entry = VANILLA_ATTRIBUTE_TYPES
            .iter()
            .find(|entry| entry.resource_id == attribute_id)
            .expect("the resource argument only resolves catalog ids");
        let description = translatable(entry.description_key, Vec::new());
        let name = target.display_name(reads);
        let Some(map) = living_attribute_map(target) else {
            return Err(translatable("commands.attribute.failed.entity", vec![name]));
        };
        if !map.has_attribute(entry.id) {
            return Err(translatable(
                "commands.attribute.failed.no_attribute",
                vec![name, description],
            ));
        }
        if !map.has_modifier(entry.id, &id) {
            return Err(translatable(
                "commands.attribute.failed.no_modifier",
                vec![description, name, component_literal_nbt(&id)],
            ));
        }
        Ok::<_, NbtSummaryValue>((attribute_target(target), entry.id, description, name))
    })?;
    cx.world.remove_attribute_modifier(&target, attribute, &id);
    run.succeed(
        translatable(
            "commands.attribute.modifier.remove.success",
            vec![component_literal_nbt(&id), description, name],
        ),
        false,
    );
    Ok(1)
}

/// `AttributeCommand.getAttributeModifier` (`AttributeCommand.java:270-286`).
pub(super) fn get_attribute_modifier_value(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
    scaled: bool,
) -> Outcome {
    world.with_entity_reads(|reads| {
        let target = single_damage_entity_argument(parse, "target", source, at, reads)?;
        let attribute_id = resource_argument(parse, "attribute");
        let id = identifier_argument(parse, "id").to_string();
        let scale = if scaled {
            double_argument(parse, "scale")
        } else {
            1.0
        };
        let entry = VANILLA_ATTRIBUTE_TYPES
            .iter()
            .find(|entry| entry.resource_id == attribute_id)
            .expect("the resource argument only resolves catalog ids");
        let description = translatable(entry.description_key, Vec::new());
        let name = target.display_name(reads);
        let Some(map) = living_attribute_map(target) else {
            return Err(translatable("commands.attribute.failed.entity", vec![name]));
        };
        if !map.has_attribute(entry.id) {
            return Err(translatable(
                "commands.attribute.failed.no_attribute",
                vec![name, description],
            ));
        }
        if !map.has_modifier(entry.id, &id) {
            return Err(translatable(
                "commands.attribute.failed.no_modifier",
                vec![description, name, component_literal_nbt(&id)],
            ));
        }
        let result = map
            .instance(entry.id)
            .and_then(|instance| instance.modifier(&id))
            .map(|modifier| modifier.amount)
            .expect("hasModifier already accepted this id");
        run.succeed(
            translatable(
                "commands.attribute.modifier.value.get.success",
                vec![
                    component_literal_nbt(&id),
                    description,
                    name,
                    NbtSummaryValue::Double(result.to_bits()),
                ],
            ),
            false,
        );
        Ok((result * scale) as i32)
    })
}

fn attribute_target(target: CommandDamageEntityRef<'_>) -> CommandAttributeTarget {
    target.attribute_target()
}

/// `LivingEntity.getAttributes()` (`LivingEntity.java:2172-2174`).
fn living_attribute_map<'a>(target: CommandDamageEntityRef<'a>) -> Option<&'a AttributeMap> {
    match target {
        CommandDamageEntityRef::Player(player) => Some(player.attributes),
        CommandDamageEntityRef::Stored(stored) => {
            stored.entity.living().map(|living| living.attributes())
        }
    }
}

/// Which of the four `DamageSource` constructors the reached node builds
/// (DamageCommand.java:34, :44, :57, :73 and :86-89).
///
/// The bare `/damage <target> <amount>` node is the `None` case: it takes
/// `damageSources().generic()` rather than building a source out of arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DamageSourceShape {
    /// `new DamageSource(type)` (DamageCommand.java:44).
    Type,
    /// `new DamageSource(type, position)` (`:47-49`).
    At,
    /// `new DamageSource(type, entity)` (`:72-74`), which puts the same entity
    /// in both roles (DamageSource.java:51-53).
    By,
    /// `new DamageSource(type, entity, cause)` (`:86-89`).
    ByFrom,
}

impl<'a> CommandDamageEntityRef<'a> {
    pub(in crate::commands) fn position(self) -> [f64; 3] {
        match self {
            Self::Player(player) => player.position,
            Self::Stored(stored) => {
                let position = stored.entity.position();
                [position.x, position.y, position.z]
            }
        }
    }

    pub(in crate::commands) fn dimension(self) -> &'a Java26LevelKey {
        match self {
            Self::Player(player) => player.dimension,
            Self::Stored(stored) => stored.dimension,
        }
    }

    pub(in crate::commands) fn rotation(self) -> [f32; 2] {
        match self {
            Self::Player(player) => player.rotation,
            Self::Stored(stored) => [stored.entity.y_rot(), stored.entity.x_rot()],
        }
    }

    /// `EntityAnchorArgument.Anchor.apply(Entity)` (`EntityAnchorArgument.java:81-83`).
    pub(crate) fn anchored_position(self, anchor: EntityAnchor) -> [f64; 3] {
        let [x, y, z] = self.position();
        let y = match anchor {
            EntityAnchor::Feet => y,
            EntityAnchor::Eyes => y + f64::from(self.eye_height()),
        };
        [x, y, z]
    }

    /// Which `kill` body `/kill` hands this victim to.
    ///
    /// `LivingEntity.kill` (`LivingEntity.java:303-305`) replaces the base body
    /// with one `genericKill` hurt. `ArmorStand.kill` (`ArmorStand.java:461-464`)
    /// repeats `Entity.kill` (`Entity.java:399-402`) verbatim, so a stand
    /// answers `false` although it is a `LivingEntity`; every non-living kind
    /// the store holds reaches the base body, `ItemFrame.kill`
    /// (`ItemFrame.java:154-157`) included, whose `removeFramedMap` is already
    /// part of `EntityStore::remove`'s `should_destroy` teardown.
    /// `EnderDragon.kill` (`EnderDragon.java:495-502`) is taken apart from
    /// this answer by `ServerWorld::kill_by_command`.
    pub(super) fn kills_through_hurt(self) -> bool {
        match self {
            Self::Player(_) => true,
            Self::Stored(stored) => matches!(stored.entity.data(), EntityData::GeneralLiving(_)),
        }
    }

    /// `Entity.getType()`'s registry id, which the `type=` selector option
    /// compares (EntitySelectorOptions.java:299).
    pub(super) fn type_resource_id(self) -> &'a str {
        match self {
            Self::Player(_) => {
                vanilla_entity_type(VANILLA_ENTITY_TYPE_PLAYER_ID)
                    .expect("the player entity type is in the generated catalog")
                    .resource_id
            }
            Self::Stored(stored) => stored.entity.data().entity_resource_id(),
        }
    }

    /// `Entity.entityTags()` (Entity.java:387-389), which `/tag` writes and the
    /// `tag=` selector option reads (EntitySelectorOptions.java:308).
    pub(in crate::commands) fn entity_tags(self) -> &'a [bbb_protocol::JavaString] {
        match self {
            Self::Player(player) => player.tags,
            Self::Stored(stored) => stored.entity.entity_tags(),
        }
    }

    /// `Entity.getEyeHeight()` (Entity.java:3379-3381), which is what `EYES`
    /// raises a position by (EntityAnchorArgument.java:62).
    pub(in crate::commands) fn eye_height(self) -> f32 {
        match self {
            Self::Player(player) => player.eye_height,
            Self::Stored(stored) => stored.entity.eye_height(),
        }
    }

    /// `CommandSourceStack.withEntity` (`CommandSourceStack.java:151-169`):
    /// textName, displayName and entity change; permissions and the feedback
    /// `source` do not.
    pub(super) fn as_command_source_named(
        self,
        permission: CommandPermission,
        world: &EntityReads<'_>,
    ) -> CommandSource {
        self.command_source(
            permission,
            self.display_name(world),
            world.content_registry(),
        )
    }

    /// `as_command_source_named` with a display name the caller resolved.
    pub(crate) fn as_command_source_with_name(
        self,
        permission: CommandPermission,
        display_name: NbtSummaryValue,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> CommandSource {
        self.command_source(permission, display_name, content_registry)
    }

    /// A source for relation and function lookups, which never show its
    /// display name, so a stored entity is named by its type.
    pub(crate) fn as_command_source(self, permission: CommandPermission) -> CommandSource {
        self.command_source(
            permission,
            self.type_name(),
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    fn command_source(
        self,
        permission: CommandPermission,
        display_name: NbtSummaryValue,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> CommandSource {
        match self {
            Self::Player(player) => CommandSource::player(player.name, player.id, permission),
            Self::Stored(stored) => CommandSource::stored_entity(
                stored.entity.custom_name().map_or_else(
                    || bbb_protocol::JavaString::from(stored.entity.data().entity_resource_id()),
                    |name| {
                        bbb_server_datapack::language::component_plain_text(name.component()).into()
                    },
                ),
                stored.entity.uuid(),
                stored.entity.id(),
                stored.dimension.clone(),
                RichTextComponent::try_from_component_nbt_with_registry(
                    display_name,
                    content_registry,
                )
                .expect("entity display names are validated components"),
                permission,
            ),
        }
    }

    pub(super) fn attribute_target(self) -> CommandAttributeTarget {
        match self {
            Self::Player(player) => CommandAttributeTarget::Player(player.target()),
            Self::Stored(stored) => CommandAttributeTarget::Stored {
                dimension: stored.dimension.clone(),
                entity_id: stored.entity.id(),
            },
        }
    }

    fn source(self, world: &EntityReads<'_>) -> CommandDamageSourceEntity {
        match self {
            Self::Player(player) => CommandDamageSourceEntity::Player {
                user: player.target(),
                entity_id: player.entity_id,
                position: Vec3d {
                    x: player.position[0],
                    y: player.position[1],
                    z: player.position[2],
                },
                instabuild: player.game_type == GameType::Creative,
                may_build: bbb_server_types::block_queries::game_type_may_build(player.game_type),
                spectator: player.game_type == GameType::Spectator,
                display_name: self.display_name(world),
            },
            Self::Stored(stored) => CommandDamageSourceEntity::Stored {
                entity_id: stored.entity.id(),
                uuid: Some(stored.entity.uuid()),
                position: Some(stored.entity.position()),
                entity_resource_id: stored.entity.data().entity_resource_id().to_string(),
                is_mob: matches!(stored.entity.data(), EntityData::GeneralLiving(data) if data.is_mob()),
                is_living_non_player: stored.entity.living().is_some(),
                display_name: self.display_name(world),
            },
        }
    }

    /// `Entity.getDisplayName` (Entity.java:3207-3209); an entity whose saved
    /// name source cannot be read falls back to its type name.
    pub(super) fn display_name(self, world: &EntityReads<'_>) -> NbtSummaryValue {
        super::text::display_name(self, world).unwrap_or_else(|_| self.type_name())
    }

    /// `EntityType.getDescription` (EntityType.java:1403-1405).
    fn type_name(self) -> NbtSummaryValue {
        let (namespace, path) = self
            .type_resource_id()
            .split_once(':')
            .expect("native entity ids are canonical resource locations");
        translatable(format!("entity.{namespace}.{path}"), Vec::new())
    }
}

/// `DamageCommand.damage` (DamageCommand.java:103-110): `hurtServer` runs on
/// the live target and its answer decides between `commands.damage.success`
/// and `ERROR_INVULNERABLE`.
pub(super) fn damage_entity(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    shape: Option<DamageSourceShape>,
) -> Outcome {
    let (victim, display_name, causing, direct, amount, damage_type, source_position) =
        cx.world.with_entity_reads(|reads| {
            let at = &cx.origin;
            let target = single_damage_entity_argument(parse, "target", source, at, reads)?;
            let amount = float_argument(parse, "amount");
            // `damageSources().generic()` (DamageCommand.java:34) is
            // `DamageSources.source(DamageTypes.GENERIC)` (DamageSources.java:147-149),
            // a source with neither entity nor position.
            let damage_type = match shape {
                None => GENERIC_DAMAGE_TYPE,
                Some(_) => resource_argument(parse, "damageType"),
            };
            let source_position = match shape {
                Some(DamageSourceShape::At) => {
                    Some(coordinates_argument(parse, "location").position(at))
                }
                _ => None,
            };
            // `new DamageSource(type, entity)` fills both roles with the one entity,
            // and `new DamageSource(type, entity, cause)` puts `<entity>` in the direct
            // role and `<cause>` in the causing one (DamageSource.java:43-53).
            let (direct, causing) = match shape {
                Some(DamageSourceShape::By) => {
                    let by = single_damage_entity_argument(parse, "entity", source, at, reads)?;
                    (Some(by), Some(by))
                }
                Some(DamageSourceShape::ByFrom) => {
                    let by = single_damage_entity_argument(parse, "entity", source, at, reads)?;
                    let cause = single_damage_entity_argument(parse, "cause", source, at, reads)?;
                    (Some(by), Some(cause))
                }
                _ => (None, None),
            };
            let victim = target.attribute_target();
            let display_name = target.display_name(reads);
            let causing = causing.map(|entity| entity.source(reads));
            let direct = direct.map(|entity| entity.source(reads));
            Ok::<_, NbtSummaryValue>((
                victim,
                display_name,
                causing,
                direct,
                amount,
                damage_type,
                source_position,
            ))
        })?;
    if !cx.world.damage_by_command(
        &victim,
        damage_type,
        amount,
        causing,
        direct,
        source_position,
        cx.events,
    ) {
        return Err(translatable("commands.damage.invulnerable", Vec::new()));
    }
    // `:105`. The amount is a boxed `Float`, which `TranslatableContents`
    // serializes as the primitive it is (`PRIMITIVE_ARG_CODEC`,
    // TranslatableContents.java:32-37) rather than as a component.
    run.succeed(
        translatable(
            "commands.damage.success",
            vec![NbtSummaryValue::Float(amount.to_bits()), display_name],
        ),
        true,
    );
    Ok(1)
}

/// `KillCommand.kill` (KillCommand.java:22-34).
///
/// `Entity.kill` is not one body for every victim. For a player and for a
/// stored plain `LivingEntity` it is `LivingEntity.kill`
/// (LivingEntity.java:303-305): one `hurtServer` with `damageSources().genericKill()`
/// (DamageSources.java:271-273, built from `DamageTypes.GENERIC_KILL` at `:72`)
/// for `Float.MAX_VALUE`, so `/kill` is the hurt pipeline rather than a removal
/// of its own. Every other stored kind -- an item entity, a vehicle, and an
/// armor stand, whose `ArmorStand.kill` (ArmorStand.java:461-464) repeats
/// `Entity.kill` (Entity.java:399-402) verbatim -- takes that base body
/// instead.
///
/// The feedback does not depend on what the kill did. `kill` returns void and
/// the command reports on the victim list alone (`:27-31`), so a target the
/// kill could not touch is still reported as killed -- which is why nothing
/// here reads an outcome the way `/damage` does.
///
/// `sendSuccess(.., true)` (`:28`, `:30`): the `broadcastToAdmins` half is the
/// gap [`super::run`] names at the top of this file.
pub(super) fn kill(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    targeted: bool,
) -> Outcome {
    let victims = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let victims = if targeted {
            let Some(ArgumentValue::Players(selector)) = parse.argument("targets") else {
                unreachable!("targets is an entity argument");
            };
            let victims = select_damage_entity_states(selector, source, at, reads)?;
            if victims.is_empty() {
                return Err(translatable("argument.entity.notfound.entity", Vec::new()));
            }
            victims
        } else {
            // `ImmutableList.of(c.getSource().getEntityOrException())` (`:17`).
            vec![source_entity(source, reads)?]
        };
        Ok(victims
            .into_iter()
            .map(|victim| {
                (
                    victim.attribute_target(),
                    victim.kills_through_hurt(),
                    victim.display_name(reads),
                )
            })
            .collect::<Vec<_>>())
    })?;
    for (victim, living_kill, _) in &victims {
        cx.world.kill_by_command(victim, *living_kill, cx.events);
    }
    let count = i32::try_from(victims.len()).unwrap_or(i32::MAX);
    if victims.len() == 1 {
        run.succeed(
            translatable("commands.kill.success.single", vec![victims[0].2.clone()]),
            true,
        );
    } else {
        run.succeed(
            translatable(
                "commands.kill.success.multiple",
                vec![NbtSummaryValue::Int(count)],
            ),
            true,
        );
    }
    Ok(count)
}

/// `SwingCommand.swing` (SwingCommand.java:35-57).
pub(super) fn swing(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    targeted: bool,
    off_hand: bool,
) -> Outcome {
    let (swinging, first_name) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let targets = if targeted {
            damage_entity_targets(parse, "targets", source, at, reads)?
        } else {
            // `List.of(c.getSource().getEntityOrException())` (`:25`).
            vec![source_entity(source, reads)?]
        };
        let swinging = targets
            .iter()
            .filter_map(|target| match target {
                CommandDamageEntityRef::Stored(stored) if stored.entity.living().is_none() => None,
                _ => Some(target.attribute_target()),
            })
            .collect::<Vec<_>>();
        Ok::<_, NbtSummaryValue>((
            swinging,
            targets.first().map(|target| target.display_name(reads)),
        ))
    })?;
    let mut living = 0;
    for target in &swinging {
        cx.world
            .swing_entity_by_command(target, off_hand, cx.events);
        living += 1;
    }
    if living == 0 {
        return Err(translatable("commands.swing.failed.notliving", Vec::new()));
    }
    if living == 1 {
        run.succeed(
            translatable(
                "commands.swing.success.single",
                vec![first_name.expect("one entity swung")],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                "commands.swing.success.multiple",
                vec![NbtSummaryValue::Int(living)],
            ),
            true,
        );
    }
    Ok(living)
}

/// `RotateCommand.rotate` (RotateCommand.java:66-73 for the `<rotation>` arm
/// and `:75-79` for the three `facing ..` arms).
///
/// The two arms differ in everything but the report. The `<rotation>` one
/// resolves the argument against the *source* and then hands
/// `forceSetRotation` the difference from the *entity*'s own angle on each
/// relative axis (`:67-70`), so a `~ ~` typed by one player turns another to
/// the first one's facing. The `facing` one is `LookAt.perform` (`:76`), the
/// same call `/teleport ... facing ..` makes after it has moved something.
pub(super) fn rotate(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    facing: Option<FacingShape>,
) -> Outcome {
    enum Turn {
        LookAt(CommandLookAt),
        Rotation([f32; 2], [bool; 2]),
    }
    let (target, name, turn) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let entity = single_damage_entity_argument(parse, "target", source, at, reads)?;
        let turn = match facing {
            Some(shape) => Turn::LookAt(look_at_target(parse, source, reads, at, shape)?),
            None => {
                let rotation = rotation_argument(parse, "rotation");
                let [yaw, pitch] = rotation.rotation(at.rotation);
                let [entity_y_rot, entity_x_rot] = entity.rotation();
                let relative = [rotation.is_y_relative(), rotation.is_x_relative()];
                let yaw_delta = if relative[0] { yaw - entity_y_rot } else { yaw };
                let pitch_delta = if relative[1] {
                    pitch - entity_x_rot
                } else {
                    pitch
                };
                Turn::Rotation(
                    [
                        if yaw_delta.is_finite() {
                            yaw_delta
                        } else {
                            0.0
                        },
                        if pitch_delta.is_finite() {
                            pitch_delta
                        } else {
                            0.0
                        },
                    ],
                    relative,
                )
            }
        };
        Ok::<_, NbtSummaryValue>((entity.attribute_target(), entity.display_name(reads), turn))
    })?;
    match turn {
        Turn::LookAt(look_at) => cx.world.look_at_command_entity(&target, look_at, cx.events),
        Turn::Rotation([y_rot, x_rot], [relative_y, relative_x]) => {
            cx.world.force_set_rotation_by_command(
                &target, y_rot, relative_y, x_rot, relative_x, cx.events,
            )
        }
    }
    run.succeed(translatable("commands.rotate.success", vec![name]), true);
    Ok(1)
}

/// `SpectateCommand.spectate` (SpectateCommand.java:40-61).
pub(super) fn spectate(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    targeted: bool,
    player_argument: bool,
) -> Outcome {
    let (player, camera, report) = cx.world.with_entity_reads(|reads| {
        spectate_target(parse, source, reads, &cx.origin, targeted, player_argument)
    })?;
    cx.world
        .set_camera_by_command(&player, camera.as_ref(), cx.events);
    run.succeed(report, false);
    Ok(1)
}

fn spectate_target(
    parse: &ParseResult,
    source: &CommandSource,
    world: &crate::world::EntityReads<'_>,
    at: &CommandOrigin,
    targeted: bool,
    player_argument: bool,
) -> Result<
    (
        CommandPlayerTarget,
        Option<crate::commands::CommandCameraTarget>,
        NbtSummaryValue,
    ),
    NbtSummaryValue,
> {
    use crate::commands::CommandCameraTarget;
    let target = if targeted {
        Some(single_damage_entity_argument(
            parse, "target", source, at, world,
        )?)
    } else {
        None
    };
    let player = if player_argument {
        single_player_argument(parse, "player", source, world, at)?
    } else {
        source_player_state(source, world)
            .ok_or_else(|| translatable("permissions.requires.player", Vec::new()))?
    };
    if matches!(target, Some(CommandDamageEntityRef::Player(target)) if target.id == player.id) {
        return Err(translatable("commands.spectate.self", Vec::new()));
    }
    if player.game_type != GameType::Spectator {
        return Err(translatable(
            "commands.spectate.not_spectator",
            vec![team_player_display_name_nbt(
                &view_of(player),
                world.scoreboard,
            )],
        ));
    }
    if let Some(target) = target {
        let type_id = match target {
            CommandDamageEntityRef::Player(_) => VANILLA_ENTITY_TYPE_PLAYER_ID,
            CommandDamageEntityRef::Stored(stored) => stored.entity.entity_type_id(),
        };
        if vanilla_entity_type_client_tracking_range(type_id) == Some(0) {
            return Err(translatable(
                "commands.spectate.cannot_spectate",
                vec![target.display_name(world)],
            ));
        }
    }
    let camera = target.map(|target| match target {
        CommandDamageEntityRef::Player(player) => CommandCameraTarget::Player { uuid: player.id },
        CommandDamageEntityRef::Stored(stored) => CommandCameraTarget::Stored {
            dimension: stored.dimension.clone(),
            entity_id: stored.entity.id(),
            uuid: stored.entity.uuid(),
        },
    });
    let report = match target {
        Some(target) => translatable(
            "commands.spectate.success.started",
            vec![target.display_name(world)],
        ),
        None => translatable("commands.spectate.success.stopped", Vec::new()),
    };
    Ok((player.target(), camera, report))
}

pub(super) fn mount(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    use crate::commands::types::ExecuteRelation;
    let (target, vehicle, name, vehicle_name) = cx.world.with_entity_reads(|reads| {
        let target = single_damage_entity_argument(parse, "target", source, &cx.origin, reads)?;
        let vehicle = single_damage_entity_argument(parse, "vehicle", source, &cx.origin, reads)?;
        let target_source = target.as_command_source(source.permission);
        let name = target.display_name(reads);
        let vehicle_name = vehicle.display_name(reads);
        if let Some(current) = reads
            .related_command_entities(&target_source, ExecuteRelation::Vehicle)
            .first()
        {
            return Err(translatable(
                "commands.ride.already_riding",
                vec![name, current.display_name(reads)],
            ));
        }
        if matches!(vehicle, CommandDamageEntityRef::Player(_)) {
            return Err(translatable(
                "commands.ride.mount.failure.cant_ride_players",
                Vec::new(),
            ));
        }
        let mut ancestor = Some(vehicle);
        while let Some(current) = ancestor {
            if current.attribute_target() == target.attribute_target() {
                return Err(translatable("commands.ride.mount.failure.loop", Vec::new()));
            }
            ancestor = reads
                .related_command_entities(
                    &current.as_command_source(source.permission),
                    ExecuteRelation::Vehicle,
                )
                .into_iter()
                .next();
        }
        if target.dimension() != vehicle.dimension() {
            return Err(translatable(
                "commands.ride.mount.failure.wrong_dimension",
                Vec::new(),
            ));
        }
        let target = target.attribute_target();
        let vehicle = vehicle.attribute_target();
        Ok::<_, NbtSummaryValue>((target, vehicle, name, vehicle_name))
    })?;
    if !cx.world.mount_command_entity(&target, &vehicle, cx.events) {
        return Err(translatable(
            "commands.ride.mount.failure.generic",
            vec![name, vehicle_name],
        ));
    }
    run.succeed(
        translatable("commands.ride.mount.success", vec![name, vehicle_name]),
        true,
    );
    Ok(1)
}

pub(super) fn dismount(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let (target, name, vehicle_name) = cx.world.with_entity_reads(|reads| {
        let target = single_damage_entity_argument(parse, "target", source, &cx.origin, reads)?;
        let name = target.display_name(reads);
        let vehicle = reads.related_command_entities(
            &target.as_command_source(source.permission),
            crate::commands::types::ExecuteRelation::Vehicle,
        );
        let Some(vehicle) = vehicle.first() else {
            return Err(translatable("commands.ride.not_riding", vec![name]));
        };
        let vehicle_name = vehicle.display_name(reads);
        let target = target.attribute_target();
        Ok::<_, NbtSummaryValue>((target, name, vehicle_name))
    })?;
    cx.world.dismount_command_entity(&target, cx.events);
    run.succeed(
        translatable("commands.ride.dismount.success", vec![name, vehicle_name]),
        true,
    );
    Ok(1)
}

/// `TagCommand.addTag` (TagCommand.java:58-78) and `removeTag` (`:80-100`).
///
/// The two are one function because they differ in three names and nothing
/// else: which predicate decides a target changed, which refusal an unchanged
/// run reports, and which pair of success keys the report picks from. The
/// count and the success branch do not agree on what they count --
/// `count` is the targets that changed and the branch reads `targets.size()`
/// (`:71`, `:93`) -- so a `/tag @a add x` over two players one of which already
/// has the tag still reports the multiple form.
pub(super) fn set_tag(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
    add: bool,
) -> Outcome {
    let name = text_argument(parse, "name").clone();
    let (changing, target_count, first_name) = cx.world.with_entity_reads(|reads| {
        let at = &cx.origin;
        let targets = damage_entity_targets(parse, "targets", source, at, reads)?;
        let changing = targets
            .iter()
            .filter(|target| entity_tag_changes(**target, &name, add))
            .map(|target| target.attribute_target())
            .collect::<Vec<_>>();
        Ok::<_, NbtSummaryValue>((
            changing,
            targets.len(),
            targets.first().map(|target| target.display_name(reads)),
        ))
    })?;
    let count = i32::try_from(changing.len()).unwrap_or(i32::MAX);
    for target in &changing {
        cx.world.set_command_entity_tag(target, &name, add);
    }
    if count == 0 {
        // `ERROR_ADD_FAILED` (`:19`, thrown at `:68`) and `ERROR_REMOVE_FAILED`
        // (`:20`, thrown at `:90`).
        return Err(translatable(
            if add {
                "commands.tag.add.failed"
            } else {
                "commands.tag.remove.failed"
            },
            Vec::new(),
        ));
    }
    let name = component_literal_nbt(&name);
    if target_count == 1 {
        run.succeed(
            translatable(
                if add {
                    "commands.tag.add.success.single"
                } else {
                    "commands.tag.remove.success.single"
                },
                vec![name, first_name.expect("one target was named")],
            ),
            true,
        );
    } else {
        run.succeed(
            translatable(
                if add {
                    "commands.tag.add.success.multiple"
                } else {
                    "commands.tag.remove.success.multiple"
                },
                vec![
                    name,
                    NbtSummaryValue::Int(i32::try_from(target_count).unwrap_or(i32::MAX)),
                ],
            ),
            true,
        );
    }
    Ok(count)
}

/// `Entity.addTag` (Entity.java:391-393) and `Entity.removeTag` (`:395-397`) as
/// booleans over the projection.
///
/// `addTag` refuses a set already holding `MAX_ENTITY_TAG_COUNT` tags
/// (Entity.java:193) and a tag the set already holds, because `Set.add`
/// answers false for one it holds; `removeTag` is `Set.remove`.
fn entity_tag_changes(
    target: CommandDamageEntityRef<'_>,
    tag: &bbb_protocol::JavaString,
    add: bool,
) -> bool {
    let tags = target.entity_tags();
    let held = tags.iter().any(|entry| entry == tag);
    if add {
        tags.len() < MAX_ENTITY_TAG_COUNT && !held
    } else {
        held
    }
}

/// `TagCommand.listTags` (TagCommand.java:102-127).
///
/// The two counts the multiple form carries are different numbers: the first is
/// `targets.size()` and the second is the size of the *union* of their tags
/// (`:122`), which is also what the command returns (`:126`).
///
/// `sendSuccess(.., false)` on all four arms: a list is a query.
pub(super) fn list_tags(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    world: &mut ServerWorld,
    at: &CommandOrigin,
) -> Outcome {
    world.with_entity_reads(|reads| {
        let targets = damage_entity_targets(parse, "targets", source, at, reads)?;
        // `Sets.newHashSet()` over every target's tags (`:103-107`). The union is
        // built in first-seen order here; see [`CommandPlayer::tags`] for why
        // the order a vanilla `HashSet` would iterate in is not reproduced.
        let mut tags: Vec<&bbb_protocol::JavaString> = Vec::new();
        for target in &targets {
            for tag in target.entity_tags() {
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }
        let tag_count = i32::try_from(tags.len()).unwrap_or(i32::MAX);
        let target_count = i32::try_from(targets.len()).unwrap_or(i32::MAX);
        let formatted = || {
            format_list(
                tags.iter()
                    .map(|tag| component_literal_nbt(*tag))
                    .collect::<Vec<_>>(),
            )
        };
        if targets.len() == 1 {
            let entity = targets[0].display_name(reads);
            if tags.is_empty() {
                // `:112`.
                run.succeed(
                    translatable("commands.tag.list.single.empty", vec![entity]),
                    false,
                );
            } else {
                // `:115`.
                run.succeed(
                    translatable(
                        "commands.tag.list.single.success",
                        vec![entity, NbtSummaryValue::Int(tag_count), formatted()],
                    ),
                    false,
                );
            }
        } else if tags.is_empty() {
            // `:119`.
            run.succeed(
                translatable(
                    "commands.tag.list.multiple.empty",
                    vec![NbtSummaryValue::Int(target_count)],
                ),
                false,
            );
        } else {
            // `:122`.
            run.succeed(
                translatable(
                    "commands.tag.list.multiple.success",
                    vec![
                        NbtSummaryValue::Int(target_count),
                        NbtSummaryValue::Int(tag_count),
                        formatted(),
                    ],
                ),
                false,
            );
        }
        Ok(tag_count)
    })
}

pub(super) fn command_entity_data(
    entity: CommandDamageEntityRef<'_>,
    world: &crate::world::EntityReads<'_>,
) -> Result<Option<NbtSummaryValue>, String> {
    let uuid = match entity {
        CommandDamageEntityRef::Player(player) => player.id,
        CommandDamageEntityRef::Stored(stored) => stored.entity.uuid(),
    };
    world.entity_data(uuid)
}
