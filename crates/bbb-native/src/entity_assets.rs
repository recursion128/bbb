use anyhow::{bail, Context, Result};
use bbb_pack::{PackRoots, ResourceLocation, SpriteImage};
use bbb_renderer::{entity_model_texture_refs, EntityModelTextureImage, EntityModelTextureRef};

pub(crate) fn load_entity_model_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: Option<&PackRoots>,
) {
    let Some(roots) = roots else {
        tracing::warn!(
            "continuing without vanilla entity textures because pack roots are unavailable"
        );
        return;
    };
    if let Err(err) = try_load_entity_model_textures(renderer, roots) {
        tracing::warn!(?err, "continuing without vanilla entity textures");
    }
}

fn try_load_entity_model_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
) -> Result<()> {
    let images = load_entity_model_texture_images(roots)?;
    renderer.upload_entity_model_textures(&images)?;
    tracing::info!(
        textures = images.len(),
        "loaded vanilla entity model textures"
    );
    Ok(())
}

fn load_entity_model_texture_images(roots: &PackRoots) -> Result<Vec<EntityModelTextureImage>> {
    entity_model_texture_refs()
        .iter()
        .copied()
        .map(|texture| load_entity_model_texture_image(roots, texture))
        .collect()
}

fn load_entity_model_texture_image(
    roots: &PackRoots,
    texture: EntityModelTextureRef,
) -> Result<EntityModelTextureImage> {
    let location = ResourceLocation::parse(texture.path)?;
    let resource = roots
        .resource_stack()
        .get_resource(&location)
        .with_context(|| format!("missing entity texture minecraft:{}", texture.path))?;
    let image = SpriteImage::from_png_file(entity_texture_id(texture.path), resource.path)?;
    if [image.width, image.height] != texture.size {
        bail!(
            "entity texture {} has {}x{}, expected {}x{}",
            texture.path,
            image.width,
            image.height,
            texture.size[0],
            texture.size[1]
        );
    }
    Ok(EntityModelTextureImage::new(texture, image.rgba))
}

