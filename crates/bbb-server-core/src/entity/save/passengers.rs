use super::*;

pub(super) fn saved_entity_record_with_view(
    view: crate::entity::store::EntityBodyReadView<'_>,
    entity: &Entity,
) -> Result<Java26EntityRecord, EntitySaveError> {
    saved_entity_record_excluding(view, entity, &rustc_hash::FxHashSet::default())
}

pub(super) fn saved_entity_record_excluding(
    view: crate::entity::store::EntityBodyReadView<'_>,
    entity: &Entity,
    excluded: &rustc_hash::FxHashSet<Uuid>,
) -> Result<Java26EntityRecord, EntitySaveError> {
    let mut raw = entity_record(entity, false, Some(view.store()))?;
    let position = apply_vehicle_save_position(view, entity, &mut raw);
    let passengers = saved_passenger_records(view, entity, excluded)?;
    Ok(Java26EntityRecord::new(
        raw,
        entity.data().entity_resource_id(),
        entity.uuid(),
        [position.x, position.y, position.z],
        passengers,
    ))
}

pub(super) fn apply_vehicle_save_position(
    view: crate::entity::store::EntityBodyReadView<'_>,
    entity: &Entity,
    entries: &mut [NbtSummaryEntry],
) -> Vec3d {
    let Some(vehicle) = view
        .store()
        .vehicle(entity.id())
        .and_then(|id| view.get(id))
    else {
        return entity.position();
    };
    let Some(entry) = entries.iter_mut().find(|entry| entry.name == "Pos") else {
        unreachable!("entity records contain Pos")
    };
    let position = Vec3d {
        x: vehicle.position().x,
        y: entity.position().y,
        z: vehicle.position().z,
    };
    entry.value = double_list(position);
    position
}

pub(super) fn append_passenger_records(
    view: crate::entity::store::EntityBodyReadView<'_>,
    entity: &Entity,
    entries: &mut Vec<NbtSummaryEntry>,
) -> Result<(), EntitySaveError> {
    // `Entity.saveWithoutId` recursively emits serializable passengers and
    // discards an empty list (`Entity.java:2027-2038`).
    let passengers: Vec<_> =
        saved_passenger_records(view, entity, &rustc_hash::FxHashSet::default())?
            .into_iter()
            .map(|record| NbtSummaryValue::Compound(record.into_raw()))
            .collect();
    if !passengers.is_empty() {
        entries.push(NbtSummaryEntry {
            name: "Passengers".into(),
            value: NbtSummaryValue::List(passengers),
        });
    }
    Ok(())
}

fn saved_passenger_records(
    view: crate::entity::store::EntityBodyReadView<'_>,
    entity: &Entity,
    excluded: &rustc_hash::FxHashSet<Uuid>,
) -> Result<Vec<Java26EntityRecord>, EntitySaveError> {
    view.store()
        .passengers(entity.id())
        .iter()
        .filter_map(|id| view.get(*id))
        .filter(|passenger| {
            !excluded.contains(&passenger.uuid())
                && passenger
                    .removal_reason()
                    .is_none_or(|reason| reason.should_save())
                && (bbb_protocol::entity_types::vanilla_entity_type_is_serializable(
                    passenger.entity_type_id(),
                ) || view
                    .store()
                    .plugins()
                    .content_registry()
                    .entity_by_id(passenger.entity_type_id())
                    .is_some_and(|spec| spec.serializable))
        })
        .map(|passenger| saved_entity_record_excluding(view, passenger, excluded))
        .collect()
}
