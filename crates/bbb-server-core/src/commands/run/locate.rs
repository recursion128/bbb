//! `/locate`.

use bbb_protocol::{
    chat_formattings::ChatFormatting, component_translatable_nbt, packets::BlockPos,
    ComponentClickEvent, ComponentHoverEvent, ComponentStyle, NbtSummaryValue,
};

use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{chat_formatting_style, translatable, CommandOrigin};
use crate::world::ServerWorld;

use super::{CommandCx, CommandRun, Outcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocateKind {
    Biome,
    Structure,
    Poi,
}

impl LocateKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Biome => "biome",
            Self::Structure => "structure",
            Self::Poi => "poi",
        }
    }

    pub(crate) fn registry(self) -> &'static str {
        match self {
            Self::Biome => "minecraft:worldgen/biome",
            Self::Structure => "minecraft:worldgen/structure",
            Self::Poi => "minecraft:point_of_interest_type",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LocateRequest {
    pub(crate) kind: LocateKind,
    pub(crate) level: bbb_server_types::save::Java26LevelKey,
    pub(crate) origin: BlockPos,
    pub(crate) target_display: String,
    pub(crate) allowed_ids: Vec<String>,
    pub(crate) target_is_tag: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LocateSearchResult {
    Found { pos: BlockPos, resource_id: String },
    NotFound,
    Unavailable,
}

// LocateCommand.java:94-136: structure keys resolve during execution, biome/POI during parse.
pub(super) fn locate(
    run: &mut CommandRun,
    parse: &ParseResult,
    cx: &mut CommandCx<'_>,
    kind: &str,
) -> Outcome {
    let request = locate_request(parse, cx.world, &cx.origin, kind)?;
    let result = cx.world.locate_live_target(&request);
    report_search(run, &request, result)
}

fn locate_request(
    parse: &ParseResult,
    world: &ServerWorld,
    at: &CommandOrigin,
    kind: &str,
) -> Result<LocateRequest, NbtSummaryValue> {
    let kind = match kind {
        "biome" => LocateKind::Biome,
        "structure" => LocateKind::Structure,
        "poi" => LocateKind::Poi,
        _ => unreachable!(),
    };
    let Some(ArgumentValue::ResourceOrTag { tag, id }) = parse.argument(kind.name()) else {
        unreachable!("locate takes a resource-or-tag argument");
    };
    let target_display = if *tag { format!("#{id}") } else { id.clone() };
    let allowed_ids = if kind == LocateKind::Biome {
        if *tag {
            world
                .registry_ids("minecraft:worldgen/biome")
                .into_iter()
                .filter(|biome| world.biome_in_tag(id, biome))
                .collect()
        } else {
            vec![id.clone()]
        }
    } else if *tag {
        match world.biome_tags() {
            Some(tags) if tags.tag_exists(kind.registry(), id) => tags
                .members(kind.registry(), id)
                .iter()
                .map(|id| id.to_string())
                .collect(),
            _ => {
                return Err(translatable(
                    "commands.locate.structure.invalid",
                    vec![NbtSummaryValue::string(target_display)],
                ))
            }
        }
    } else {
        if kind == LocateKind::Structure
            && !world
                .startup_registry_ids("minecraft:worldgen/structure")
                .iter()
                .any(|known| known == id)
        {
            return Err(translatable(
                "commands.locate.structure.invalid",
                vec![NbtSummaryValue::string(target_display)],
            ));
        }
        vec![id.clone()]
    };
    Ok(LocateRequest {
        kind,
        level: at.level.clone(),
        origin: BlockPos {
            x: at.position[0].floor() as i32,
            y: at.position[1].floor() as i32,
            z: at.position[2].floor() as i32,
        },
        target_display,
        allowed_ids,
        target_is_tag: *tag,
    })
}

// LocateCommand.java:138-193. A search the world could not run (an unknown
// biome id, a chunk that failed to load) reports as nothing found.
fn report_search(
    run: &mut CommandRun,
    request: &LocateRequest,
    result: LocateSearchResult,
) -> Outcome {
    match result {
        LocateSearchResult::Found { pos, resource_id } => {
            let include_y = request.kind == LocateKind::Biome;
            let distance = distance(request.origin, pos, include_y);
            let target = if request.target_is_tag {
                format!("{} ({resource_id})", request.target_display)
            } else {
                request.target_display.clone()
            };
            run.succeed(
                translatable(
                    format!("commands.locate.{}.success", request.kind.name()),
                    vec![
                        NbtSummaryValue::string(target),
                        coordinates(pos, include_y),
                        NbtSummaryValue::Int(distance),
                    ],
                ),
                false,
            );
            Ok(distance)
        }
        LocateSearchResult::NotFound | LocateSearchResult::Unavailable => Err(translatable(
            format!("commands.locate.{}.not_found", request.kind.name()),
            vec![NbtSummaryValue::string(request.target_display.clone())],
        )),
    }
}

fn distance(origin: BlockPos, found: BlockPos, include_y: bool) -> i32 {
    if !include_y {
        let dx = found.x.wrapping_sub(origin.x);
        let dz = found.z.wrapping_sub(origin.z);
        return (dx.wrapping_mul(dx).wrapping_add(dz.wrapping_mul(dz)) as f32)
            .sqrt()
            .floor() as i32;
    }
    let dx = f64::from(origin.x) - f64::from(found.x);
    let dy = f64::from(origin.y) - f64::from(found.y);
    let dz = f64::from(origin.z) - f64::from(found.z);
    ((dx * dx + dy * dy + dz * dz) as f32).sqrt().floor() as i32
}

fn coordinates(pos: BlockPos, include_y: bool) -> NbtSummaryValue {
    let y = if include_y {
        pos.y.to_string()
    } else {
        "~".to_string()
    };
    let inner = translatable(
        "chat.coordinates",
        vec![
            NbtSummaryValue::Int(pos.x),
            NbtSummaryValue::string(y.clone()),
            NbtSummaryValue::Int(pos.z),
        ],
    );
    component_translatable_nbt(
        "chat.square_brackets",
        vec![inner],
        &ComponentStyle {
            click_event: Some(ComponentClickEvent::SuggestCommand {
                command: format!("/tp @s {} {} {}", pos.x, y, pos.z).into(),
            }),
            hover_event: Some(ComponentHoverEvent::ShowText {
                value: Vec::new(),
                component: Some(translatable("chat.coordinates.tooltip", Vec::new())),
            }),
            ..chat_formatting_style(ChatFormatting::Green)
        },
    )
    .expect("locate coordinates are representable")
}
