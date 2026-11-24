//! The player inventory and the `InventoryMenu` slot layout addressed over it.
//!
//! Two different slot numberings meet here and neither is a renaming of the
//! other. `Inventory` (Inventory.java:28-53) numbers 36 storage slots and then
//! the equipment slots its `EQUIPMENT_SLOT_MAPPING` places above them — the
//! generated catalog already carries those positions as
//! `player_inventory_slot`. `InventoryMenu` (InventoryMenu.java:14-27) numbers
//! the 46 slots a *client* addresses, in a different order and with four
//! crafting slots and a result slot that belong to the menu rather than to the
//! inventory. Every packet that names a slot names one of the two numberings,
//! so the projection between them lives here instead of at each call site.
//!
//! One `inventory/<domain>.rs` per domain (slot layout, inventory half, menu clicks,
//! remote sync, container menus, item rules); this file keeps the types and constants.

mod bundle;
pub mod container_menu;
mod crafter_menu;
pub mod crafting;
mod ender_chest;
pub mod item_rules;
pub mod menu_click;
pub mod menu_layout;
pub mod menu_sync;
mod merchant_menu;
pub mod mount_menu;
pub mod player_inventory;
mod recipe_placement;
pub mod utility_menu;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use bbb_protocol::{
    equipment_slots::EquipmentSlot,
    packets::{BlockPos, HashedStack, ItemStackSummary},
};
use bbb_server_types::Result;

use crafting::InventoryCrafting;

use bbb_server_types::item_stack::RuntimeItemStack;
pub use bundle::{bundle_remove_one, BundleSound};
pub use crafting::{recipe_book_from_id_lists, RecipeBook};
pub use menu_sync::{MenuBroadcastCursor, MenuBroadcastEvent, MenuBroadcastMode};
pub use merchant_menu::{MerchantMenuEvent, MerchantTrade, MerchantTradeCost};
pub use recipe_placement::PostPlaceAction;
pub use utility_menu::{
    EnchantmentOffers, MapFacts, UtilityMenuEvent, UtilityMenuFacts, UtilityMenuKind,
    UtilityPlayerFacts, ENCHANTMENT_OFFER_COUNT,
};

pub use item_rules::{
    adventure_mode_predicate_matches, adventure_mode_predicate_matches_with_registry, attack_range,
    can_destroy_block, has_component, has_piercing_weapon, is_same_item_same_components,
    item_damage_per_block, item_destroy_speed, item_is_correct_tool_for_drops,
    item_prototype_int_component, jukebox_playable_song, main_hand_attack_attributes,
    main_hand_attack_damage_type, max_stack_size, minimum_attack_charge,
    weapon_item_damage_per_attack, MainHandAttackAttributes, MainHandAttackDamageType,
};

/// `Inventory.INVENTORY_SIZE` (Inventory.java:30).
pub const INVENTORY_SIZE: usize = 36;
/// `Inventory.SELECTION_SIZE` (Inventory.java:31).
pub const SELECTION_SIZE: usize = 9;
pub use bbb_server_types::slot_ranges::ENDER_CHEST_SIZE;

/// `InventoryMenu.CONTAINER_ID` (InventoryMenu.java:14): the player inventory
/// is the one menu that is always open, and it is always container zero.
pub const MENU_CONTAINER_ID: i32 = 0;
/// `InventoryMenu.RESULT_SLOT` … `SHIELD_SLOT` (InventoryMenu.java:15-27).
const MENU_RESULT_SLOT: i16 = 0;
const MENU_CRAFT_SLOT_START: i16 = 1;
const MENU_CRAFT_SLOT_END: i16 = 5;
const MENU_ARMOR_SLOT_START: i16 = 5;
const MENU_ARMOR_SLOT_END: i16 = 9;
const MENU_INV_SLOT_START: i16 = 9;
const MENU_INV_SLOT_END: i16 = 36;
const MENU_USE_ROW_SLOT_START: i16 = 36;
const MENU_USE_ROW_SLOT_END: i16 = 45;
const MENU_SHIELD_SLOT: i16 = 45;
/// One past the last menu slot, which is what `sendAllDataToRemote` walks.
pub const MENU_SLOT_COUNT: i16 = MENU_SHIELD_SLOT + 1;

