//! `EnderDragonFight` SavedData (`EnderDragonFight.java:105-121`), filed as
//! `data/minecraft/ender_dragon_fight.dat` in the dimension that hosts it.

use uuid::Uuid;

use crate::level_file::read_gzip_compound;
use crate::schema_support::{
    bool_value, compound_or_empty, field, int_array, int_value, require_java_26_1_root, uuid_nbt,
    uuid_value,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};
use bbb_server_types::save::{
    Java26LevelKey, NamedCompound, SaveResult, WorldSaveLimits, WorldSavePaths,
    JAVA_26_1_DATA_VERSION,
};

pub(crate) const ENDER_DRAGON_FIGHT_FILE: &str = "ender_dragon_fight.dat";

/// `DragonRespawnStage` (`DragonRespawnStage.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Java26DragonRespawnStage {
    Start,
    PreparingToSummonPillars,
    SummoningPillars,
    SummoningDragon,
    End,
}

impl Java26DragonRespawnStage {
    const ALL: [(Self, &'static str); 5] = [
        (Self::Start, "start"),
        (
            Self::PreparingToSummonPillars,
            "preparing_to_summon_pillars",
        ),
        (Self::SummoningPillars, "summoning_pillars"),
        (Self::SummoningDragon, "summoning_dragon"),
        (Self::End, "end"),
    ];

    fn name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(stage, _)| *stage == self)
            .map(|(_, name)| *name)
            .expect("every stage is named")
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, candidate)| *candidate == name)
            .map(|(stage, _)| *stage)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26EnderDragonFight {
    pub needs_state_scanning: bool,
    pub dragon_killed: bool,
    pub previously_killed: bool,
    pub respawn_stage: Option<Java26DragonRespawnStage>,
    pub respawn_time: i32,
    pub dragon_uuid: Option<Uuid>,
    pub exit_portal_location: Option<BlockPos>,
    pub gateways: Vec<i32>,
    pub respawn_crystals: Vec<Uuid>,
}

impl Default for Java26EnderDragonFight {
    /// `EnderDragonFight.createDefault` (`EnderDragonFight.java:123-125`).
    fn default() -> Self {
        Self {
            needs_state_scanning: true,
            dragon_killed: false,
            previously_killed: false,
            respawn_stage: None,
            respawn_time: 0,
            dragon_uuid: None,
            exit_portal_location: None,
            gateways: Vec::new(),
            respawn_crystals: Vec::new(),
        }
    }
}

/// `None` when the dimension has never saved a fight.
pub fn load_java_26_1_ender_dragon_fight(
    paths: &WorldSavePaths,
    dimension: &Java26LevelKey,
    limits: WorldSaveLimits,
) -> SaveResult<Option<(Java26EnderDragonFight, usize)>> {
    let path = paths
        .dimension_dir(dimension)
        .join("data/minecraft")
        .join(ENDER_DRAGON_FIGHT_FILE);
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(crate::atomic_file::io_error(
                "reading ender dragon fight metadata",
                &path,
                error,
            ))
        }
        Ok(_) => {}
    }
    let (document, bytes) = read_gzip_compound(&path, limits)?;
    Ok(Some((
        parse_java_26_1_ender_dragon_fight_document(&document)?,
        bytes,
    )))
}

pub fn parse_java_26_1_ender_dragon_fight_document(
    document: &NamedCompound,
) -> SaveResult<Java26EnderDragonFight> {
    let data = compound_or_empty(
        require_java_26_1_root(document, "ender_dragon_fight")?,
        "data",
    );
    let boolean =
        |name: &str, default: bool| field(data, name).and_then(bool_value).unwrap_or(default);
    let respawn_stage = match field(data, "respawn_stage") {
        Some(NbtSummaryValue::String(name)) => name
            .try_to_string()
            .ok()
            .and_then(|name| Java26DragonRespawnStage::from_name(&name)),
        _ => None,
    };
    let gateways = match field(data, "gateways") {
        Some(NbtSummaryValue::IntArray(values)) => values.clone(),
        Some(NbtSummaryValue::List(values)) => values
            .iter()
            .map(int_value)
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    let respawn_crystals = match field(data, "respawn_crystals") {
        Some(NbtSummaryValue::List(values)) => values.iter().filter_map(uuid_value).collect(),
        _ => Vec::new(),
    };
    Ok(Java26EnderDragonFight {
        needs_state_scanning: boolean("needs_state_scanning", true),
        dragon_killed: boolean("dragon_killed", false),
        previously_killed: boolean("previously_killed", false),
        respawn_stage,
        respawn_time: field(data, "respawn_time").and_then(int_value).unwrap_or(0),
        dragon_uuid: field(data, "dragon_uuid").and_then(uuid_value),
        exit_portal_location: field(data, "exit_portal_location")
            .and_then(int_array)
            .map(|[x, y, z]| BlockPos { x, y, z }),
        gateways,
        respawn_crystals,
    })
}

pub fn java_26_1_ender_dragon_fight_document(fight: &Java26EnderDragonFight) -> NamedCompound {
    let flag = |value: bool| NbtSummaryValue::Byte(i8::from(value));
    let mut fields = vec![
        entry("needs_state_scanning", flag(fight.needs_state_scanning)),
        entry("dragon_killed", flag(fight.dragon_killed)),
        entry("previously_killed", flag(fight.previously_killed)),
    ];
    if let Some(stage) = fight.respawn_stage {
        fields.push(entry(
            "respawn_stage",
            NbtSummaryValue::string(stage.name()),
        ));
    }
    fields.push(entry(
        "respawn_time",
        NbtSummaryValue::Int(fight.respawn_time),
    ));
    if let Some(uuid) = fight.dragon_uuid {
        fields.push(entry("dragon_uuid", uuid_nbt(uuid)));
    }
    if let Some(pos) = fight.exit_portal_location {
        fields.push(entry(
            "exit_portal_location",
            NbtSummaryValue::IntArray(vec![pos.x, pos.y, pos.z]),
        ));
    }
    fields.push(entry(
        "gateways",
        NbtSummaryValue::IntArray(fight.gateways.clone()),
    ));
    if !fight.respawn_crystals.is_empty() {
        fields.push(entry(
            "respawn_crystals",
            NbtSummaryValue::List(
                fight
                    .respawn_crystals
                    .iter()
                    .copied()
                    .map(uuid_nbt)
                    .collect(),
            ),
        ));
    }
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("data", NbtSummaryValue::Compound(fields)),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ender_dragon_fight_document_round_trips_and_defaults_missing_fields() {
        let fight = Java26EnderDragonFight {
            needs_state_scanning: false,
            dragon_killed: true,
            previously_killed: true,
            respawn_stage: Some(Java26DragonRespawnStage::SummoningPillars),
            respawn_time: 42,
            dragon_uuid: Some(Uuid::from_u128(0x1234)),
            exit_portal_location: Some(BlockPos { x: 0, y: 63, z: 0 }),
            gateways: vec![3, 17, 9],
            respawn_crystals: vec![Uuid::from_u128(5), Uuid::from_u128(6)],
        };
        let document = java_26_1_ender_dragon_fight_document(&fight);
        assert_eq!(
            parse_java_26_1_ender_dragon_fight_document(&document).unwrap(),
            fight
        );
        let empty = NamedCompound::new(
            "",
            vec![
                entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                entry("data", NbtSummaryValue::Compound(Vec::new())),
            ],
        );
        assert_eq!(
            parse_java_26_1_ender_dragon_fight_document(&empty).unwrap(),
            Java26EnderDragonFight::default()
        );
    }
}
