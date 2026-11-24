//! Registry membership answers the persistent codecs ask of the caller's
//! registry view.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistentRegistryMembership {
    Present,
    Absent,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistentRegistryTagMembers {
    Present(Vec<String>),
    Absent,
    Unavailable,
}

pub trait PersistentRegistryResolver {
    /// Legacy complete views remain compatible; partial views must supply their getter status.
    fn registry_available(&self, _registry_id: &str) -> bool {
        true
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership;

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers;

    fn tag_contains(&self, registry_id: &str, tag_id: &str, resource_id: &str) -> Option<bool> {
        match self.tag_members(registry_id, tag_id) {
            PersistentRegistryTagMembers::Present(members) => {
                Some(members.iter().any(|member| member == resource_id))
            }
            PersistentRegistryTagMembers::Absent => Some(false),
            PersistentRegistryTagMembers::Unavailable => None,
        }
    }
}
