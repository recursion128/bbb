use bbb_renderer::{
    CameraPose, HudContextualBar, HudJumpBar, HudLocatorBar, HudLocatorMarker, HudLocatorPitch,
};
use bbb_world::{java_string_hash_code, EntityVec3, WaypointState, WaypointVec3iState, WorldStore};
use glam::Vec3;
use uuid::Uuid;

const VISIBLE_DEGREE_RANGE: f32 = 60.0;
const DEFAULT_COLOR_BRIGHTNESS: f32 = 0.9;
const EXPERIENCE_PRIORITY_TICKS: u64 = 100;

#[derive(Debug, Clone, Copy)]
struct CameraEntityProjection {
    uuid: Option<Uuid>,
    position: EntityVec3,
}

#[derive(Debug, Default)]
pub(super) struct HudContextualRuntime {
    level_generation: Option<u64>,
    observed_experience: bool,
    last_experience_progress: f32,
    experience_display_start_tick: Option<u64>,
    last_priority_screen: Option<(i32, i32)>,
}

impl HudContextualRuntime {
    pub(super) fn select(
        &mut self,
        world: &WorldStore,
        level_generation: u64,
        client_tick: u64,
        locator: Option<HudLocatorBar>,
        jump: Option<HudJumpBar>,
        experience_progress: Option<f32>,
    ) -> HudContextualBar {
        self.synchronize_presentation_clock(
            world,
            level_generation,
            client_tick,
            experience_progress,
        );

        let jump = jump.and_then(sanitize_jump_bar);
        let experience_progress = experience_progress
            .filter(|progress| progress.is_finite())
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let can_show_experience =
            !world.local_player_is_creative() && !world.local_player_is_spectator();
        let prioritize_experience = self
            .experience_display_start_tick
            .is_some_and(|start| start.saturating_add(EXPERIENCE_PRIORITY_TICKS) > client_tick);
        let prioritize_jump = jump.is_some_and(|jump| jump.progress > 0.0 || jump.cooldown);

        if let Some(locator) = locator {
            if prioritize_jump {
                return HudContextualBar::Jump(jump.expect("prioritized jump exists"));
            }
            if can_show_experience && prioritize_experience {
                return HudContextualBar::Experience {
                    progress: experience_progress,
                };
            }
            return HudContextualBar::Locator(locator);
        }
        if let Some(jump) = jump {
            return HudContextualBar::Jump(jump);
        }
        if can_show_experience {
            return HudContextualBar::Experience {
                progress: experience_progress,
            };
        }
        HudContextualBar::Empty
    }

    fn synchronize_presentation_clock(
        &mut self,
        world: &WorldStore,
        level_generation: u64,
        client_tick: u64,
        experience_progress: Option<f32>,
    ) {
        if self.level_generation != Some(level_generation) {
            *self = Self {
                level_generation: Some(level_generation),
                ..Self::default()
            };
        }

        if let Some(progress) = experience_progress.filter(|progress| progress.is_finite()) {
            if self.observed_experience {
                if progress != self.last_experience_progress {
                    self.experience_display_start_tick = Some(client_tick);
                    self.last_experience_progress = progress;
                }
            } else {
                // LocalPlayer starts at progress zero and MIN_VALUE. Its first
                // synchronization changes MIN_VALUE to MIN_VALUE+1 rather than
                // opening the 100-tick contextual priority window.
                self.observed_experience = true;
                self.last_experience_progress = progress;
            }
        }

        let priority_screen = world
            .inventory()
            .open_container
            .as_ref()
            .and_then(|container| {
                let menu_type = container.menu_type_id?;
                matches!(
                    menu_type,
                    bbb_protocol::menu_types::VANILLA_MENU_TYPE_ANVIL_ID
                        | bbb_protocol::menu_types::VANILLA_MENU_TYPE_ENCHANTMENT_ID
                )
                .then_some((container.container_id, menu_type))
            });
        if priority_screen != self.last_priority_screen {
            if priority_screen.is_some() {
                self.experience_display_start_tick = Some(client_tick);
            }
            self.last_priority_screen = priority_screen;
        }
    }
}

