use crate::registry_tags::RegistryTagSnapshot;

type ArmorStandPassengerTeamFn<'a> = &'a dyn Fn(uuid::Uuid, &bbb_protocol::JavaString);

#[derive(Clone, Copy)]
pub struct EntityNbtReadContext<'a> {
    pub(super) registry_tags: &'a RegistryTagSnapshot,
    section_registration: bool,
    armor_stand_passenger_team: Option<ArmorStandPassengerTeamFn<'a>>,
}

impl<'a> EntityNbtReadContext<'a> {
    pub(crate) fn new(registry_tags: &'a RegistryTagSnapshot) -> Self {
        Self {
            registry_tags,
            section_registration: false,
            armor_stand_passenger_team: None,
        }
    }

    pub(crate) fn with_armor_stand_passenger_team(
        self,
        apply: &'a dyn Fn(uuid::Uuid, &bbb_protocol::JavaString),
    ) -> Self {
        Self {
            armor_stand_passenger_team: Some(apply),
            ..self
        }
    }

    pub(super) fn apply_armor_stand_passenger_team(
        self,
        entity: &crate::entity::Entity,
        entries: &[bbb_protocol::NbtSummaryEntry],
    ) {
        if !matches!(entity.data(), crate::entity::EntityData::ArmorStand(_)) {
            return;
        }
        if let (Some(apply), Some(bbb_protocol::NbtSummaryValue::String(team))) = (
            self.armor_stand_passenger_team,
            super::optional_field(entries, "Team"),
        ) {
            apply(entity.uuid(), team);
        }
    }
}

impl EntityNbtReadContext<'_> {
    pub(super) fn with_section_registration(self) -> Self {
        // PersistentEntitySectionManager.java:74-112 registers removed chunk entities;
        // ServerPlayer.java:439-445 instead post-processes through addWithUUID.
        Self {
            section_registration: true,
            ..self
        }
    }

    pub(super) fn registers_discarded_items(self) -> bool {
        self.section_registration
    }
}