/// `AbstractContainerMenu.SLOTS_PER_ROW` (:48), the width every generic
/// container grid and the player's own rows are laid out in.
const SLOTS_PER_ROW: i16 = 9;
/// The slots `addStandardInventorySlots` (AbstractContainerMenu.java:86-91)
/// contributes: `addInventoryExtendedSlots`' three rows (:78-84) followed by
/// `addInventoryHotbarSlots`' one (:72-76).
const STANDARD_INVENTORY_SLOT_COUNT: i16 = 4 * SLOTS_PER_ROW;
/// `ChestBlockEntity.items` (`ChestBlockEntity.java:31`): the literal 27 the
/// field is sized with, which `getContainerSize` (`:73-75`) repeats.
pub const CHEST_CONTAINER_SIZE: usize = 27;
/// `ChestMenu.threeRows` / `sixRows` (`ChestMenu.java:41-47`): the two row
/// counts a chest block entity is ever opened at.
pub const CHEST_MENU_ROWS: i16 = 3;
pub const DOUBLE_CHEST_MENU_ROWS: i16 = 6;

/// `ServerPlayer.nextContainerCounter` (`ServerPlayer.java:1314-1316`):
/// `this.containerCounter = this.containerCounter % 100 + 1`, so an allocated
/// id is always in `1..=100` and never collides with `InventoryMenu`'s zero.
pub const CONTAINER_COUNTER_WRAP: i32 = 100;

/// The armor block's order, from `InventoryMenu.SLOT_IDS`
/// (InventoryMenu.java:45). The inventory slot each one occupies comes from
/// the generated equipment catalog, not from this order.
const MENU_ARMOR_SLOT_IDS: [EquipmentSlot; 4] = [
    EquipmentSlot::Head,
    EquipmentSlot::Chest,
    EquipmentSlot::Legs,
    EquipmentSlot::Feet,
];

/// `Container.getMaxStackSize` (Container.java:34-36): the ceiling a container
/// puts on any slot, independent of the item.
pub const CONTAINER_MAX_STACK_SIZE: i32 = 99;

/// `Container.getMaxStackSize(ItemStack)` (Container.java:38-40): the
/// container's own ceiling and the stack's, whichever is smaller.
pub fn container_max_stack_size<S: ContainerStack + ?Sized>(item: &S) -> Result<i32> {
    Ok(item.stack_max_size()?.min(CONTAINER_MAX_STACK_SIZE))
}

pub trait ContainerStack {
    fn stack_count(&self) -> i32;
    fn stack_is_empty(&self) -> bool;
    fn stack_max_size(&self) -> Result<i32>;
}
impl ContainerStack for RuntimeItemStack {
    fn stack_count(&self) -> i32 {
        self.count()
    }
    fn stack_is_empty(&self) -> bool {
        self.is_empty()
    }
    fn stack_max_size(&self) -> Result<i32> {
        self.max_stack_size()
    }
}
impl ContainerStack for ItemStackSummary {
    fn stack_count(&self) -> i32 {
        self.count
    }
    fn stack_is_empty(&self) -> bool {
        is_empty(self)
    }
    fn stack_max_size(&self) -> Result<i32> {
        max_stack_size(self)
    }
}
impl<S: ContainerStack + ?Sized> ContainerStack for &S {
    fn stack_count(&self) -> i32 {
        (**self).stack_count()
    }
    fn stack_is_empty(&self) -> bool {
        (**self).stack_is_empty()
    }
    fn stack_max_size(&self) -> Result<i32> {
        (**self).stack_max_size()
    }
}

/// `AbstractContainerMenu.getRedstoneSignalFromContainer`
/// (`AbstractContainerMenu.java:751-767`): what a comparator reads off a
/// container, as the average of each slot's fill fraction.
///
/// The divisor is per stack, not the flat container ceiling: a chest full of
/// 64-stacks is `1.0` and answers 15, where dividing by
/// [`CONTAINER_MAX_STACK_SIZE`] would answer 10.
///
/// An empty slot list is vanilla's `0 / 0`, whose `NaN` fails both the floor
/// and the `> 0` test; it is taken here as the zero it evaluates to.
pub fn redstone_signal_from_container<S: ContainerStack>(items: &[S]) -> Result<i32> {
    redstone_signal_from_container_access(items.len(), |slot| Ok(&items[slot]), |error| error)
}

