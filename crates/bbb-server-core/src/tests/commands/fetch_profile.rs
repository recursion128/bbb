use super::*;

use crate::entity::{EntityData, GeneralLivingData};
use bbb_protocol::packets::{GameProfileProperty, Vec3d};
use bbb_protocol::NbtSummaryValue;

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn string_field(value: &NbtSummaryValue, name: &str) -> Option<String> {
    match field(value, name) {
        Some(NbtSummaryValue::String(value)) => Some(value.try_to_string().unwrap()),
        _ => None,
    }
}

fn collect_click_payloads(value: &NbtSummaryValue, payloads: &mut Vec<(String, String)>) {
    match value {
        NbtSummaryValue::Compound(entries) => {
            if let Some(click) = field(value, "click_event") {
                let action = string_field(click, "action").expect("click action");
                let payload = match action.as_str() {
                    "run_command" | "suggest_command" => string_field(click, "command"),
                    "copy_to_clipboard" => string_field(click, "value"),
                    other => panic!("unexpected click action {other}"),
                }
                .expect("click payload");
                payloads.push((action.to_string(), payload.to_string()));
            }
            for entry in entries {
                if entry.name != "click_event" {
                    collect_click_payloads(&entry.value, payloads);
                }
            }
        }
        NbtSummaryValue::List(values) => {
            for value in values {
                collect_click_payloads(value, payloads);
            }
        }
        _ => {}
    }
}

fn assert_fetch_success(tick: &TickResult, profile_snbt: &str) {
    let Ok(AdminResult::Command(report)) = &admin_outcome(tick).result else {
        panic!("command.execute returns a command report");
    };
    assert_eq!(report.result, Some(1));
    assert_eq!(report.messages.len(), 1);
    let component = report.messages[0].component();
    assert_eq!(
        string_field(component, "translate").as_deref(),
        Some("commands.fetchprofile.entity.success")
    );
    let Some(NbtSummaryValue::List(arguments)) = field(component, "with") else {
        panic!("fetchprofile success arguments");
    };
    assert_eq!(arguments.len(), 2);
    let mut payloads = Vec::new();
    collect_click_payloads(&arguments[1], &mut payloads);
    assert_eq!(
        payloads,
        [
            ("copy_to_clipboard".to_string(), profile_snbt.to_string()),
            (
                "run_command".to_string(),
                format!("give @s minecraft:player_head[profile={profile_snbt}]")
            ),
            (
                "run_command".to_string(),
                format!("summon minecraft:mannequin ~ ~ ~ {{profile:{profile_snbt}}}")
            ),
            (
                "copy_to_clipboard".to_string(),
                format!("{{player:{profile_snbt}}}")
            ),
        ]
    );
}

#[test]
fn fetchprofile_entity_settles_live_players_for_command_execute() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: GameProfile {
                uuid: Uuid::from_u128(1),
                name: "alice".to_string(),
                properties: vec![GameProfileProperty {
                    name: "textures".to_string(),
                    value: "signed-value".to_string(),
                    signature: Some("signed-signature".to_string()),
                }],
            },
        },
    )
    .unwrap();
    world.tick(receiver.drain());

    let tick = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "fetchprofile entity alice",
    );
    assert_fetch_success(
        &tick,
        "{id:[I;0,0,0,1],name:\"alice\",properties:[{name:\"textures\",signature:\"signed-signature\",value:\"signed-value\"}]}",
    );
}

#[test]
fn fetchprofile_entity_distinguishes_mannequins_from_other_stored_entities() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (_, mannequin_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::GeneralLiving(
            GeneralLivingData::new(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID)
                .unwrap(),
        ),
    );
    let mannequin = block_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("fetchprofile entity {mannequin_uuid}"),
    );
    assert_fetch_success(&mannequin, "{}");

    let (_, zombie_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::GeneralLiving(
            GeneralLivingData::new(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID)
                .unwrap(),
        ),
    );
    let zombie = block_command(
        &sender,
        &receiver,
        &mut world,
        2,
        &format!("fetchprofile entity {zombie_uuid}"),
    );
    let (result, messages) = management_report(&zombie);
    assert_eq!(result, None);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].starts_with("commands.fetchprofile.no_profile("));
}
