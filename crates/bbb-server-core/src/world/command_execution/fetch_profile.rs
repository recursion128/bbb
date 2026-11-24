use crate::entity::EntityData;
use bbb_protocol::nbt_summary::entry;
use bbb_protocol::packets::GameProfile;
use bbb_protocol::{
    component_hover_uuid_nbt, normalize_resource_location, JavaString, NbtSummaryEntry,
    NbtSummaryValue,
};

use crate::commands::{FetchProfileResult, FetchProfileTarget};
use bbb_protocol::player_names::valid_player_name;

use super::super::display_names::{player_display_name, stored_display_name};
use super::super::ServerWorld;

impl ServerWorld {
    /// `FetchProfileCommand.printForEntity` (`FetchProfileCommand.java:153-164`).
    pub(crate) fn fetch_profile(&self, target: &FetchProfileTarget) -> FetchProfileResult {
        match target {
            FetchProfileTarget::Player { uuid } => {
                let Some(player) = self
                    .levels
                    .players()
                    .find(|player| player.profile.uuid == *uuid)
                else {
                    return FetchProfileResult::Unavailable;
                };
                FetchProfileResult::Avatar {
                    profile: profile_nbt(&player.profile),
                    display_name: player_display_name(
                        &player.profile,
                        self.scoreboard.board().players_team(&player.profile.name),
                    ),
                }
            }
            FetchProfileTarget::Stored {
                dimension,
                entity_id,
                uuid,
            } => {
                let Some(entity) = self
                    .level_by_id(&self.require_level_id(dimension))
                    .entities
                    .get(*entity_id)
                else {
                    return FetchProfileResult::Unavailable;
                };
                if entity.uuid() != *uuid {
                    return FetchProfileResult::Unavailable;
                }
                let display_name = stored_display_name(
                    entity,
                    self.scoreboard.board().players_team(&uuid.to_string()),
                );
                match entity.data() {
                    EntityData::GeneralLiving(data) => match data.mannequin_profile() {
                        Some(profile) => match normalize_profile(profile) {
                            Some(profile) => FetchProfileResult::Avatar {
                                profile,
                                display_name,
                            },
                            None => FetchProfileResult::Unavailable,
                        },
                        None => FetchProfileResult::NotAvatar { display_name },
                    },
                    _ => FetchProfileResult::NotAvatar { display_name },
                }
            }
        }
    }
}

pub(in crate::world) fn profile_nbt(profile: &GameProfile) -> NbtSummaryValue {
    let mut fields = vec![
        entry("id", component_hover_uuid_nbt(profile.uuid)),
        entry("name", string(&profile.name)),
    ];
    if !profile.properties.is_empty() {
        fields.push(entry(
            "properties",
            NbtSummaryValue::List(
                profile
                    .properties
                    .iter()
                    .map(|property| {
                        let mut fields = vec![
                            entry("name", string(&property.name)),
                            entry("value", string(&property.value)),
                        ];
                        if let Some(signature) = &property.signature {
                            fields.push(entry("signature", string(signature)));
                        }
                        NbtSummaryValue::Compound(fields)
                    })
                    .collect(),
            ),
        ));
    }
    NbtSummaryValue::Compound(fields)
}

/// `ResolvableProfile.CODEC` decoded and encoded through its `FULL_CODEC`
/// (`ResolvableProfile.java:27-35`).
fn normalize_profile(profile: &NbtSummaryValue) -> Option<NbtSummaryValue> {
    match profile {
        NbtSummaryValue::String(name) => {
            let name = name.try_to_string().ok()?;
            valid_player_name(&name).then(|| compound(vec![entry("name", string(&name))]))
        }
        NbtSummaryValue::Compound(fields) => normalize_profile_fields(fields),
        _ => None,
    }
}