pub fn redstone_signal_from_container_access<S: ContainerStack, E>(
    size: usize,
    mut read: impl FnMut(usize) -> std::result::Result<S, E>,
    mut ceiling_error: impl FnMut(bbb_server_types::ServerWorldError) -> E,
) -> std::result::Result<i32, E> {
    if size == 0 {
        return Ok(0);
    }
    let mut total_percent = 0.0f32;
    for slot in 0..size {
        let item = read(slot)?;
        if item.stack_is_empty() {
            continue;
        }
        total_percent += item.stack_count() as f32
            / container_max_stack_size(&item).map_err(&mut ceiling_error)? as f32;
    }
    total_percent /= size as f32;
    // `Mth.lerpDiscrete(totalPercent, 0, 15)` (`Mth.java:527-530`).
    Ok((total_percent * 14.0).floor() as i32 + i32::from(total_percent > 0.0))
}

/// `AbstractContainerMenu.SLOT_CLICKED_OUTSIDE` (:39): the click that lands on
/// no slot at all.
pub const SLOT_CLICKED_OUTSIDE: i16 = -999;
/// `AbstractContainerMenu.QUICKCRAFT_TYPE_*` / `QUICKCRAFT_HEADER_*` (:40-45).
const QUICKCRAFT_TYPE_CHARITABLE: i32 = 0;
const QUICKCRAFT_TYPE_GREEDY: i32 = 1;
const QUICKCRAFT_TYPE_CLONE: i32 = 2;
pub const QUICKCRAFT_HEADER_START: i32 = 0;
pub const QUICKCRAFT_HEADER_CONTINUE: i32 = 1;
pub const QUICKCRAFT_HEADER_END: i32 = 2;
/// `AbstractContainerMenu.CARRIED_SLOT_SIZE` (:46), the ceiling `tryRemove`
/// uses when nothing narrower applies.
pub const CARRIED_SLOT_SIZE: i32 = i32::MAX;
/// `ArmorSlot.getMaxStackSize` (ArmorSlot.java:40-42).
pub const ARMOR_SLOT_MAX_STACK_SIZE: i32 = 1;
/// `AbstractContainerMenu.SWAP` accepts a hotbar button or the offhand button
/// (:472); the offhand's number is its `Inventory` slot.
pub const SWAP_OFFHAND_BUTTON: i32 = 40;

/// `ItemStack.isEmpty` (ItemStack.java:308-310). A stack whose count reached
/// zero is the empty stack, so every write normalizes through here.
pub fn normalized(item: ItemStackSummary) -> ItemStackSummary {
    if item.item_id.is_none() || item.count <= 0 {
        return ItemStackSummary::empty();
    }
    item
}

/// `BrewingStandMenu.PotionSlot.mayPlaceItem` (`BrewingStandMenu.java:184-186`):
/// the three potion items and an empty bottle.
#[must_use]
pub fn brewing_potion_item(item: &ItemStackSummary) -> bool {
    brewing_potion_item_for_id(item.item_id)
}

pub(super) fn brewing_potion_item_for_id(item_id: Option<i32>) -> bool {
    item_id.is_some_and(|item_id| {
        item_id == bbb_protocol::item_types::VANILLA_ITEM_POTION_ID
            || item_id == bbb_protocol::item_types::VANILLA_ITEM_SPLASH_POTION_ID
            || item_id == bbb_protocol::item_types::VANILLA_ITEM_LINGERING_POTION_ID
            || item_id == bbb_protocol::item_types::VANILLA_ITEM_GLASS_BOTTLE_ID
    })
}

pub fn is_empty(item: &ItemStackSummary) -> bool {
    item.item_id.is_none() || item.count <= 0
}

/// `ItemStack.split` (ItemStack.java:328-333): takes at most `count` off and
/// leaves the rest behind.
pub fn split(item: &mut ItemStackSummary, count: i32) -> ItemStackSummary {
    let taken = count.min(item.count).max(0);
    let result = ItemStackSummary {
        count: taken,
        ..item.clone()
    };
    item.count -= taken;
    *item = normalized(std::mem::replace(item, ItemStackSummary::empty()));
    normalized(result)
}

/// `ItemStack.copyWithCount` (ItemStack.java:590-598).
pub fn copy_with_count(item: &ItemStackSummary, count: i32) -> ItemStackSummary {
    normalized(ItemStackSummary {
        count,
        ..item.clone()
    })
}

/// `ItemStack.matches` (ItemStack.java:647-653): the same item, the same
/// components and the same count.
pub fn item_stack_matches(a: &ItemStackSummary, b: &ItemStackSummary) -> bool {
    a.count == b.count && is_same_item_same_components(a, b)
}

