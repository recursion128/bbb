//! `ParticleArgument.readParticle` (`ParticleArgument.java:59-82`): the type
//! id, then the option compound -- or the empty map -- through that type's
//! codec, and the wire bytes of the value it decodes.

use bbb_protocol::particle_types::vanilla_particle_type_for_resource_id;
use bbb_protocol::{
    component_literal_nbt, parse_command_snbt_value_prefix_utf16, NbtSummaryValue,
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_registry_codec::registry_codec::RegistryReferenceLookup;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::particle_options::particle_payload_from_nbt;

use super::{read_identifier_value, snbt_failure, ArgumentValue, ParseFailure};
use crate::commands::translatable;
use crate::world::registry_runtime::PlayerItemStackResolver;
use crate::world::ServerWorld;

pub(in crate::commands) fn read_particle(
    command: &[u16],
    cursor: usize,
    world: &ServerWorld,
) -> Result<(usize, ArgumentValue), ParseFailure> {
    let (end, resource_id) = read_identifier_value(command, cursor)?;
    // `createWithContext` reads the cursor `Identifier.read` left after the id
    // (`ParticleArgument.java:65-67`).
    let Some(type_id) = vanilla_particle_type_for_resource_id(&resource_id)
        .map(|entry| entry.id)
        .or_else(|| {
            world
                .plugins()
                .content_registry()
                .particle_by_resource_id(&resource_id)
                .map(|spec| spec.id)
        })
    else {
        return Err(ParseFailure {
            message: translatable(
                "particle.notFound",
                vec![component_literal_nbt(&resource_id)],
            ),
            cursor: Some(end),
        });
    };
    let (end, options) = if command[end..].starts_with(&[123]) {
        match parse_command_snbt_value_prefix_utf16(&command[end..]) {
            Ok((consumed, value)) => (end + consumed, value),
            Err(failure) => {
                return Err(snbt_failure(end, &failure).unwrap_or_else(|| ParseFailure {
                    message: translatable("argument.nbt.invalid", Vec::new()),
                    cursor: Some(end),
                }))
            }
        }
    } else {
        (end, NbtSummaryValue::Compound(Vec::new()))
    };
    let payload = particle_payload_from_nbt(
        &resource_id,
        &options,
        &ParticleRegistries(&world.item_stack_resolver()),
    )
    .map_err(invalid_options)?;
    Ok((
        end,
        ArgumentValue::Particle {
            type_id,
            resource_id,
            raw_options: payload.raw_options,
        },
    ))
}

/// `ERROR_INVALID_OPTIONS::create` (`ParticleArgument.java:32-34`, `:81`)
/// carries no input, so the refusal has no cursor line.
fn invalid_options(message: String) -> ParseFailure {
    ParseFailure {
        message: translatable(
            "particle.invalidOptions",
            vec![NbtSummaryValue::string(message)],
        ),
        cursor: None,
    }
}

/// The registries `ParticleArgument` decodes against (`ParticleArgument.java:73`):
/// the served level's publication.
struct ParticleRegistries<'a>(&'a PlayerItemStackResolver);

impl PersistentRegistryResolver for ParticleRegistries<'_> {
    fn registry_available(&self, registry_id: &str) -> bool {
        self.0.registry_available(registry_id)
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        self.0.resource_membership(registry_id, resource_id)
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        self.0.tag_members(registry_id, tag_id)
    }
}

impl PersistentItemStackHolderResolver for ParticleRegistries<'_> {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        self.0.holder_id(registry_id, resource_id)
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        self.0.holder_resource_id(registry_id, holder_id)
    }

    fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        self.0.content_registry()
    }
}

impl RegistryReferenceLookup for ParticleRegistries<'_> {
    fn registry_available(&self, registry: &str) -> bool {
        PersistentRegistryResolver::registry_available(self, registry)
    }

    fn element_membership(
        &self,
        registry: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        self.resource_membership(registry, resource_id)
    }

    fn tag_membership(&self, registry: &str, resource_id: &str) -> PersistentRegistryMembership {
        match self.tag_members(registry, resource_id) {
            PersistentRegistryTagMembers::Present(_) => PersistentRegistryMembership::Present,
            PersistentRegistryTagMembers::Absent => PersistentRegistryMembership::Absent,
            PersistentRegistryTagMembers::Unavailable => PersistentRegistryMembership::Unavailable,
        }
    }

    fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
        matches!(
            self.resource_membership(registry, resource_id),
            PersistentRegistryMembership::Present
        )
    }

    fn contains_tag(&self, registry: &str, resource_id: &str) -> bool {
        matches!(
            self.tag_members(registry, resource_id),
            PersistentRegistryTagMembers::Present(_)
        )
    }
}