fn entity_texture_id(path: &str) -> String {
    let path = path.strip_suffix(".png").unwrap_or(path);
    format!("minecraft:{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_vanilla_entity_textures_from_resource_stack() {
        let temp = unique_temp_dir("bbb-entity-textures");
        let sources = temp.join("sources").join("26.1");
        for texture in entity_model_texture_refs() {
            let texture_path = sources.join("assets").join("minecraft").join(texture.path);
            std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
            write_png(&texture_path, texture.size[0], texture.size[1]);
        }
        let roots = PackRoots {
            mc_code_root: temp.clone(),
            sources_dir: sources,
            assets_dir: temp.join("unused-assets"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };

        let images = load_entity_model_texture_images(&roots).unwrap();

        assert_eq!(
            images
                .iter()
                .map(|image| image.texture.path)
                .collect::<Vec<_>>(),
            vec![
                "textures/entity/player/wide/steve.png",
                "textures/entity/player/slim/steve.png",
                "textures/entity/sheep/sheep.png",
                "textures/entity/sheep/sheep_baby.png",
                "textures/entity/sheep/sheep_wool_undercoat.png",
                "textures/entity/sheep/sheep_wool.png",
                "textures/entity/sheep/sheep_wool_baby.png",
                "textures/entity/wolf/wolf.png",
                "textures/entity/wolf/wolf_tame.png",
                "textures/entity/wolf/wolf_angry.png",
                "textures/entity/wolf/wolf_baby.png",
                "textures/entity/wolf/wolf_tame_baby.png",
                "textures/entity/wolf/wolf_angry_baby.png",
                "textures/entity/wolf/wolf_collar.png",
                "textures/entity/wolf/wolf_collar_baby.png",
                "textures/entity/goat/goat.png",
                "textures/entity/goat/goat_baby.png",
                "textures/entity/bear/polarbear.png",
                "textures/entity/bear/polarbear_baby.png",
                "textures/entity/hoglin/hoglin.png",
                "textures/entity/hoglin/hoglin_baby.png",
                "textures/entity/hoglin/zoglin.png",
                "textures/entity/hoglin/zoglin_baby.png",
                "textures/entity/illager/ravager.png",
                "textures/entity/villager/villager.png",
                "textures/entity/villager/villager_baby.png",
                "textures/entity/wandering_trader/wandering_trader.png",
                "textures/entity/chicken/chicken_temperate.png",
                "textures/entity/chicken/chicken_temperate_baby.png",
                "textures/entity/chicken/chicken_warm.png",
                "textures/entity/chicken/chicken_warm_baby.png",
                "textures/entity/chicken/chicken_cold.png",
                "textures/entity/chicken/chicken_cold_baby.png",
                "textures/entity/pig/pig_temperate.png",
                "textures/entity/pig/pig_temperate_baby.png",
                "textures/entity/pig/pig_warm.png",
                "textures/entity/pig/pig_warm_baby.png",
                "textures/entity/pig/pig_cold.png",
                "textures/entity/pig/pig_cold_baby.png",
                "textures/entity/cow/cow_temperate.png",
                "textures/entity/cow/cow_temperate_baby.png",
                "textures/entity/cow/cow_warm.png",
                "textures/entity/cow/cow_warm_baby.png",
                "textures/entity/cow/cow_cold.png",
                "textures/entity/cow/cow_cold_baby.png",
                "textures/entity/skeleton/skeleton.png",
                "textures/entity/skeleton/stray.png",
                "textures/entity/skeleton/stray_overlay.png",
                "textures/entity/skeleton/parched.png",
                "textures/entity/skeleton/wither_skeleton.png",
                "textures/entity/skeleton/bogged.png",
                "textures/entity/skeleton/bogged_overlay.png",
                "textures/entity/creeper/creeper.png",
                "textures/entity/spider/spider.png",
                "textures/entity/spider/cave_spider.png",
                "textures/entity/spider/spider_eyes.png",
                "textures/entity/enderman/enderman.png",
                "textures/entity/enderman/enderman_eyes.png",
                "textures/entity/iron_golem/iron_golem.png",
                "textures/entity/snow_golem/snow_golem.png",
                "textures/entity/witch/witch.png",
                "textures/entity/slime/slime.png",
                "textures/entity/slime/magmacube.png",
                "textures/entity/ghast/ghast.png",
                "textures/entity/blaze/blaze.png",
                "textures/entity/endermite/endermite.png",
                "textures/entity/silverfish/silverfish.png",
                "textures/entity/lead_knot/lead_knot.png",
                "textures/entity/trident/trident.png",
                "textures/entity/illager/evoker_fangs.png",
                "textures/entity/tadpole/tadpole.png",
                "textures/entity/creaking/creaking.png",
                "textures/entity/creaking/creaking_eyes.png",
                "textures/entity/sniffer/sniffer.png",
                "textures/entity/parrot/parrot_red_blue.png",
                "textures/entity/parrot/parrot_blue.png",
                "textures/entity/parrot/parrot_green.png",
                "textures/entity/parrot/parrot_yellow_blue.png",
                "textures/entity/parrot/parrot_grey.png",
                "textures/entity/shulker/shulker.png",
                "textures/entity/shulker/shulker_white.png",
                "textures/entity/shulker/shulker_orange.png",
                "textures/entity/shulker/shulker_magenta.png",
                "textures/entity/shulker/shulker_light_blue.png",
                "textures/entity/shulker/shulker_yellow.png",
                "textures/entity/shulker/shulker_lime.png",
                "textures/entity/shulker/shulker_pink.png",
                "textures/entity/shulker/shulker_gray.png",
                "textures/entity/shulker/shulker_light_gray.png",
                "textures/entity/shulker/shulker_cyan.png",
                "textures/entity/shulker/shulker_purple.png",
                "textures/entity/shulker/shulker_blue.png",
                "textures/entity/shulker/shulker_brown.png",
                "textures/entity/shulker/shulker_green.png",
                "textures/entity/shulker/shulker_red.png",
                "textures/entity/shulker/shulker_black.png",
                "textures/entity/enderdragon/dragon.png",
                "textures/entity/enderdragon/dragon_eyes.png",
                "textures/entity/nautilus/nautilus.png",
                "textures/entity/nautilus/nautilus_baby.png",
                "textures/entity/panda/panda.png",
                "textures/entity/panda/panda_baby.png",
                "textures/entity/panda/panda_lazy.png",
                "textures/entity/panda/lazy_panda_baby.png",
                "textures/entity/panda/panda_worried.png",
                "textures/entity/panda/worried_panda_baby.png",
                "textures/entity/panda/panda_playful.png",
                "textures/entity/panda/playful_panda_baby.png",
                "textures/entity/panda/panda_brown.png",
                "textures/entity/panda/brown_panda_baby.png",
                "textures/entity/panda/panda_weak.png",
                "textures/entity/panda/weak_panda_baby.png",
                "textures/entity/panda/panda_aggressive.png",
                "textures/entity/panda/aggressive_panda_baby.png",
                "textures/entity/axolotl/axolotl_lucy.png",
                "textures/entity/axolotl/axolotl_lucy_baby.png",
                "textures/entity/axolotl/axolotl_wild.png",
                "textures/entity/axolotl/axolotl_wild_baby.png",
                "textures/entity/axolotl/axolotl_gold.png",
                "textures/entity/axolotl/axolotl_gold_baby.png",
                "textures/entity/axolotl/axolotl_cyan.png",
                "textures/entity/axolotl/axolotl_cyan_baby.png",
                "textures/entity/axolotl/axolotl_blue.png",
                "textures/entity/axolotl/axolotl_blue_baby.png",
                "textures/entity/fox/fox.png",
                "textures/entity/fox/fox_baby.png",
                "textures/entity/fox/fox_sleep.png",
                "textures/entity/fox/fox_sleep_baby.png",
                "textures/entity/fox/fox_snow.png",
                "textures/entity/fox/fox_snow_baby.png",
                "textures/entity/fox/fox_snow_sleep.png",
                "textures/entity/fox/fox_snow_sleep_baby.png",
                "textures/entity/rabbit/rabbit_brown.png",
                "textures/entity/rabbit/rabbit_brown_baby.png",
                "textures/entity/rabbit/rabbit_white.png",
                "textures/entity/rabbit/rabbit_white_baby.png",
                "textures/entity/rabbit/rabbit_black.png",
                "textures/entity/rabbit/rabbit_black_baby.png",
                "textures/entity/rabbit/rabbit_white_splotched.png",
                "textures/entity/rabbit/rabbit_white_splotched_baby.png",
                "textures/entity/rabbit/rabbit_gold.png",
                "textures/entity/rabbit/rabbit_gold_baby.png",
                "textures/entity/rabbit/rabbit_salt.png",
                "textures/entity/rabbit/rabbit_salt_baby.png",
                "textures/entity/rabbit/rabbit_caerbannog.png",
                "textures/entity/rabbit/rabbit_caerbannog_baby.png",
                "textures/entity/rabbit/rabbit_toast.png",
                "textures/entity/rabbit/rabbit_toast_baby.png",
                "textures/entity/cat/cat_tabby.png",
                "textures/entity/cat/cat_tabby_baby.png",
                "textures/entity/cat/cat_black.png",
                "textures/entity/cat/cat_black_baby.png",
                "textures/entity/cat/cat_red.png",
                "textures/entity/cat/cat_red_baby.png",
                "textures/entity/cat/cat_siamese.png",
                "textures/entity/cat/cat_siamese_baby.png",
                "textures/entity/cat/cat_british_shorthair.png",
                "textures/entity/cat/cat_british_shorthair_baby.png",
                "textures/entity/cat/cat_calico.png",
                "textures/entity/cat/cat_calico_baby.png",
                "textures/entity/cat/cat_persian.png",
                "textures/entity/cat/cat_persian_baby.png",
                "textures/entity/cat/cat_ragdoll.png",
                "textures/entity/cat/cat_ragdoll_baby.png",
                "textures/entity/cat/cat_white.png",
                "textures/entity/cat/cat_white_baby.png",
                "textures/entity/cat/cat_jellie.png",
                "textures/entity/cat/cat_jellie_baby.png",
                "textures/entity/cat/cat_all_black.png",
                "textures/entity/cat/cat_all_black_baby.png",
                "textures/entity/cat/ocelot.png",
                "textures/entity/cat/ocelot_baby.png",
                "textures/entity/cat/cat_collar.png",
                "textures/entity/cat/cat_collar_baby.png",
                "textures/entity/cow/mooshroom_red.png",
                "textures/entity/cow/mooshroom_red_baby.png",
                "textures/entity/projectiles/arrow.png",
                "textures/entity/llama/llama_spit.png",
                "textures/entity/shulker/spark.png",
                "textures/entity/wither/wither.png",
                "textures/entity/wither/wither_invulnerable.png",
                "textures/entity/projectiles/wind_charge.png",
                "textures/entity/guardian/guardian.png",
                "textures/entity/guardian/guardian_elder.png",
                "textures/entity/warden/warden.png",
                "textures/entity/frog/frog_temperate.png",
                "textures/entity/frog/frog_warm.png",
                "textures/entity/frog/frog_cold.png",
                "textures/entity/armadillo/armadillo.png",
                "textures/entity/armadillo/armadillo_baby.png",
                "textures/entity/phantom/phantom.png",
                "textures/entity/phantom/phantom_eyes.png",
                "textures/entity/fish/pufferfish.png",
                "textures/entity/ghast/happy_ghast.png",
                "textures/entity/minecart/minecart.png",
                "textures/entity/armorstand/armorstand.png",
                "textures/entity/zombie/zombie.png",
                "textures/entity/zombie/zombie_baby.png",
                "textures/entity/zombie/husk.png",
                "textures/entity/zombie/husk_baby.png",
                "textures/entity/zombie/drowned.png",
                "textures/entity/zombie/drowned_baby.png",
                "textures/entity/zombie_villager/zombie_villager.png",
                "textures/entity/zombie_villager/zombie_villager_baby.png",
                "textures/entity/piglin/piglin.png",
                "textures/entity/piglin/piglin_baby.png",
                "textures/entity/piglin/piglin_brute.png",
                "textures/entity/piglin/zombified_piglin.png",
                "textures/entity/piglin/zombified_piglin_baby.png",
                "textures/entity/illager/evoker.png",
                "textures/entity/illager/illusioner.png",
                "textures/entity/illager/pillager.png",
                "textures/entity/illager/vindicator.png",
                "textures/entity/boat/acacia.png",
                "textures/entity/chest_boat/acacia.png",
                "textures/entity/boat/bamboo.png",
                "textures/entity/chest_boat/bamboo.png",
                "textures/entity/boat/birch.png",
                "textures/entity/chest_boat/birch.png",
                "textures/entity/boat/cherry.png",
                "textures/entity/chest_boat/cherry.png",
                "textures/entity/boat/dark_oak.png",
                "textures/entity/chest_boat/dark_oak.png",
                "textures/entity/boat/jungle.png",
                "textures/entity/chest_boat/jungle.png",
                "textures/entity/boat/mangrove.png",
                "textures/entity/chest_boat/mangrove.png",
                "textures/entity/boat/oak.png",
                "textures/entity/chest_boat/oak.png",
                "textures/entity/boat/pale_oak.png",
                "textures/entity/chest_boat/pale_oak.png",
                "textures/entity/boat/spruce.png",
                "textures/entity/chest_boat/spruce.png",
                "textures/entity/llama/llama_creamy.png",
                "textures/entity/llama/llama_creamy_baby.png",
                "textures/entity/llama/llama_white.png",
                "textures/entity/llama/llama_white_baby.png",
                "textures/entity/llama/llama_brown.png",
                "textures/entity/llama/llama_brown_baby.png",
                "textures/entity/llama/llama_gray.png",
                "textures/entity/llama/llama_gray_baby.png",
                "textures/entity/camel/camel.png",
                "textures/entity/camel/camel_baby.png",
                "textures/entity/camel/camel_husk.png",
                "textures/entity/squid/squid.png",
                "textures/entity/squid/squid_baby.png",
                "textures/entity/squid/glow_squid.png",
                "textures/entity/squid/glow_squid_baby.png",
                "textures/entity/fish/cod.png",
                "textures/entity/fish/salmon.png",
                "textures/entity/fish/tropical_a.png",
                "textures/entity/fish/tropical_b.png",
                "textures/entity/illager/vex.png",
                "textures/entity/allay/allay.png",
                "textures/entity/strider/strider.png",
                "textures/entity/strider/strider_baby.png",
                "textures/entity/strider/strider_cold.png",
                "textures/entity/strider/strider_cold_baby.png",
                "textures/entity/turtle/turtle.png",
                "textures/entity/turtle/turtle_baby.png",
                "textures/entity/bat/bat.png",
                "textures/entity/bee/bee.png",
                "textures/entity/bee/bee_baby.png",
                "textures/entity/breeze/breeze.png",
                "textures/entity/dolphin/dolphin.png",
                "textures/entity/dolphin/dolphin_baby.png",
                "textures/entity/fish/tropical_a_pattern_1.png",
                "textures/entity/fish/tropical_a_pattern_2.png",
                "textures/entity/fish/tropical_a_pattern_3.png",
                "textures/entity/fish/tropical_a_pattern_4.png",
                "textures/entity/fish/tropical_a_pattern_5.png",
                "textures/entity/fish/tropical_a_pattern_6.png",
                "textures/entity/fish/tropical_b_pattern_1.png",
                "textures/entity/fish/tropical_b_pattern_2.png",
                "textures/entity/fish/tropical_b_pattern_3.png",
                "textures/entity/fish/tropical_b_pattern_4.png",
                "textures/entity/fish/tropical_b_pattern_5.png",
                "textures/entity/fish/tropical_b_pattern_6.png",
            ]
        );
        for image in images {
            assert_eq!(
                image.rgba.len(),
                usize::try_from(image.texture.size[0] * image.texture.size[1] * 4).unwrap()
            );
        }

        std::fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rejects_entity_texture_with_wrong_dimensions() {
        let temp = unique_temp_dir("bbb-entity-textures-bad");
        let sources = temp.join("sources").join("26.1");
        for texture in entity_model_texture_refs() {
            let texture_path = sources.join("assets").join("minecraft").join(texture.path);
            std::fs::create_dir_all(texture_path.parent().unwrap()).unwrap();
            let [width, height] = if texture.path == "textures/entity/sheep/sheep.png" {
                [32, 32]
            } else {
                texture.size
            };
            write_png(&texture_path, width, height);
        }
        let roots = PackRoots {
            mc_code_root: temp.clone(),
            sources_dir: sources,
            assets_dir: temp.join("unused-assets"),
            generated_assets_dir: None,
            resource_pack_dirs: Vec::new(),
        };

        let err = load_entity_model_texture_images(&roots).unwrap_err();

        assert!(err.to_string().contains("expected 64x32"));
        std::fs::remove_dir_all(temp).unwrap();
    }

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }

    fn write_png(path: &std::path::Path, width: u32, height: u32) {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([7, 11, 13, 255]));
        image.save(path).unwrap();
    }
}