/// `RemoteSlot` (RemoteSlot.java): what the server believes the client's copy
/// of one slot holds.
///
/// The click packet carries the client's own prediction, so a server that
/// agrees sends nothing back. Vanilla keeps either a stack it sent itself or
/// the hash the client sent, and compares whichever it has.
#[derive(Debug, Clone, PartialEq)]
pub enum RemoteSlot {
    /// `RemoteSlot.Synchronized.remoteStack`: a value this server put there.
    Known(RuntimeItemStack),
    /// `remoteHash`: the client's claim, still unverified.
    Hashed(HashedStack),
    /// Both fields still null, which matches nothing.
    Unknown,
}

impl RemoteSlot {
    /// `RemoteSlot.Synchronized.force` (:40-43).
    pub fn force<S: Clone + Into<RuntimeItemStack>>(&mut self, outgoing: &S) {
        *self = Self::Known(outgoing.clone().into());
    }

    /// `RemoteSlot.Synchronized.receive` (:46-49).
    pub fn receive(&mut self, incoming: &HashedStack) {
        *self = Self::Hashed(incoming.clone());
    }

    /// `RemoteSlot.Synchronized.matches` (:52-61) delegating to
    /// `HashedStack.matches` (HashedStack.java:20-22, 50-56) and
    /// `HashedPatchMap.matches` (HashedPatchMap.java:32-55). A hash that
    /// matches is promoted to the stack it matched, exactly as vanilla does.
    pub fn matches(
        &mut self,
        lookup: &dyn bbb_registry_codec::item_hash::ItemHashLookup,
        local: &RuntimeItemStack,
    ) -> bool {
        match self {
            Self::Known(remote) => remote.matches(local),
            Self::Hashed(hash) => {
                if !bbb_registry_codec::item_hash::hashed_stack_matches(
                    lookup,
                    hash,
                    local.summary(),
                ) {
                    return false;
                }
                *self = Self::Known(local.clone());
                true
            }
            Self::Unknown => false,
        }
    }

    /// [`Self::matches`] without hash promotion, `None` when a hash needs
    /// verification and potential promotion, so an unchanged known slot is
    /// compared without a copy.
    fn matches_in_place(&self, local: &RuntimeItemStack) -> Option<bool> {
        match self {
            Self::Known(remote) => Some(remote.matches(local)),
            Self::Hashed(_) => None,
            Self::Unknown => Some(false),
        }
    }
}

/// One packet the container synchronizer (ServerPlayer.java:273-321) hands the
/// connection.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuSync {
    /// `ClientboundContainerSetContentPacket` from `sendInitialData`
    /// (ServerPlayer.java:289-296).
    Content {
        container_id: i32,
        state_id: i32,
        items: Vec<ItemStackSummary>,
        carried: ItemStackSummary,
    },
    /// `ClientboundContainerSetSlotPacket` from `sendSlotChange`
    /// (ServerPlayer.java:299-301).
    Slot {
        container_id: i32,
        state_id: i32,
        slot: i16,
        item: ItemStackSummary,
    },
    /// `ClientboundSetCursorItemPacket` from `sendCarriedChange`
    /// (ServerPlayer.java:304-306), which carries no container id: the cursor
    /// belongs to the player, not to a menu.
    Cursor { item: ItemStackSummary },
}

/// What a menu slot id addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MenuSlot {
    /// The open crafting menu's result, which `AbstractCraftingMenu` owns.
    CraftingResult,
    /// `CrafterMenu`'s independent `NonInteractiveResultSlot`.
    CrafterPreview,
    /// One slot of the open crafting menu's grid (the player's 2x2 or a
    /// crafting table's 3x3), which the menu also owns.
    CraftingGrid(usize),
    /// A slot of the player's own `Inventory`.
    Inventory(usize),
    /// A slot of the `Container` the open menu was built over, which for a
    /// `ChestMenu` is the block entity behind the clicked block
    /// (`ChestMenu.java:52`, `:64`).
    Container(usize),
}

