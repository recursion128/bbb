use serde::{Deserialize, Serialize};

const VANILLA_26_1_SOUND_EVENTS: &str = include_str!("../data/sound_events_26_1.txt");

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundEventRegistry {
    by_protocol_id: Vec<String>,
}

impl SoundEventRegistry {
    pub fn vanilla_26_1() -> Self {
        Self::from_ids(
            VANILLA_26_1_SOUND_EVENTS
                .lines()
                .filter(|line| !line.is_empty()),
        )
    }

    pub fn from_ids(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            by_protocol_id: ids.into_iter().map(Into::into).collect(),
        }
    }

    pub fn event_id(&self, registry_id: i32) -> Option<&str> {
        let index = usize::try_from(registry_id).ok()?;
        self.by_protocol_id.get(index).map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.by_protocol_id.iter().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.by_protocol_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_protocol_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_26_1_registry_uses_sound_events_static_order() {
        let registry = SoundEventRegistry::vanilla_26_1();

        assert_eq!(registry.len(), 1902);
        assert_eq!(
            registry.event_id(0),
            Some("minecraft:entity.allay.ambient_with_item")
        );
        assert_eq!(registry.event_id(7), Some("minecraft:ambient.cave"));
        assert_eq!(registry.event_id(286), Some("minecraft:entity.cat.ambient"));
        assert_eq!(
            registry.event_id(295),
            Some("minecraft:entity.cat_royal.ambient")
        );
        assert_eq!(
            registry.event_id(839),
            Some("minecraft:item.goat_horn.sound.0")
        );
        assert_eq!(
            registry.event_id(846),
            Some("minecraft:item.goat_horn.sound.7")
        );
        assert_eq!(
            registry.event_id(1834),
            Some("minecraft:entity.wolf_cute.whine")
        );
        assert_eq!(
            registry.event_id(1901),
            Some("minecraft:item.nautilus_saddle_equip")
        );
        assert_eq!(registry.event_id(1902), None);
    }

    #[test]
    fn vanilla_26_1_registry_has_unique_ids() {
        let registry = SoundEventRegistry::vanilla_26_1();
        let mut ids = registry.iter().collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), registry.len());
    }
}