fn sanitize_jump_bar(jump: HudJumpBar) -> Option<HudJumpBar> {
    jump.progress.is_finite().then_some(HudJumpBar {
        progress: jump.progress.clamp(0.0, 1.0),
        cooldown: jump.cooldown,
    })
}

pub(super) fn hud_locator_bar_from_world(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    camera_fov_degrees: f32,
    partial_tick: f32,
) -> Option<HudLocatorBar> {
    if world.tracked_waypoints().is_empty() {
        return None;
    }
    let Some(camera_pose) = camera_pose.filter(camera_pose_is_finite) else {
        return Some(HudLocatorBar::default());
    };
    let Some(camera_entity) = camera_entity_projection(world) else {
        return Some(HudLocatorBar::default());
    };
    let camera_eye = EntityVec3 {
        x: f64::from(camera_pose.position[0]),
        y: f64::from(camera_pose.position[1]) + f64::from(camera_pose.eye_height),
        z: f64::from(camera_pose.position[2]),
    };
    let mut markers = world
        .tracked_waypoints()
        .values()
        .filter_map(|waypoint| {
            marker_from_waypoint(
                world,
                waypoint,
                camera_pose,
                camera_eye,
                camera_entity,
                camera_fov_degrees,
                partial_tick,
            )
        })
        .collect::<Vec<_>>();
    markers.sort_by(|left, right| right.distance.total_cmp(&left.distance));
    Some(HudLocatorBar { markers })
}

fn marker_from_waypoint(
    world: &WorldStore,
    waypoint: &WaypointState,
    camera_pose: CameraPose,
    camera_eye: EntityVec3,
    camera_entity: CameraEntityProjection,
    camera_fov_degrees: f32,
    partial_tick: f32,
) -> Option<HudLocatorMarker> {
    let waypoint_uuid = waypoint_uuid(waypoint)?;
    if waypoint.identifier_kind == "uuid" && waypoint_uuid == camera_entity.uuid {
        return None;
    }

    let projection = match waypoint.data.kind.as_str() {
        "position" => {
            let vector = waypoint.data.position?;
            let center = vector_center(vector);
            let target = waypoint_uuid
                .and_then(|uuid| {
                    world.probe_waypoint_entity_eye_position_at_partial_tick(
                        uuid,
                        vector,
                        partial_tick,
                    )
                })
                .unwrap_or(center);
            WaypointProjection {
                distance: distance(camera_entity.position, center),
                angle_degrees: point_yaw_angle(camera_eye, target, camera_pose.y_rot),
                pitch: point_pitch_direction(camera_pose, target, camera_fov_degrees),
            }
        }
        "chunk" => {
            let chunk = waypoint.data.chunk?;
            let target = EntityVec3 {
                x: f64::from(chunk.x) * 16.0 + 8.5,
                y: f64::from(camera_entity.position.y.floor() as i32) + 0.5,
                z: f64::from(chunk.z) * 16.0 + 8.5,
            };
            WaypointProjection {
                distance: distance(camera_entity.position, target),
                angle_degrees: point_yaw_angle(camera_eye, target, camera_pose.y_rot),
                pitch: horizon_pitch_direction(camera_pose.x_rot, camera_fov_degrees),
            }
        }
        "azimuth" => {
            let azimuth = waypoint.data.azimuth?;
            WaypointProjection {
                distance: f32::INFINITY,
                angle_degrees: degrees_difference(camera_pose.y_rot, azimuth.to_degrees()),
                pitch: horizon_pitch_direction(camera_pose.x_rot, camera_fov_degrees),
            }
        }
        "empty" => return None,
        _ => return None,
    };
    if !angle_is_visible(projection.angle_degrees)
        || projection.distance.is_nan()
        || projection.distance.is_sign_negative()
    {
        return None;
    }

    Some(HudLocatorMarker {
        angle_degrees: projection.angle_degrees,
        pitch: projection.pitch,
        distance: projection.distance,
        style_id: waypoint.icon_style.clone(),
        color_argb: waypoint_color(waypoint, waypoint_uuid),
    })
}