/// One menu's `AbstractContainerMenu.slots` list, as a shape rather than as a
/// list of `Slot` objects.
///
/// Vanilla builds the list in the menu's constructor and every `Slot` in it
/// remembers both which `Container` it reads and where in that container its
/// index lands (`Slot.java:11-22`). Both facts are a pure function of the
/// menu's kind and of the slot index, so they are answered here instead of
/// being stored per slot -- which is what lets one `doClick` run over every
/// menu without a second slot list beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuLayout {
    /// `InventoryMenu` (InventoryMenu.java:48-74): the menu that is always
    /// open, and the only one with a crafting grid, armour slots and an
    /// off-hand slot.
    PlayerInventory,
    /// `ChestMenu` (`ChestMenu.java:49-59`): `rows * 9` plain slots over the
    /// backing container first (`addChestGrid`, `:61-67`), then
    /// `addStandardInventorySlots` (AbstractContainerMenu.java:86-91).
    Chest { rows: i16 },
    /// `DispenserMenu` (`DispenserMenu.java:9-28`): nine container slots in a
    /// 3x3 grid, then `addStandardInventorySlots`. Slot indices match a
    /// one-row chest; the `MenuType` is `generic_3x3`.
    Generic3x3,
    /// `CrafterMenu`: nine inputs, the standard inventory, then a preview.
    Crafter,
    /// `CraftingMenu` (`CraftingMenu.java:38-45`): the result, the 3x3 grid,
    /// then `addStandardInventorySlots`; grid and result belong to the menu.
    CraftingTable,
    /// `HopperMenu` (`HopperMenu.java:17-28`): five container slots followed by player inventory.
    Hopper,
    /// `ShulkerBoxMenu` (`ShulkerBoxMenu.java:17-32`): twenty-seven container
    /// slots then `addStandardInventorySlots`. Indices match a three-row chest;
    /// container slots refuse nested shulker boxes (`ShulkerBoxSlot.java:12-14`).
    ShulkerBox,
    /// `LecternMenu` (`LecternMenu.java:22-36`): one book slot and no player
    /// inventory rows. Page turns are `clickMenuButton`, not slot clicks.
    Lectern,
    /// `AbstractFurnaceMenu` (`AbstractFurnaceMenu.java:38-73`) with its three
    /// subclasses: three container slots -- input, fuel, result -- then
    /// `addStandardInventorySlots` (`:71`). The kind only picks the
    /// `MenuType`.
    Furnace { kind: FurnaceMenuKind },
    /// `BrewingStandMenu` (`BrewingStandMenu.java:41-55`): three bottle slots,
    /// the ingredient slot, the fuel slot, then `addStandardInventorySlots`.
    BrewingStand,
    /// `MerchantMenu` (`MerchantMenu.java:38-46`): two payment slots and the
    /// result over the menu's own `MerchantContainer`, then
    /// `addStandardInventorySlots`.
    Merchant,
    /// A block menu whose slots all belong to the menu: its inputs and result,
    /// then `addStandardInventorySlots`.
    Utility(UtilityMenuKind),
    /// `HorseInventoryMenu` (`HorseInventoryMenu.java:18-45`): the saddle and
    /// body `ArmorSlot`s, `columns * 3` cargo slots, then
    /// `addStandardInventorySlots`. Opened by `ClientboundMountScreenOpenPacket`,
    /// so it has no `MenuType`.
    Mount { columns: i16 },
}

/// `FurnaceMenu` / `SmokerMenu` / `BlastFurnaceMenu`, which differ only in the
/// `MenuType` they register (`FurnaceMenu.java:10-14`,
/// `SmokerMenu.java:10-14`, `BlastFurnaceMenu.java:10-14`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FurnaceMenuKind {
    Furnace,
    Smoker,
    BlastFurnace,
}

