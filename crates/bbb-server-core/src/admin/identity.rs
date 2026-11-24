//! Management-surface player identity.
//!
//! Vanilla owners: `NameAndId` (NameAndId.java:11-45), the key every stored
//! user list is stored under, and `PlayerDto` (PlayerDto.java:11-30), the
//! by-id-and/or-by-name reference the management API accepts.

use bbb_protocol::{codec::offline_player_uuid_utf16, player_names::valid_player_name, JavaString};
use uuid::Uuid;

/// `NameAndId` (NameAndId.java:11): one resolved identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NameAndId {
    pub id: Uuid,
    pub name: JavaString,
}

impl NameAndId {
    /// `NameAndId.createOffline` (NameAndId.java:42-45).
    pub fn offline(name: impl Into<JavaString>) -> Self {
        let name = name.into();
        Self {
            id: offline_player_uuid_utf16(name.units()),
            name,
        }
    }
}

/// `PlayerDto` (PlayerDto.java:11): a management request names a player by id,
/// by name, or by both, and every field is optional on the wire.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerRef {
    pub id: Option<Uuid>,
    pub name: Option<JavaString>,
}

impl PlayerRef {
    pub fn by_id(id: Uuid) -> Self {
        Self {
            id: Some(id),
            name: None,
        }
    }

    pub fn by_name(name: impl Into<JavaString>) -> Self {
        Self {
            id: None,
            name: Some(name.into()),
        }
    }
}

impl From<&NameAndId> for PlayerRef {
    fn from(user: &NameAndId) -> Self {
        Self {
            id: Some(user.id),
            name: Some(user.name.clone()),
        }
    }
}

/// Everything identity resolution may consult: the players currently on the
/// server plus the names already recorded in a stored user list.
///
/// `MinecraftPlayerListServiceImpl.getUser` (MinecraftPlayerListServiceImpl.java:38-45)
/// answers from the server's name-to-id cache and, failing that, from the
/// Mojang session service. This server is offline-only by product decision, so
/// there is no session service to ask and no cache that a network lookup could
/// fill: a name alone resolves through `UUIDUtil.createOfflinePlayerUUID`,
/// which is exactly how the login path already assigns the id
/// (`bbb_protocol::codec::offline_player_uuid`), and an id alone resolves only
/// from an identity the server has actually seen.
#[derive(Debug, Clone, Copy)]
pub struct IdentityDirectory<'a> {
    pub known: &'a [NameAndId],
}

impl IdentityDirectory<'_> {
    /// `MinecraftPlayerListService.getUser(Optional<UUID>, Optional<String>)`.
    ///
    /// Returns `None` where vanilla's future completes empty, which every
    /// caller treats as "skip this entry" rather than as an error.
    pub fn resolve(&self, reference: &PlayerRef) -> Option<NameAndId> {
        match (reference.id, reference.name.as_ref()) {
            (Some(id), Some(name)) => {
                name.with_utf8(valid_player_name)
                    .unwrap_or(false)
                    .then(|| NameAndId {
                        id,
                        name: name.clone(),
                    })
            }
            (Some(id), None) => self.known.iter().find(|user| user.id == id).cloned(),
            (None, Some(name)) => {
                if !name.with_utf8(valid_player_name).unwrap_or(false) || name.is_empty() {
                    return None;
                }
                // A name the server already knows keeps that identity even when
                // its id is not the offline derivation, so a saved list entry
                // never forks into a second identity for the same name.
                self.known
                    .iter()
                    .find(|user| &user.name == name)
                    .cloned()
                    .or_else(|| Some(NameAndId::offline(name)))
            }
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::codec::offline_player_uuid;

    fn directory(known: &[NameAndId]) -> IdentityDirectory<'_> {
        IdentityDirectory { known }
    }

    #[test]
    fn a_bare_name_resolves_through_the_offline_uuid() {
        let resolved = directory(&[]).resolve(&PlayerRef::by_name("bbb-client"));
        assert_eq!(
            resolved,
            Some(NameAndId {
                id: offline_player_uuid("bbb-client"),
                name: "bbb-client".into(),
            })
        );
    }

    #[test]
    fn a_known_name_keeps_its_recorded_identity() {
        let recorded = NameAndId {
            id: Uuid::from_u128(7),
            name: "bbb-client".into(),
        };
        let known = [recorded.clone()];
        assert_eq!(
            directory(&known).resolve(&PlayerRef::by_name("bbb-client")),
            Some(recorded)
        );
    }

    #[test]
    fn a_bare_id_resolves_only_from_a_seen_identity() {
        let recorded = NameAndId {
            id: Uuid::from_u128(7),
            name: "bbb-client".into(),
        };
        let known = [recorded.clone()];
        assert_eq!(
            directory(&known).resolve(&PlayerRef::by_id(Uuid::from_u128(7))),
            Some(recorded)
        );
        assert_eq!(
            directory(&known).resolve(&PlayerRef::by_id(Uuid::from_u128(8))),
            None
        );
    }

    #[test]
    fn an_empty_or_invalid_reference_resolves_to_nothing() {
        assert_eq!(directory(&[]).resolve(&PlayerRef::default()), None);
        assert_eq!(directory(&[]).resolve(&PlayerRef::by_name("")), None);
        assert_eq!(
            directory(&[]).resolve(&PlayerRef::by_name("with space")),
            None
        );
        assert_eq!(
            directory(&[]).resolve(&PlayerRef {
                id: Some(Uuid::from_u128(1)),
                name: Some("with space".into()),
            }),
            None
        );
    }

    #[test]
    fn both_fields_are_taken_verbatim() {
        assert_eq!(
            directory(&[]).resolve(&PlayerRef {
                id: Some(Uuid::from_u128(3)),
                name: Some("bbb-client".into()),
            }),
            Some(NameAndId {
                id: Uuid::from_u128(3),
                name: "bbb-client".into(),
            })
        );
    }
}