fn angle_is_visible(angle_degrees: f32) -> bool {
    angle_degrees.is_finite()
        && angle_degrees > -VISIBLE_DEGREE_RANGE
        && angle_degrees <= VISIBLE_DEGREE_RANGE
}

#[derive(Debug, Clone, Copy)]
struct WaypointProjection {
    distance: f32,
    angle_degrees: f32,
    pitch: HudLocatorPitch,
}

fn waypoint_uuid(waypoint: &WaypointState) -> Option<Option<Uuid>> {
    match waypoint.identifier_kind.as_str() {
        "uuid" => Uuid::parse_str(&waypoint.identifier).ok().map(Some),
        "name" => Some(None),
        _ => None,
    }
}

fn camera_entity_projection(world: &WorldStore) -> Option<CameraEntityProjection> {
    let camera = world.local_player().camera;
    if camera.follows_player {
        let pose = world.local_player().pose?;
        return Some(CameraEntityProjection {
            uuid: world.local_player_profile_id(),
            position: EntityVec3 {
                x: pose.position.x,
                y: pose.position.y,
                z: pose.position.z,
            },
        });
    }

    let transform = world.probe_entity_transform(camera.entity_id?)?;
    Some(CameraEntityProjection {
        uuid: Some(transform.uuid),
        position: transform.position,
    })
}

fn camera_pose_is_finite(camera: &CameraPose) -> bool {
    camera.position.into_iter().all(f32::is_finite)
        && camera.y_rot.is_finite()
        && camera.x_rot.is_finite()
        && camera.eye_height.is_finite()
}

fn vector_center(vector: WaypointVec3iState) -> EntityVec3 {
    EntityVec3 {
        x: f64::from(vector.x) + 0.5,
        y: f64::from(vector.y) + 0.5,
        z: f64::from(vector.z) + 0.5,
    }
}

fn distance(from: EntityVec3, to: EntityVec3) -> f32 {
    let dx = from.x - to.x;
    let dy = from.y - to.y;
    let dz = from.z - to.z;
    (dx.mul_add(dx, dy.mul_add(dy, dz * dz)) as f32).sqrt()
}

fn point_yaw_angle(camera: EntityVec3, target: EntityVec3, camera_yaw: f32) -> f32 {
    let direction_x = camera.x - target.x;
    let direction_z = camera.z - target.z;
    let waypoint_angle = direction_x.atan2(-direction_z).to_degrees() as f32;
    degrees_difference(camera_yaw, waypoint_angle)
}

fn degrees_difference(from_angle: f32, to_angle: f32) -> f32 {
    wrap_degrees(to_angle - from_angle)
}

fn wrap_degrees(angle: f32) -> f32 {
    let mut wrapped = angle % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn point_pitch_direction(
    camera: CameraPose,
    target: EntityVec3,
    fov_degrees: f32,
) -> HudLocatorPitch {
    if !fov_degrees.is_finite() || !(0.0..180.0).contains(&fov_degrees) {
        return HudLocatorPitch::None;
    }
    let camera_eye = Vec3::new(
        camera.position[0],
        camera.position[1] + camera.eye_height,
        camera.position[2],
    );
    let target = Vec3::new(target.x as f32, target.y as f32, target.z as f32);
    let yaw = camera.y_rot.to_radians();
    let pitch = camera.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let forward =
        Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch).normalize_or_zero();
    if forward == Vec3::ZERO {
        return HudLocatorPitch::None;
    }
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    if right == Vec3::ZERO || up == Vec3::ZERO {
        return HudLocatorPitch::None;
    }
    let offset = target - camera_eye;
    let depth = offset.dot(forward);
    let vertical = offset.dot(up);
    let denominator = depth * (fov_degrees.to_radians() * 0.5).tan();
    let point_y = vertical / denominator;
    if point_y.is_nan() {
        return HudLocatorPitch::None;
    }
    let behind = depth < 0.0;
    let y_in_front = if behind { -point_y } else { point_y };
    if y_in_front < -1.0 {
        return HudLocatorPitch::Down;
    }
    if y_in_front > 1.0 {
        return HudLocatorPitch::Up;
    }
    if behind {
        if point_y > 0.0 {
            return HudLocatorPitch::Up;
        }
        if point_y < 0.0 {
            return HudLocatorPitch::Down;
        }
    }
    HudLocatorPitch::None
}