/// The behaviours `InventoryMenu`'s slots differ in.
///
/// The menu builds three kinds (InventoryMenu.java:50-71): the crafting result
/// nothing may be placed into, four `ArmorSlot`s that hold one equippable item
/// each, and plain slots. The offhand slot's only override is an equipment
/// event with no wire form, so it is plain here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSlotKind {
    /// `ResultSlot` (ResultSlot.java).
    CraftingResult,
    CrafterInput,
    CrafterPreview,
    /// `ArmorSlot` (ArmorSlot.java).
    Armor(EquipmentSlot),
    /// `Slot` itself.
    Plain,
    /// `ShulkerBoxSlot` (`ShulkerBoxSlot.java:12-14`).
    ShulkerBox,
    /// The lectern book slot: `canPlaceItem` is false
    /// (`LecternBlockEntity.java:100-102`).
    LecternBook,
    /// `AbstractFurnaceMenu`'s input slot (`AbstractFurnaceMenu.java:68`): a
    /// plain `Slot`, so anything may be placed and the recipe decides later.
    FurnaceInput,
    /// `FurnaceFuelSlot` (`FurnaceFuelSlot.java:12-25`): a fuel or an empty
    /// bucket, the latter capped at one per slot.
    FurnaceFuel,
    /// `FurnaceResultSlot` (`FurnaceResultSlot.java:19-52`): nothing may be
    /// placed, and taking from it awards the used recipes.
    FurnaceResult,
    /// `BrewingStandMenu.PotionSlot` (`BrewingStandMenu.java:159-192`): a
    /// potion or an empty bottle, one per slot.
    BrewingPotion,
    /// `BrewingStandMenu.IngredientsSlot` (`:145-157`): only what
    /// `PotionBrewing.isIngredient` answers.
    BrewingIngredient,
    /// `BrewingStandMenu.FuelSlot` (`:125-143`): only the
    /// `minecraft:brewing_fuel` tag.
    BrewingFuel,
    /// `MerchantResultSlot` (`MerchantResultSlot.java:9-65`).
    MerchantResult,
    /// An input slot of a utility menu, whose rules its kind answers.
    UtilityInput,
    /// A utility menu's result slot: nothing may be placed, and taking runs
    /// the menu's `onTake`.
    UtilityResult,
    /// `HorseInventoryMenu`'s `ArmorSlot` over the mount's saddle or body.
    MountEquipment(EquipmentSlot),
}

/// One stack a click hands to `Player.drop` (`Player.java:582-583`), which is
/// `LivingEntity.drop(stack, false, thrownFromHand)` (`:758-774`).
///
/// The flag is the only thing that differs between the drop sites: a stack
/// thrown from the hand carries the thrower and awards the drop statistics
/// (`ServerPlayer.drop`, `ServerPlayer.java:2003-2013`), a remainder the
/// inventory refused does neither.
#[derive(Debug, Clone, PartialEq)]
pub struct DroppedMenuStack {
    pub stack: RuntimeItemStack,
    pub thrown_from_hand: bool,
}

/// One `AbstractContainerMenu`'s own state (:50-65), independent of which
/// menu it is.
///
/// Vanilla keeps a whole object per menu and swaps `ServerPlayer.containerMenu`
/// between them; this is the part of that object that is not derivable from
/// [`MenuLayout`], so a swap here is a swap of one of these.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuState {
    /// `AbstractContainerMenu.containerId` (:59).
    container_id: i32,
    /// Which slot list this menu has, i.e. what `slots` was filled with.
    layout: MenuLayout,
    /// `AbstractContainerMenu.stateId` (:57).
    state_id: i32,
    /// `AbstractContainerMenu.carried` (:53), the stack on the cursor.
    carried: RuntimeItemStack,
    /// `AbstractContainerMenu.quickcraftType` / `quickcraftStatus` /
    /// `quickcraftSlots` (:60-62): the drag-to-spread state machine.
    quickcraft_type: i32,
    quickcraft_status: i32,
    quickcraft_slots: BTreeSet<i16>,
    last_slots: Vec<RuntimeItemStack>,
    synchronizer_attached: bool,
    /// `AbstractContainerMenu.remoteSlots` / `remoteCarried` (:54-56): what
    /// the server believes the client already shows.
    remote_slots: Vec<RemoteSlot>,
    remote_carried: RemoteSlot,
    /// `AbstractContainerMenu.suppressRemoteUpdates` (:65).
    suppress_remote_updates: bool,
    crafter: Option<crafter_menu::CrafterMenuState>,
    merchant: Option<merchant_menu::MerchantMenuState>,
    crafting_table: Option<crafting::CraftingTableState>,
    utility: Option<utility_menu::UtilityMenuState>,
    mount: Option<mount_menu::MountMenuState>,
    vehicle_container: Option<(crate::world::LevelId, i32)>,
    /// `BundleContents.selectedItem` of the bundle in one slot, bound to the
    /// stack that was selected so swapping or replacing the slot invalidates it.
    bundle_selection: Option<(i16, i32, RuntimeItemStack)>,
    bundle_sounds: Vec<BundleSound>,
}

