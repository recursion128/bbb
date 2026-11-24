//! Command data access for a live block entity.

use std::fmt;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::{Result as CoreResult, ServerWorldError};

use super::{
    unique_field, BlockEntity, BlockEntityItemLoad, BlockEntityItemProjection, BlockEntityItemSave,
};

#[derive(Debug)]
pub enum BlockEntityCommandDataError {
    InvalidBlockState {
        block_entity_type_id: i32,
        current_block_state_id: i32,
    },
    Item(ServerWorldError),
}

impl From<ServerWorldError> for BlockEntityCommandDataError {
    fn from(error: ServerWorldError) -> Self {
        Self::Item(error)
    }
}

impl fmt::Display for BlockEntityCommandDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBlockState {
                block_entity_type_id,
                current_block_state_id,
            } => write!(
                formatter,
                "block state {current_block_state_id} does not support block entity type {block_entity_type_id}"
            ),
            Self::Item(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for BlockEntityCommandDataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidBlockState { .. } => None,
            Self::Item(error) => Some(error),
        }
    }
}

impl BlockEntity {
    // TypedEntityData.java:139-164: custom-only merge compares raw tags before loading.
    pub fn merge_item_custom_data_with_resolver(
        &mut self,
        overlay: &[NbtSummaryEntry],
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> CoreResult<bool> {
        let projection = BlockEntityItemProjection::with_resolver(resolver);
        let before = self.saved_without_metadata(projection)?;
        let mut merged = before.clone();
        bbb_protocol::merge_command_nbt_compound(&mut merged, overlay);
        if merged == before {
            return Ok(false);
        }
        self.load_additional(&merged, &mut BlockEntityItemLoad::new(projection))?;
        Ok(true)
    }

    /// `BlockDataAccessor.getData` (`BlockDataAccessor.java:70-72`).
    ///
    /// This is the resolver-free form; a caller that holds the level's
    /// publication reads [`Self::command_data_with_resolver`] instead, which is
    /// the `registryAccess()` vanilla passes to `saveWithFullMetadata`.
    pub fn command_data(&self) -> CoreResult<NbtSummaryValue> {
        Ok(NbtSummaryValue::Compound(self.save_with_full_metadata()?))
    }

    /// [`Self::command_data`] through the registry publication the caller
    /// holds, so a stack whose components name a dynamic-registry holder
    /// (`minecraft:enchantments`, `minecraft:trim`, …) keeps them where the
    /// resolver-free form above leaves the whole slot out.
    pub fn command_data_with_resolver(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> CoreResult<NbtSummaryValue> {
        Ok(NbtSummaryValue::Compound(
            self.save_with_full_metadata_with_resolver(resolver)?,
        ))
    }

    /// `BlockEntity.saveWithoutMetadata(registries)` (`BlockEntity.java:128-134`)
    /// through the same publication as [`Self::command_data_with_resolver`]:
    /// that record without the id and position `saveMetadata` adds
    /// (`:161-166`), which is what `handleBlockEntityTagQuery` answers with.
    pub fn save_without_metadata_with_resolver(
        &self,
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> CoreResult<NbtSummaryValue> {
        Ok(NbtSummaryValue::Compound(self.saved_without_metadata(
            BlockEntityItemProjection::with_resolver(resolver),
        )?))
    }

    /// Applies `BlockDataAccessor.setData`'s `loadWithComponents` to this object,
    /// answering whether the load moved its `saveWithoutMetadata` -- the
    /// `!after.equals(before)` test `BlockInput.place` (BlockInput.java:91)
    /// raises `affected` for.
    ///
    /// Identity metadata belongs to the live object, not the supplied compound
    /// (`BlockDataAccessor.java:59-65`, `BlockEntity.java:98-101`).
    ///
    /// This is the resolver-free form; a command that has the level's
    /// publication uses [`Self::load_command_data_in_place_with_resolver`].
    pub fn load_command_data_in_place(
        &mut self,
        current_block_state_id: i32,
        entries: &[NbtSummaryEntry],
    ) -> Result<bool, BlockEntityCommandDataError> {
        self.load_command_data_projected(
            current_block_state_id,
            entries,
            BlockEntityItemProjection::without_resolver(),
        )
    }

    /// [`Self::load_command_data_in_place`] through the registry publication
    /// the caller holds, which is the same one `/give` and `/item …` project
    /// with.
    ///
    /// `before` and `after` are `saveWithoutMetadata` (BlockInput.java:85,
    /// `:89`) under the projection the load itself runs with.
    pub fn load_command_data_in_place_with_resolver(
        &mut self,
        current_block_state_id: i32,
        entries: &[NbtSummaryEntry],
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> Result<bool, BlockEntityCommandDataError> {
        self.load_command_data_projected(
            current_block_state_id,
            entries,
            BlockEntityItemProjection::with_resolver(resolver),
        )
    }

    fn load_command_data_projected(
        &mut self,
        current_block_state_id: i32,
        entries: &[NbtSummaryEntry],
        projection: BlockEntityItemProjection<'_>,
    ) -> Result<bool, BlockEntityCommandDataError> {
        if !self.is_valid_block_state(current_block_state_id) {
            return Err(BlockEntityCommandDataError::InvalidBlockState {
                block_entity_type_id: self.type_id,
                current_block_state_id,
            });
        }

        let before = self.saved_without_metadata(projection)?;
        self.load_with_components_projected(entries, projection)?;
        let after = self.saved_without_metadata(projection)?;
        Ok(after != before)
    }

    fn load_with_components_projected(
        &mut self,
        entries: &[NbtSummaryEntry],
        projection: BlockEntityItemProjection<'_>,
    ) -> CoreResult<()> {
        self.load_additional(entries, &mut BlockEntityItemLoad::new(projection))?;
        self.components = match unique_field(entries, "components") {
            Some(NbtSummaryValue::Compound(entries)) => {
                super::sanitize_block_entity_components(entries, projection)
            }
            _ => Vec::new(),
        };
        Ok(())
    }

    // FallingBlockEntity.java:204-212: each root tag replaces the saved tag, including compounds.
    pub fn merge_falling_block_data_with_resolver(
        &mut self,
        overlay: &[NbtSummaryEntry],
        resolver: &dyn PersistentItemStackHolderResolver,
    ) -> CoreResult<()> {
        let projection = BlockEntityItemProjection::with_resolver(resolver);
        let mut merged = self.saved_without_metadata(projection)?;
        for entry in overlay {
            if let Some(previous) = merged
                .iter_mut()
                .find(|previous| previous.name == entry.name)
            {
                *previous = entry.clone();
            } else {
                merged.push(entry.clone());
            }
        }
        self.load_with_components_projected(&merged, projection)
    }

    /// `BlockEntity.saveWithoutMetadata(output)` (BlockEntity.java:136-139)
    /// under one projection, which is what `BlockInput.place` saves on either
    /// side of its load.
    fn saved_without_metadata(
        &self,
        projection: BlockEntityItemProjection<'_>,
    ) -> CoreResult<Vec<NbtSummaryEntry>> {
        self.save_without_metadata(&mut BlockEntityItemSave::new(projection))
    }
}

#[cfg(test)]
mod falling_data_tests;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;

    use bbb_protocol::packets::BlockPos;

    use super::*;
    use crate::block_entity::{entry, new_block_entity, unique_field};

    fn state_of(name: &str, properties: &[(&str, &str)]) -> i32 {
        let properties = properties
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<BTreeMap<_, _>>();
        vanilla_block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("{name} {properties:?} is a vanilla 26.1 block state"))
    }

    fn comparator_state() -> i32 {
        state_of(
            "minecraft:comparator",
            &[
                ("facing", "north"),
                ("mode", "compare"),
                ("powered", "false"),
            ],
        )
    }

    fn pos() -> BlockPos {
        BlockPos { x: 4, y: 5, z: 6 }
    }

    #[test]
    fn command_data_query_is_the_full_metadata_compound() {
        let mut block_entity = new_block_entity(pos(), comparator_state()).unwrap();
        block_entity.comparator_mut().unwrap().set_output_signal(13);

        assert_eq!(
            block_entity.command_data().unwrap(),
            NbtSummaryValue::Compound(
                block_entity
                    .save_with_full_metadata_with_resolver(
                        crate::block_entity::test_item_resolver(),
                    )
                    .unwrap()
            )
        );
        let NbtSummaryValue::Compound(entries) = block_entity.command_data().unwrap() else {
            panic!("command data must be a compound");
        };
        assert_eq!(
            unique_field(&entries, "id"),
            Some(&NbtSummaryValue::string("minecraft:comparator"))
        );
        assert_eq!(unique_field(&entries, "x"), Some(&NbtSummaryValue::Int(4)));
        assert_eq!(unique_field(&entries, "y"), Some(&NbtSummaryValue::Int(5)));
        assert_eq!(unique_field(&entries, "z"), Some(&NbtSummaryValue::Int(6)));
        assert_eq!(
            unique_field(&entries, "OutputSignal"),
            Some(&NbtSummaryValue::Int(13))
        );
        assert_eq!(
            unique_field(&entries, "components"),
            Some(&NbtSummaryValue::Compound(Vec::new()))
        );
    }

    #[test]
    fn command_data_load_updates_payload_and_components_without_identity() {
        let state = comparator_state();
        let mut block_entity = new_block_entity(pos(), state).unwrap();
        let original_type = block_entity.type_id();

        let components = vec![entry(
            "minecraft:custom_name",
            NbtSummaryValue::string("kept"),
        )];
        block_entity
            .load_command_data_in_place(
                state,
                &[
                    entry("OutputSignal", NbtSummaryValue::Short(2)),
                    entry("components", NbtSummaryValue::Compound(components.clone())),
                    entry("id", NbtSummaryValue::string("minecraft:banner")),
                    entry("x", NbtSummaryValue::Int(-100)),
                    entry("y", NbtSummaryValue::Int(-101)),
                    entry("z", NbtSummaryValue::Int(-102)),
                ],
            )
            .unwrap();

        assert_eq!(block_entity.type_id(), original_type);
        assert_eq!(block_entity.block_pos(), pos());
        assert_eq!(block_entity.block_state(), state);
        assert_eq!(block_entity.comparator().unwrap().output_signal(), 2);
        let NbtSummaryValue::Compound(saved) = block_entity.command_data().unwrap() else {
            unreachable!();
        };
        assert_eq!(
            unique_field(&saved, "components"),
            Some(&NbtSummaryValue::Compound(components))
        );
        assert_eq!(
            unique_field(&saved, "id"),
            Some(&NbtSummaryValue::string("minecraft:comparator"))
        );
        assert_eq!(unique_field(&saved, "x"), Some(&NbtSummaryValue::Int(4)));
        assert_eq!(unique_field(&saved, "y"), Some(&NbtSummaryValue::Int(5)));
        assert_eq!(unique_field(&saved, "z"), Some(&NbtSummaryValue::Int(6)));
    }

    /// The load answers whether it moved the record, which is what
    /// `BlockInput.place` (BlockInput.java:91) raises `affected` for.
    ///
    /// `loadAdditional` reads the whole record rather than merging it, so a
    /// compound that omits a field the type carries resets that field to the
    /// type's own default -- and that reset is a change like any other.
    #[test]
    fn command_data_load_reports_whether_it_moved_the_record() {
        let state = comparator_state();
        let mut block_entity = new_block_entity(pos(), state).unwrap();
        block_entity.comparator_mut().unwrap().set_output_signal(13);

        // The record this object already carries, plus a key the type does not
        // read: nothing moves.
        for entries in [
            vec![entry("OutputSignal", NbtSummaryValue::Int(13))],
            vec![
                entry("OutputSignal", NbtSummaryValue::Int(13)),
                entry("Lock", NbtSummaryValue::string("k")),
            ],
        ] {
            assert!(
                !block_entity
                    .load_command_data_in_place(state, &entries)
                    .unwrap(),
                "{entries:?}"
            );
            assert_eq!(block_entity.comparator().unwrap().output_signal(), 13);
        }

        // A different value for the field the type reads.
        assert!(block_entity
            .load_command_data_in_place(state, &[entry("OutputSignal", NbtSummaryValue::Int(7))],)
            .unwrap());
        assert_eq!(block_entity.comparator().unwrap().output_signal(), 7);

        // A compound that omits the field resets it to `DEFAULT_OUTPUT`, which
        // is a change too.
        assert!(block_entity
            .load_command_data_in_place(state, &[entry("Lock", NbtSummaryValue::string("k"))])
            .unwrap());
        assert_eq!(block_entity.comparator().unwrap().output_signal(), 0);

        // The component map is part of the record, so a load that sets one is
        // a change even where every field of the type is the same.
        assert!(block_entity
            .load_command_data_in_place(
                state,
                &[
                    entry("OutputSignal", NbtSummaryValue::Int(0)),
                    entry(
                        "components",
                        NbtSummaryValue::Compound(vec![entry(
                            "minecraft:custom_name",
                            NbtSummaryValue::string("named"),
                        )]),
                    ),
                ],
            )
            .unwrap());
    }
}
