//! `/give` (`GiveCommand.java:23-103`).

use bbb_protocol::chat_formattings::ChatFormatting;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_RARITY_TYPE_ID, VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::{
    component_appended_nbt, component_literal_nbt, component_translatable_nbt, ComponentHoverEvent,
    ComponentStyle, JavaString, NbtSummaryValue,
};

use crate::commands::item_modifier::{native_item_argument_prototype, CommandComponentPatchEntry};
use crate::commands::parse::{ArgumentValue, ParseResult};
use crate::commands::{
    chat_formatting_style, team_player_display_name_nbt, translatable, CommandCx, CommandPlayer,
    CommandSource,
};
use crate::world::ServerWorld;

use super::{integer_argument, player_states_argument, view_of, CommandRun, Outcome};

/// `GiveCommand.MAX_ALLOWED_ITEMSTACKS` (`GiveCommand.java:21`).
const MAX_ALLOWED_ITEMSTACKS: i32 = 100;

/// `GiveCommand.giveItem` (`GiveCommand.java:48-103`).
pub(super) fn give_items(
    run: &mut CommandRun,
    parse: &ParseResult,
    source: &CommandSource,
    cx: &mut CommandCx<'_>,
) -> Outcome {
    let (players, target) = cx.world.with_entity_reads(|reads| {
        let players = player_states_argument(parse, "targets", source, reads, &cx.origin)?;
        let target = if players.len() == 1 {
            team_player_display_name_nbt(&view_of(players[0]), reads.scoreboard)
        } else {
            NbtSummaryValue::Int(i32::try_from(players.len()).unwrap_or(i32::MAX))
        };
        Ok::<_, NbtSummaryValue>((
            players
                .iter()
                .map(CommandPlayer::target)
                .collect::<Vec<_>>(),
            target,
        ))
    })?;
    let world: &ServerWorld = cx.world;
    let (_, resource_id, patch) = item_stack_argument(parse);
    let count = parse
        .argument("count")
        .map(|_| integer_argument(parse, "count"))
        .unwrap_or(1);
    // `createItemStack(1)` (`GiveCommand.java:49`) builds the prototype the
    // patch is part of, and `:50-51` reads the max stack size off it.
    let resolver = world.item_stack_resolver();
    let prototype = native_item_argument_prototype(&resource_id, &patch, &resolver)?;
    let max_stack = prototype
        .max_stack_size()
        .map_err(|error| component_literal_nbt(error.to_string()))?;
    let max_allowed = max_stack.saturating_mul(MAX_ALLOWED_ITEMSTACKS);
    let display = runtime_item_stack_display_name(&prototype, resolver.as_dyn())?;
    if count > max_allowed {
        run.fail(translatable(
            "commands.give.failed.toomanyitems",
            vec![NbtSummaryValue::Int(max_allowed), display],
        ));
        return Ok(0);
    }
    let feedback = translatable(
        "commands.give.success.single",
        vec![NbtSummaryValue::Int(count), display, target],
    );
    for player in &players {
        cx.world
            .give_command_items(player, &prototype, count, cx.events)?;
    }
    run.succeed(feedback, true);
    Ok(i32::try_from(players.len()).unwrap_or(i32::MAX))
}

/// `ItemArgument.getItem` (`ItemArgument.java:26-28`): the id, its resource
/// name and the component patch the argument read.
pub(in crate::commands::run) fn item_stack_argument(
    parse: &ParseResult,
) -> (i32, String, Vec<CommandComponentPatchEntry>) {
    match parse.argument("item") {
        Some(ArgumentValue::ItemStack {
            item_id,
            resource_id,
            patch,
        }) => (*item_id, resource_id.clone(), patch.clone()),
        other => unreachable!("item is an item_stack argument, not {other:?}"),
    }
}

// ItemStack.java:804-824: custom name, book title, then the concrete item's name.
pub(super) fn runtime_item_stack_hover_name(
    stack: &bbb_server_types::item_stack::RuntimeItemStack,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Result<NbtSummaryValue, NbtSummaryValue> {
    if stack.is_empty() || stack.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_AIR_ID) {
        return Ok(component_literal_nbt(""));
    }
    let raw = stack
        .write(resolver)
        .map_err(|error| component_literal_nbt(error.to_string()))?
        .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new()));
    let component = |type_id| {
        stack
            .component_nbt(type_id, resolver)
            .map_err(|error| component_literal_nbt(error.to_string()))
    };
    if let Some(name) = component(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID)? {
        return Ok(name);
    }
    if let Some(title) = component(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)?
        .as_ref()
        .and_then(written_book_title)
    {
        return Ok(component_literal_nbt(title));
    }
    super::selector::item_name_from_nbt_with_registry(&raw, resolver.content_registry())
}