impl MenuState {
    /// The state `addSlot` (:123-129) leaves behind for a freshly built menu:
    /// one `RemoteSlot.PLACEHOLDER` per slot, and a state id that has not
    /// been stamped yet.
    pub fn new(container_id: i32, layout: MenuLayout) -> Self {
        Self {
            container_id,
            layout,
            state_id: 0,
            carried: RuntimeItemStack::empty(),
            // `quickcraftType` starts at -1 (AbstractContainerMenu.java:60),
            // which is not one of the three declared types.
            quickcraft_type: -1,
            quickcraft_status: 0,
            quickcraft_slots: BTreeSet::new(),
            last_slots: vec![
                RuntimeItemStack::empty();
                usize::try_from(layout.slot_count()).expect("a menu has slots")
            ],
            synchronizer_attached: true,
            remote_slots: vec![
                RemoteSlot::Unknown;
                usize::try_from(layout.slot_count()).expect("a menu has slots")
            ],
            remote_carried: RemoteSlot::Unknown,
            suppress_remote_updates: false,
            crafter: (layout == MenuLayout::Crafter).then(Default::default),
            merchant: (layout == MenuLayout::Merchant).then(Default::default),
            crafting_table: (layout == MenuLayout::CraftingTable).then(Default::default),
            utility: match layout {
                MenuLayout::Utility(kind) => Some(utility_menu::UtilityMenuState::new(kind)),
                _ => None,
            },
            mount: None,
            vehicle_container: None,
            bundle_selection: None,
            bundle_sounds: Vec::new(),
        }
    }
}

/// Where the `Container` an open menu was built over lives, so the level can
/// be re-read before a click and written back after it.
///
/// Vanilla's menu holds the `Container` itself and every write lands in the
/// block entity at once. This server cannot hold that borrow across a packet
/// handler, so the menu carries a working copy of the container's slots and
/// the caller keeps it in step; the positions are what tells it which block
/// entities to copy. A double chest names two of them in
/// `CompoundContainer` order (`CompoundContainer.java:11-14`), first then
/// second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerAnchor {
    pub positions: Vec<BlockPos>,
}

/// A player's `Inventory`, the `InventoryMenu` that is always open over it,
/// and whatever `ServerPlayer.containerMenu` currently points at.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerInventoryMenu {
    /// `Inventory`: the storage slots followed by the equipment slots.
    inventory: Vec<RuntimeItemStack>,
    /// `AbstractCraftingMenu.craftSlots`, a 2x2 grid the menu owns.
    crafting_grid: Vec<RuntimeItemStack>,
    /// `Player.enderChestInventory` (`Player.java:151`). Vanilla hangs it off
    /// the player rather than off a menu; this server keeps the player's item
    /// stores together, next to the working copy of an opened container.
    ender_chest: ender_chest::EnderChestContainer,
    /// `AbstractCraftingMenu.resultSlots`.
    crafting_result: RuntimeItemStack,
    /// Catalog, RecipeBook and take-side state for the 2x2 player grid.
    crafting: InventoryCrafting,
    /// `Inventory.selected`.
    selected: u8,
    /// `ServerPlayer.containerMenu` (`Player.java:153`): the menu clicks
    /// address, which is the inventory menu unless one was opened over a
    /// block.
    menu: MenuState,
    /// `Player.inventoryMenu` (`Player.java:152`) while `containerMenu` is a
    /// different menu. `ServerPlayer.doCloseContainer`
    /// (`ServerPlayer.java:1405-1409`) puts it back.
    suspended_inventory_menu: Option<MenuState>,
    /// A working copy of the `Container` the open menu was built over, empty
    /// whenever the inventory menu is the open one.
    container: Vec<RuntimeItemStack>,
    /// Which block entities that copy came from.
    container_anchor: Option<ContainerAnchor>,
    ender_chest_menu: bool,
    container_instances: Vec<crate::block_entity::BlockEntity>,
    /// `ServerPlayer.containerCounter` (`ServerPlayer.java:361`).
    container_counter: i32,
    menu_epoch: u64,
    /// The furnace menu's two datapack-driven admission sets
    /// (`AbstractFurnaceMenu.java:140-146`): the items a cooking recipe accepts
    /// as input, and the items `FuelValues` names as fuel. They are attached
    /// when a furnace menu is opened and dropped with it, because both come
    /// from the live recipe and tag generation rather than from the slot kind.
    furnace_inputs: Option<Arc<BTreeSet<i32>>>,
    furnace_fuels: Option<Arc<BTreeSet<i32>>>,
    /// `AbstractContainerMenu.remoteDataSlots` (`AbstractContainerMenu.java:55`)
    /// for the open menu: the data-slot values the client was last told, which
    /// is what keeps a settled menu quiet in `broadcastChanges` (`:204-207`).
    /// It belongs to the menu and dies with it.
    menu_data: BTreeMap<i16, i16>,
    /// The `minecraft:brewing_fuel` tag members the brewing stand menu's fuel
    /// slot admits (`BrewingStandMenu.java:135-137`), attached when the menu
    /// opens because the tag is datapack data.
    brewing_fuels: Option<Arc<BTreeSet<i32>>>,
    /// `FurnaceResultSlot.removeCount` (`FurnaceResultSlot.java:11`,
    /// `:24-30`): what the slot's `remove` collected, which
    /// `ItemStack.onCraftedBy` (`ItemStack.java:723-726`) is handed.
    furnace_result_remove_count: i32,
    /// How many `FurnaceResultSlot.onTake` / `onQuickCraft` runs
    /// (`:36-49`) the current click completed. A hotbar swap takes without
    /// calling `remove` (`AbstractContainerMenu.java:478-481`), so its count
    /// is zero and the recipes are still awarded.
    furnace_result_takes: i32,
    brewed_potion_take: Option<i32>,
    /// Enchantment holder ids carrying `PREVENT_ARMOR_CHANGE`, which the
    /// world attaches before a click because they are datapack data.
    armor_change_locks: Arc<BTreeSet<i32>>,
    /// The entity type tags `Equippable.canBeEquippedBy` reads for the armor slots.
    entity_tags: crate::entity::EntityRegistryTagView,
    /// Session modded content registry for resolving custom holder IDs in
    /// `HashedStack` verification (`ServerPlayer.java:274-286`).
    content_registry: Arc<bbb_protocol::content_registry::ContentRegistry>,
    /// `ArmorSlot.setByPlayer` / the offhand slot's override
    /// (`ArmorSlot.java:34-37`, `InventoryMenu.java:62-66`): the
    /// `onEquipItem` calls the world plays after the write.
    equip_changes: Vec<EquipChange>,
}