fn normalize_profile_fields(fields: &[NbtSummaryEntry]) -> Option<NbtSummaryValue> {
    let field = |name: &str| {
        fields
            .iter()
            .rev()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    };
    let name = match field("name") {
        Some(NbtSummaryValue::String(name)) => {
            let name = name.try_to_string().ok()?;
            if !valid_player_name(&name) {
                return None;
            }
            Some(name)
        }
        Some(_) => return None,
        None => None,
    };
    let id = match field("id") {
        Some(NbtSummaryValue::IntArray(id)) if id.len() == 4 => Some(id.clone()),
        Some(_) => return None,
        None => None,
    };
    let properties = match field("properties") {
        Some(properties) => Some(normalize_properties(properties)?),
        None => None,
    };
    let mut patch = Vec::new();
    for name in ["texture", "cape", "elytra"] {
        match field(name) {
            Some(NbtSummaryValue::String(value)) => patch.push(entry(
                name,
                string(&normalize_resource_location(value.try_to_string().ok()?).ok()?),
            )),
            Some(_) => return None,
            None => {}
        }
    }
    match field("model") {
        Some(NbtSummaryValue::String(model)) if (model == "slim" || model == "wide") => {
            patch.push(entry("model", string(model)));
        }
        Some(_) => return None,
        None => {}
    }

    let mut normalized = Vec::with_capacity(7);
    match (id, name) {
        (Some(id), Some(name)) => {
            normalized.push(entry("id", NbtSummaryValue::IntArray(id)));
            normalized.push(entry("name", string(name)));
        }
        (id, name) => {
            if let Some(name) = name {
                normalized.push(entry("name", string(&name)));
            }
            if let Some(id) = id {
                normalized.push(entry("id", NbtSummaryValue::IntArray(id)));
            }
        }
    }
    if let Some(properties) = properties.filter(|properties| !properties.is_empty()) {
        normalized.push(entry("properties", NbtSummaryValue::List(properties)));
    }
    normalized.extend(patch);
    Some(compound(normalized))
}

fn normalize_properties(properties: &NbtSummaryValue) -> Option<Vec<NbtSummaryValue>> {
    let normalized = match properties {
        NbtSummaryValue::List(properties) => properties
            .iter()
            .map(normalize_property)
            .collect::<Option<Vec<_>>>()?,
        NbtSummaryValue::Compound(fields) => {
            let mut normalized = Vec::new();
            for (index, field) in fields.iter().enumerate() {
                if fields[index + 1..]
                    .iter()
                    .any(|later| later.name == field.name)
                {
                    continue;
                }
                let NbtSummaryValue::List(values) = &field.value else {
                    return None;
                };
                for value in values {
                    let NbtSummaryValue::String(value) = value else {
                        return None;
                    };
                    normalized.push(profile_property(&field.name, value, None)?);
                }
            }
            normalized
        }
        _ => return None,
    };
    (normalized.len() <= 16).then_some(normalized)
}

fn normalize_property(property: &NbtSummaryValue) -> Option<NbtSummaryValue> {
    let NbtSummaryValue::Compound(fields) = property else {
        return None;
    };
    let string_field = |name: &str| {
        fields
            .iter()
            .rev()
            .find(|entry| entry.name == name)
            .and_then(|entry| match &entry.value {
                NbtSummaryValue::String(value) => Some(value),
                _ => None,
            })
    };
    let signature = match fields
        .iter()
        .rev()
        .find(|entry| entry.name == "signature")
        .map(|entry| &entry.value)
    {
        Some(NbtSummaryValue::String(signature)) => Some(signature),
        Some(_) => return None,
        None => None,
    };
    profile_property(string_field("name")?, string_field("value")?, signature)
}

fn profile_property(
    name: &JavaString,
    value: &JavaString,
    signature: Option<&JavaString>,
) -> Option<NbtSummaryValue> {
    if name.units().len() > 64 || value.units().len() > 32_767 {
        return None;
    }
    if signature.is_some_and(|signature| signature.units().len() > 1_024) {
        return None;
    }
    let mut fields = vec![entry("name", string(name)), entry("value", string(value))];
    if let Some(signature) = signature {
        fields.push(entry("signature", string(signature)));
    }
    Some(compound(fields))
}

fn string(value: impl Into<JavaString>) -> NbtSummaryValue {
    NbtSummaryValue::string(value)
}

fn compound(fields: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(fields)
}

