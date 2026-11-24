//! Registry-target argument readers.

use super::*;

/// `ResourceOrTagArgument.parse` (`ResourceOrTagArgument.java:66-89`).
pub(super) fn read_resource_or_tag(
    command: &[u16],
    cursor: usize,
    registry: &str,
    world: &ServerWorld,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    if registry != BIOME_REGISTRY && registry != "minecraft:point_of_interest_type" {
        return Err(ParseFailure {
            message: translatable("command.unknown.argument", Vec::new()),
            cursor: Some(cursor),
        });
    }
    if command[cursor..].starts_with(&[35]) {
        let start = cursor + 1;
        let (end, resource_id) = read_identifier_value(command, start)?;
        if !match registry {
            BIOME_REGISTRY => world.biome_tag_exists(&resource_id),
            _ => world
                .biome_tags()
                .is_some_and(|tags| tags.tag_exists(registry, &resource_id)),
        } {
            return Err(ParseFailure {
                message: translatable(
                    "argument.resource_tag.not_found",
                    vec![
                        component_literal_nbt(&resource_id),
                        component_literal_nbt(registry),
                    ],
                ),
                cursor: Some(end),
            });
        }
        return Ok((
            end,
            ArgumentValue::ResourceOrTag {
                tag: true,
                id: resource_id,
            },
        ));
    }
    let (end, resource_id) = read_identifier_value(command, cursor)?;
    if !match registry {
        BIOME_REGISTRY => world.registry_contains(BIOME_REGISTRY, &resource_id),
        _ => bbb_facts::poi_types::vanilla_poi_type_for_resource_id(&resource_id).is_some(),
    } {
        return Err(ParseFailure {
            message: translatable(
                "argument.resource.not_found",
                vec![
                    component_literal_nbt(&resource_id),
                    component_literal_nbt(registry),
                ],
            ),
            cursor: Some(end),
        });
    }
    Ok((
        end,
        ArgumentValue::ResourceOrTag {
            tag: false,
            id: resource_id,
        },
    ))
}

// ResourceOrTagKeyArgument.java:51-66: keys resolve at execution; a bad tag rewinds before '#'.
pub(super) fn read_resource_or_tag_key(
    command: &[u16],
    cursor: usize,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let tag = command[cursor..].starts_with(&[35]);
    let start = cursor + usize::from(tag);
    let (end, id) = read_identifier_value(command, start).map_err(|mut failure| {
        failure.cursor = Some(cursor);
        failure
    })?;
    Ok((end, ArgumentValue::ResourceOrTag { tag, id }))
}