/// One `LivingEntity.onEquipItem(slot, previous, stack)` call.
#[derive(Debug, Clone, PartialEq)]
pub struct EquipChange {
    pub slot: bbb_protocol::equipment_slots::EquipmentSlot,
    pub old: RuntimeItemStack,
    pub new: RuntimeItemStack,
}

#[derive(Debug)]
pub struct ParkedOpenMenu {
    menu: MenuState,
    container: Vec<RuntimeItemStack>,
    epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedMenu {
    pub container_id: i32,
    pub menu_type_id: i32,
}

/// What `ServerPlayer.openMenu` (`ServerPlayer.java:1337-1338`) puts on the
/// connection: the `ClientboundOpenScreenPacket` fields, then the initial data
/// `initMenu`'s `setSynchronizer` sends (AbstractContainerMenu.java:150-155).
#[derive(Debug, Clone, PartialEq)]
pub struct OpenedMenu {
    pub container_id: i32,
    pub menu_type_id: i32,
    pub initial: MenuSync,
}

/// What `ServerPlayer.doCloseContainer` (`ServerPlayer.java:1405-1409`) leaves
/// for its caller to carry out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ClosedMenu {
    pub dropped: Vec<RuntimeItemStack>,
    /// The inventory slots `Inventory.placeItemBackInInventory`
    /// (Inventory.java:307-324) reported, which is what the
    /// `ClientboundSetPlayerInventoryPacket`s name.
    pub filled: Vec<usize>,
    /// The container `ChestMenu.removed` (`ChestMenu.java:100-103`) calls
    /// `stopOpen` on, or `None` when the inventory menu was the open one.
    pub closed_container: Option<ContainerAnchor>,
}

/// `Inventory.SELECTION_SIZE` as the `SWAP` button bound reads it.
pub const SELECTION_SIZE_I32: i32 = SELECTION_SIZE as i32;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod chest_menu_tests;

#[cfg(test)]
mod brewing_menu_tests;

#[cfg(test)]
mod crafting_tests;

#[cfg(test)]
mod furnace_menu_tests;

#[cfg(test)]
mod test_support;
#[cfg(test)]
pub(crate) use test_support::test_item_resolver;