// ItemStack.java:1019-1031; ItemStackTemplate.java:62-68.
pub(crate) fn runtime_item_stack_display_name(
    stack: &bbb_server_types::item_stack::RuntimeItemStack,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Result<NbtSummaryValue, NbtSummaryValue> {
    if stack.is_empty() || stack.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_AIR_ID) {
        return empty_stack_display_name();
    }
    let raw = stack
        .write(resolver)
        .map_err(|error| component_literal_nbt(error.to_string()))?
        .unwrap_or_else(|| NbtSummaryValue::Compound(Vec::new()));
    let component = |type_id| {
        stack
            .component_nbt(type_id, resolver)
            .map_err(|error| component_literal_nbt(error.to_string()))
    };
    // ItemStack.java:804-824: custom name, book title, then the concrete item's name.
    let custom = component(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID)?;
    let mut name_style = ComponentStyle::default();
    let name = if let Some(name) = custom {
        name_style.italic = Some(true);
        name
    } else if let Some(title) = component(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)?
        .as_ref()
        .and_then(written_book_title)
    {
        component_literal_nbt(title)
    } else {
        super::selector::item_name_from_nbt_with_registry(&raw, resolver.content_registry())?
    };
    let appended = component_appended_nbt(&name_style, vec![name])
        .expect("item names are validated components");
    let rarity = component(VANILLA_DATA_COMPONENT_RARITY_TYPE_ID)?;
    let rarity = match rarity.as_ref() {
        Some(NbtSummaryValue::String(value)) => ["common", "uncommon", "rare", "epic"]
            .into_iter()
            .find(|known| value == *known)
            .unwrap_or("common"),
        _ => "common",
    };
    let enchanted = matches!(component(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)?, Some(NbtSummaryValue::Compound(ref values)) if !values.is_empty());
    let color = match (rarity, enchanted) {
        ("common" | "uncommon", true) | ("rare", false) => ChatFormatting::Aqua,
        ("rare", true) | ("epic", _) => ChatFormatting::LightPurple,
        ("uncommon", false) => ChatFormatting::Yellow,
        _ => ChatFormatting::White,
    };
    let components = match &raw {
        NbtSummaryValue::Compound(fields) => fields
            .iter()
            .find(|field| field.name == "components")
            .map(|field| field.value.clone())
            .filter(
                |value| !matches!(value, NbtSummaryValue::Compound(fields) if fields.is_empty()),
            ),
        _ => None,
    };
    let mut style = chat_formatting_style(color);
    style.hover_event = Some(ComponentHoverEvent::ShowItem {
        id: stack
            .item_resource_id_with_registry(resolver.content_registry())
            .unwrap_or("minecraft:air")
            .to_owned(),
        count: stack.count(),
        components,
    });
    Ok(
        component_translatable_nbt("chat.square_brackets", vec![appended], &style)
            .expect("item display styles are representable"),
    )
}

fn empty_stack_display_name() -> Result<NbtSummaryValue, NbtSummaryValue> {
    // ItemStack.java:220-222: an empty stack exposes no components.
    let name = component_appended_nbt(&ComponentStyle::default(), vec![component_literal_nbt("")])
        .expect("the empty name is a component");
    Ok(component_translatable_nbt(
        "chat.square_brackets",
        vec![name],
        &ComponentStyle::default(),
    )
    .expect("the empty item display is a component"))
}

fn written_book_title(book: &NbtSummaryValue) -> Option<JavaString> {
    let NbtSummaryValue::Compound(fields) = book else {
        return None;
    };
    let title = &fields.iter().find(|field| field.name == "title")?.value;
    let raw = match title {
        NbtSummaryValue::Compound(fields) => {
            &fields.iter().find(|field| field.name == "raw")?.value
        }
        title => title,
    };
    JavaString::from_nbt(raw).filter(|title| !title.is_minecraft_blank())
}