fn horizon_pitch_direction(x_rot_degrees: f32, fov_degrees: f32) -> HudLocatorPitch {
    if !x_rot_degrees.is_finite()
        || !fov_degrees.is_finite()
        || !(0.0..180.0).contains(&fov_degrees)
    {
        return HudLocatorPitch::None;
    }
    let horizon = if x_rot_degrees <= -90.0 {
        f32::NEG_INFINITY
    } else if x_rot_degrees >= 90.0 {
        f32::INFINITY
    } else {
        x_rot_degrees.to_radians().tan() / (fov_degrees.to_radians() * 0.5).tan()
    };
    if horizon < -1.0 {
        HudLocatorPitch::Down
    } else if horizon > 1.0 {
        HudLocatorPitch::Up
    } else {
        HudLocatorPitch::None
    }
}

fn waypoint_color(waypoint: &WaypointState, waypoint_uuid: Option<Uuid>) -> u32 {
    waypoint.icon_color_rgb.map_or_else(
        || {
            let hash = waypoint_uuid.map_or_else(
                || java_string_hash_code(&waypoint.identifier),
                java_uuid_hash_code,
            );
            argb_set_brightness(
                0xFF00_0000 | hash as u32 & 0x00FF_FFFF,
                DEFAULT_COLOR_BRIGHTNESS,
            )
        },
        |rgb| 0xFF00_0000 | rgb & 0x00FF_FFFF,
    )
}

fn java_uuid_hash_code(uuid: Uuid) -> i32 {
    let value = uuid.as_u128();
    let most = (value >> 64) as u64;
    let least = value as u64;
    ((most >> 32) as i32) ^ (most as i32) ^ ((least >> 32) as i32) ^ (least as i32)
}