#[cfg(test)]
mod tests {
    use crate::entity::GeneralLivingData;
    use crate::world::scoreboard::PlayerTeam;
    use bbb_protocol::chat_formattings::ChatFormatting;
    use bbb_protocol::component_literal_nbt;
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };
    use bbb_protocol::packets::{GameProfileProperty, Vec3d};
    use bbb_server_types::save::Java26BuiltinDimension;
    use uuid::Uuid;

    use super::*;
    use crate::world::scoreboard::Scoreboard;
    use crate::world::server_scoreboard::ServerScoreboard;

    fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
        let NbtSummaryValue::Compound(fields) = value else {
            return None;
        };
        fields
            .iter()
            .rev()
            .find(|field| field.name == name)
            .map(|field| &field.value)
    }

    fn click(command: &str) -> NbtSummaryValue {
        compound(vec![
            entry("action", string("run_command")),
            entry("command", string(command)),
        ])
    }

    fn property(name: &str, value: &str, signature: Option<&str>) -> NbtSummaryValue {
        profile_property(
            &name.into(),
            &value.into(),
            signature.map(JavaString::from).as_ref(),
        )
        .expect("valid property")
    }

    #[test]
    fn profile_normalization_rewrites_properties_and_skin_patch() {
        let signed = property("textures", "value", Some("signature"));
        assert_eq!(
            normalize_profile(&compound(vec![
                entry("name", string("Alex")),
                entry("properties", NbtSummaryValue::List(vec![signed.clone()])),
                entry("texture", string("skins/alex")),
                entry("cape", string("minecraft:capes/red")),
                entry("elytra", string("elytra/red")),
                entry("model", string("slim")),
            ])),
            Some(compound(vec![
                entry("name", string("Alex")),
                entry("properties", NbtSummaryValue::List(vec![signed])),
                entry("texture", string("minecraft:skins/alex")),
                entry("cape", string("minecraft:capes/red")),
                entry("elytra", string("minecraft:elytra/red")),
                entry("model", string("slim")),
            ]))
        );

        assert_eq!(
            normalize_profile(&compound(vec![entry(
                "properties",
                compound(vec![
                    entry(
                        "textures",
                        NbtSummaryValue::List(vec![string("old"), string("ignored")]),
                    ),
                    entry("textures", NbtSummaryValue::List(vec![string("winner")]),),
                    entry("cape", NbtSummaryValue::List(vec![string("red")])),
                ]),
            )])),
            Some(compound(vec![entry(
                "properties",
                NbtSummaryValue::List(vec![
                    property("textures", "winner", None),
                    property("cape", "red", None),
                ]),
            )]))
        );
    }

    #[test]
    fn live_game_profile_writer_keeps_signed_properties() {
        let profile = GameProfile {
            uuid: Uuid::from_u128(0x00000001000000020000000300000004),
            name: "alice".to_string(),
            properties: vec![GameProfileProperty {
                name: "textures".to_string(),
                value: "value".to_string(),
                signature: Some("signature".to_string()),
            }],
        };
        assert_eq!(
            profile_nbt(&profile),
            compound(vec![
                entry("id", NbtSummaryValue::IntArray(vec![1, 2, 3, 4])),
                entry("name", string("alice")),
                entry(
                    "properties",
                    NbtSummaryValue::List(vec![property("textures", "value", Some("signature"),)]),
                ),
            ])
        );
    }

    #[test]
    fn display_names_apply_team_and_entity_root_actions() {
        let profile = GameProfile {
            uuid: Uuid::from_u128(1),
            name: "alice".to_string(),
            properties: Vec::new(),
        };
        let mut team = PlayerTeam::new("red");
        team.player_prefix = Some(component_literal_nbt("[R]"));
        team.player_suffix = Some(component_literal_nbt("!"));
        team.color = ChatFormatting::Red;
        let display = player_display_name(&profile, Some(&team));
        assert_eq!(field(&display, "color"), Some(&string("red")));
        assert_eq!(
            field(&display, "extra"),
            Some(&NbtSummaryValue::List(vec![
                string("[R]"),
                string("alice"),
                string("!"),
            ]))
        );
        assert_eq!(field(&display, "insertion"), Some(&string("alice")));
        let click = field(&display, "click_event").expect("player click");
        assert_eq!(field(click, "command"), Some(&string("/tell alice ")));
        let hover = field(&display, "hover_event").expect("player hover");
        assert_eq!(field(hover, "id"), Some(&string("minecraft:player")));
        assert_eq!(
            field(hover, "uuid"),
            Some(&component_hover_uuid_nbt(Uuid::from_u128(1)))
        );
        assert_eq!(field(hover, "name"), Some(&string("alice")));
    }

    #[test]
    fn custom_name_click_removal_only_descends_through_siblings() {
        let argument = compound(vec![
            entry("text", string("argument")),
            entry("click_event", click("/argument")),
        ]);
        let sibling = compound(vec![
            entry("translate", string("example.key")),
            entry("with", NbtSummaryValue::List(vec![argument])),
            entry("click_event", click("/sibling")),
        ]);
        let hover_value = compound(vec![
            entry("text", string("hover")),
            entry("click_event", click("/hover")),
        ]);
        let name = compound(vec![
            entry("text", string("root")),
            entry("click_event", click("/root")),
            entry("extra", NbtSummaryValue::List(vec![sibling])),
            entry(
                "hover_event",
                compound(vec![
                    entry("action", string("show_text")),
                    entry("value", hover_value),
                ]),
            ),
        ]);
        let stripped = crate::world::display_names::remove_sibling_click_actions(&name);
        assert_eq!(field(&stripped, "click_event"), None);
        let NbtSummaryValue::List(siblings) = field(&stripped, "extra").unwrap() else {
            panic!("extra is a list")
        };
        assert_eq!(field(&siblings[0], "click_event"), None);
        let NbtSummaryValue::List(arguments) = field(&siblings[0], "with").unwrap() else {
            panic!("translation arguments are a list")
        };
        assert!(field(&arguments[0], "click_event").is_some());
        let hover = field(&stripped, "hover_event").unwrap();
        assert!(field(field(hover, "value").unwrap(), "click_event").is_some());
    }

    #[test]
    fn stored_resolver_validates_target_identity_and_avatar_kind() {
        let mut world = ServerWorld::new(Default::default()).expect("world");
        let (mannequin_id, mannequin_uuid) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_MANNEQUIN_ID).unwrap(),
            ),
        );
        assert_eq!(
            world.fetch_profile(&FetchProfileTarget::Stored {
                dimension: Java26BuiltinDimension::Overworld.into(),
                entity_id: mannequin_id,
                uuid: mannequin_uuid,
            }),
            FetchProfileResult::Avatar {
                profile: compound(Vec::new()),
                display_name: stored_display_name(
                    world
                        .level(Java26BuiltinDimension::Overworld)
                        .entities
                        .get(mannequin_id)
                        .unwrap(),
                    None,
                ),
            }
        );
        for target in [
            FetchProfileTarget::Stored {
                dimension: Java26BuiltinDimension::Nether.into(),
                entity_id: mannequin_id,
                uuid: mannequin_uuid,
            },
            FetchProfileTarget::Stored {
                dimension: Java26BuiltinDimension::Overworld.into(),
                entity_id: mannequin_id + 99,
                uuid: mannequin_uuid,
            },
            FetchProfileTarget::Stored {
                dimension: Java26BuiltinDimension::Overworld.into(),
                entity_id: mannequin_id,
                uuid: Uuid::from_u128(99),
            },
        ] {
            assert_eq!(
                world.fetch_profile(&target),
                FetchProfileResult::Unavailable
            );
        }

        let (zombie_id, zombie_uuid) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
        );
        let mut board = Scoreboard::default();
        assert!(board.add_team("red"));
        let team = board.team_mut("red").unwrap();
        team.player_prefix = Some(string("[R]"));
        team.player_suffix = Some(string("!"));
        team.color = ChatFormatting::Red;
        board.add_player_to_team(&zombie_uuid.to_string(), "red");
        world.scoreboard = ServerScoreboard::restore(&board.pack());
        let FetchProfileResult::NotAvatar { display_name } =
            world.fetch_profile(&FetchProfileTarget::Stored {
                dimension: Java26BuiltinDimension::Overworld.into(),
                entity_id: zombie_id,
                uuid: zombie_uuid,
            })
        else {
            panic!("zombie is not an avatar")
        };
        assert_eq!(field(&display_name, "color"), Some(&string("red")));
        let NbtSummaryValue::List(parts) = field(&display_name, "extra").unwrap() else {
            panic!("team display has parts")
        };
        assert_eq!(parts[0], string("[R]"));
        assert_eq!(
            field(&parts[1], "translate"),
            Some(&string("entity.minecraft.zombie"))
        );
        assert_eq!(parts[2], string("!"));
        let hover = field(&display_name, "hover_event").expect("entity hover");
        assert_eq!(field(hover, "id"), Some(&string("minecraft:zombie")));
        assert_eq!(
            field(field(hover, "name").unwrap(), "translate"),
            Some(&string("entity.minecraft.zombie"))
        );
        assert_eq!(
            field(&display_name, "insertion"),
            Some(&string(zombie_uuid.to_string()))
        );
    }
}