fn argb_set_brightness(color: u32, brightness: f32) -> u32 {
    let mut red = (color >> 16) & 0xFF;
    let mut green = (color >> 8) & 0xFF;
    let mut blue = color & 0xFF;
    let alpha = color >> 24;
    let rgb_max = red.max(green).max(blue);
    let rgb_min = red.min(green).min(blue);
    let rgb_range = (rgb_max - rgb_min) as f32;
    let saturation = if rgb_max != 0 {
        rgb_range / rgb_max as f32
    } else {
        0.0
    };

    let mut hue = if saturation == 0.0 {
        0.0
    } else {
        let constant_red = (rgb_max - red) as f32 / rgb_range;
        let constant_green = (rgb_max - green) as f32 / rgb_range;
        let constant_blue = (rgb_max - blue) as f32 / rgb_range;
        let hue = if red == rgb_max {
            constant_blue - constant_green
        } else if green == rgb_max {
            2.0 + constant_red - constant_blue
        } else {
            4.0 + constant_green - constant_red
        };
        hue / 6.0
    };
    if hue < 0.0 {
        hue += 1.0;
    }
    if saturation == 0.0 {
        let component = (brightness * 255.0).round() as u32;
        return alpha << 24 | component << 16 | component << 8 | component;
    }

    let segment = (hue - hue.floor()) * 6.0;
    let offset = segment - segment.floor();
    let primary = brightness * (1.0 - saturation);
    let secondary = brightness * (1.0 - saturation * offset);
    let tertiary = brightness * (1.0 - saturation * (1.0 - offset));
    let components = match segment as i32 {
        0 => (brightness, tertiary, primary),
        1 => (secondary, brightness, primary),
        2 => (primary, brightness, tertiary),
        3 => (primary, secondary, brightness),
        4 => (tertiary, primary, brightness),
        _ => (brightness, primary, secondary),
    };
    red = (components.0 * 255.0).round() as u32;
    green = (components.1 * 255.0).round() as u32;
    blue = (components.2 * 255.0).round() as u32;
    alpha << 24 | red << 16 | green << 8 | blue
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        CommonPlayerSpawnInfo, OpenScreen, PlayLogin, PlayerExperience,
        TrackedWaypoint as ProtocolTrackedWaypoint, TrackedWaypointPacket, WaypointData,
        WaypointIcon, WaypointIdentifier, WaypointOperation, WaypointVec3i,
    };
    use bbb_world::LocalPlayerPoseState;

    fn world_with_local_player(profile_id: Uuid) -> WorldStore {
        world_with_game_type(profile_id, 0)
    }

    fn world_with_game_type(profile_id: Uuid, game_type: i8) -> WorldStore {
        let mut world = WorldStore::new();
        world.set_local_player_profile_id(profile_id);
        world.apply_login(&PlayLogin {
            player_id: 7,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        world.set_local_player_pose(LocalPlayerPoseState {
            position: bbb_protocol::packets::Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            ..LocalPlayerPoseState::default()
        });
        world
    }

    fn locator_bar() -> Option<HudLocatorBar> {
        Some(HudLocatorBar::default())
    }

    fn track(
        world: &mut WorldStore,
        identifier: WaypointIdentifier,
        style: &str,
        color_rgb: Option<u32>,
        data: WaypointData,
    ) {
        assert!(world.apply_waypoint(TrackedWaypointPacket {
            operation: WaypointOperation::Track,
            waypoint: ProtocolTrackedWaypoint {
                identifier,
                icon: WaypointIcon {
                    style: style.to_string(),
                    color_rgb,
                },
                data,
            },
        }));
    }

    fn camera_pose() -> CameraPose {
        CameraPose {
            position: [0.0, 64.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        }
    }

    #[test]
    fn waypoint_locator_projects_all_renderable_data_forms_far_first() {
        let profile_id = Uuid::from_u128(7);
        let mut world = world_with_local_player(profile_id);
        track(
            &mut world,
            WaypointIdentifier::Name("position".to_string()),
            "minecraft:default",
            Some(0x112233),
            WaypointData::Position(WaypointVec3i { x: 0, y: 64, z: 8 }),
        );
        track(
            &mut world,
            WaypointIdentifier::Name("chunk".to_string()),
            "minecraft:bowtie",
            None,
            WaypointData::Chunk(bbb_protocol::packets::ChunkPos { x: 0, z: 1 }),
        );
        track(
            &mut world,
            WaypointIdentifier::Name("azimuth".to_string()),
            "example:unknown",
            None,
            WaypointData::Azimuth(0.0),
        );
        track(
            &mut world,
            WaypointIdentifier::Name("empty".to_string()),
            "minecraft:default",
            None,
            WaypointData::Empty,
        );

        let bar = hud_locator_bar_from_world(&world, Some(camera_pose()), 70.0, 0.5).unwrap();
        assert_eq!(bar.markers.len(), 3);
        assert!(bar.markers[0].distance.is_infinite());
        assert_eq!(bar.markers[0].style_id, "example:unknown");
        assert!(bar.markers[1].distance > bar.markers[2].distance);
        assert_eq!(bar.markers[2].color_argb, 0xFF112233);
        assert!(bar
            .markers
            .iter()
            .all(|marker| marker.angle_degrees > -60.0 && marker.angle_degrees <= 60.0));
    }

    #[test]
    fn waypoint_locator_suppresses_only_the_camera_uuid() {
        let profile_id = Uuid::from_u128(0x1234);
        let mut world = world_with_local_player(profile_id);
        track(
            &mut world,
            WaypointIdentifier::Uuid(profile_id),
            "minecraft:default",
            None,
            WaypointData::Position(WaypointVec3i { x: 0, y: 64, z: 8 }),
        );
        track(
            &mut world,
            WaypointIdentifier::Name(profile_id.to_string()),
            "minecraft:default",
            None,
            WaypointData::Position(WaypointVec3i { x: 0, y: 64, z: 8 }),
        );

        let bar = hud_locator_bar_from_world(&world, Some(camera_pose()), 70.0, 0.5).unwrap();
        assert_eq!(bar.markers.len(), 1);
    }

    #[test]
    fn waypoint_locator_uses_exact_visibility_and_pitch_boundaries() {
        assert!(!angle_is_visible(-60.0));
        assert!(angle_is_visible(60.0));
        assert!(!angle_is_visible(60.000004));
        assert!(!angle_is_visible(f32::NAN));
        assert_eq!(wrap_degrees(540.0), -180.0);
        assert_eq!(wrap_degrees(-540.0), -180.0);
        assert_eq!(horizon_pitch_direction(36.0, 70.0), HudLocatorPitch::Up);
        assert_eq!(horizon_pitch_direction(-36.0, 70.0), HudLocatorPitch::Down);
        assert_eq!(
            point_pitch_direction(
                camera_pose(),
                EntityVec3 {
                    x: 0.0,
                    y: 100.0,
                    z: 10.0,
                },
                70.0,
            ),
            HudLocatorPitch::Up
        );
        assert_eq!(
            point_pitch_direction(
                camera_pose(),
                EntityVec3 {
                    x: 0.0,
                    y: 30.0,
                    z: 10.0,
                },
                70.0,
            ),
            HudLocatorPitch::Down
        );
        assert_eq!(
            point_pitch_direction(
                camera_pose(),
                EntityVec3 {
                    x: 0.0,
                    y: 100.0,
                    z: -10.0,
                },
                70.0,
            ),
            HudLocatorPitch::Up
        );
        assert_eq!(
            point_pitch_direction(
                camera_pose(),
                EntityVec3 {
                    x: 0.0,
                    y: 30.0,
                    z: -10.0,
                },
                70.0,
            ),
            HudLocatorPitch::Down
        );
    }

    #[test]
    fn waypoint_locator_java_hash_and_brightness_match_vanilla() {
        assert_eq!(java_string_hash_code("abc"), 96_354);
        assert_eq!(
            java_uuid_hash_code(Uuid::parse_str("00112233-4455-6677-8899-aabbccddeeff").unwrap()),
            0
        );
        assert_eq!(argb_set_brightness(0xFF000000, 0.9), 0xFFE6E6E6);
        assert_eq!(argb_set_brightness(0xFF017862, 0.9), 0xFF02E6BB);
    }

    #[test]
    fn waypoint_locator_keeps_background_state_without_a_camera() {
        let mut world = world_with_local_player(Uuid::from_u128(7));
        track(
            &mut world,
            WaypointIdentifier::Name("north".to_string()),
            "minecraft:default",
            None,
            WaypointData::Azimuth(std::f32::consts::PI),
        );

        let bar = hud_locator_bar_from_world(&world, None, 70.0, 0.5).unwrap();
        assert!(bar.markers.is_empty());
        assert_eq!(
            hud_locator_bar_from_world(&WorldStore::new(), None, 70.0, 0.5),
            None
        );
    }

    #[test]
    fn contextual_bar_uses_vanilla_idle_and_jump_priority() {
        let world = world_with_local_player(Uuid::from_u128(7));
        let mut runtime = HudContextualRuntime::default();
        assert_eq!(
            runtime.select(&world, 1, 0, None, None, None),
            HudContextualBar::Experience { progress: 0.0 }
        );
        assert_eq!(
            runtime.select(&world, 1, 1, locator_bar(), None, None),
            HudContextualBar::Locator(HudLocatorBar::default())
        );
        assert_eq!(
            runtime.select(
                &world,
                1,
                2,
                locator_bar(),
                Some(HudJumpBar {
                    progress: 0.0,
                    cooldown: false,
                }),
                None,
            ),
            HudContextualBar::Locator(HudLocatorBar::default())
        );
        assert_eq!(
            runtime.select(
                &world,
                1,
                3,
                locator_bar(),
                Some(HudJumpBar {
                    progress: 0.25,
                    cooldown: false,
                }),
                None,
            ),
            HudContextualBar::Jump(HudJumpBar {
                progress: 0.25,
                cooldown: false,
            })
        );
        assert_eq!(
            runtime.select(
                &world,
                1,
                4,
                locator_bar(),
                Some(HudJumpBar {
                    progress: 0.0,
                    cooldown: true,
                }),
                None,
            ),
            HudContextualBar::Jump(HudJumpBar {
                progress: 0.0,
                cooldown: true,
            })
        );
    }

    #[test]
    fn contextual_bar_prioritizes_only_subsequent_experience_changes_for_one_hundred_ticks() {
        let mut world = world_with_local_player(Uuid::from_u128(7));
        let mut runtime = HudContextualRuntime::default();
        world.apply_player_experience(PlayerExperience {
            progress: 0.5,
            level: 1,
            total: 1,
        });
        assert_eq!(
            runtime.select(&world, 1, 5, locator_bar(), None, Some(0.5)),
            HudContextualBar::Locator(HudLocatorBar::default())
        );

        world.apply_player_experience(PlayerExperience {
            progress: 0.75,
            level: 1,
            total: 2,
        });
        assert_eq!(
            runtime.select(&world, 1, 10, locator_bar(), None, Some(0.75)),
            HudContextualBar::Experience { progress: 0.75 }
        );
        assert_eq!(
            runtime.select(&world, 1, 109, locator_bar(), None, Some(0.75)),
            HudContextualBar::Experience { progress: 0.75 }
        );
        assert_eq!(
            runtime.select(&world, 1, 110, locator_bar(), None, Some(0.75)),
            HudContextualBar::Locator(HudLocatorBar::default())
        );

        // Respawn/level replacement resets the presentation clock. The
        // current progress becomes the new first observation without priority.
        assert_eq!(
            runtime.select(&world, 2, 111, locator_bar(), None, Some(0.75)),
            HudContextualBar::Locator(HudLocatorBar::default())
        );
    }

    #[test]
    fn contextual_bar_anvil_opening_restarts_but_does_not_extend_experience_priority() {
        let mut world = world_with_local_player(Uuid::from_u128(7));
        let mut runtime = HudContextualRuntime::default();
        assert_eq!(
            runtime.select(&world, 1, 0, locator_bar(), None, None),
            HudContextualBar::Locator(HudLocatorBar::default())
        );
        world.apply_open_screen(OpenScreen {
            container_id: 2,
            menu_type_id: bbb_protocol::menu_types::VANILLA_MENU_TYPE_ANVIL_ID,
            title: bbb_protocol::RichTextComponent::plain("Anvil"),
        });
        assert_eq!(
            runtime.select(&world, 1, 10, locator_bar(), None, None),
            HudContextualBar::Experience { progress: 0.0 }
        );
        assert_eq!(
            runtime.select(&world, 1, 109, locator_bar(), None, None),
            HudContextualBar::Experience { progress: 0.0 }
        );
        assert_eq!(
            runtime.select(&world, 1, 110, locator_bar(), None, None),
            HudContextualBar::Locator(HudLocatorBar::default())
        );

        assert!(world.close_local_container(2));
        let _ = runtime.select(&world, 1, 111, locator_bar(), None, None);
        world.apply_open_screen(OpenScreen {
            container_id: 2,
            menu_type_id: bbb_protocol::menu_types::VANILLA_MENU_TYPE_ENCHANTMENT_ID,
            title: bbb_protocol::RichTextComponent::plain("Enchant"),
        });
        assert_eq!(
            runtime.select(&world, 1, 112, locator_bar(), None, None),
            HudContextualBar::Experience { progress: 0.0 }
        );
    }

    #[test]
    fn contextual_bar_never_falls_back_to_experience_in_creative_or_spectator() {
        for game_type in [1, 3] {
            let world = world_with_game_type(Uuid::from_u128(7), game_type);
            let mut runtime = HudContextualRuntime::default();
            assert_eq!(
                runtime.select(&world, 1, 0, None, None, Some(0.5)),
                HudContextualBar::Empty
            );
            assert_eq!(
                runtime.select(&world, 1, 1, locator_bar(), None, Some(0.75)),
                HudContextualBar::Locator(HudLocatorBar::default())
            );
        }
    }
}
