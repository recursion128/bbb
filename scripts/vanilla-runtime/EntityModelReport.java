import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.Enumeration;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import net.minecraft.SharedConstants;
import net.minecraft.client.animation.AnimationChannel;
import net.minecraft.client.animation.AnimationDefinition;
import net.minecraft.client.animation.Keyframe;
import net.minecraft.client.model.geom.LayerDefinitions;
import net.minecraft.client.model.geom.ModelLayerLocation;
import net.minecraft.client.model.geom.PartPose;
import net.minecraft.client.model.geom.builders.CubeDefinition;
import net.minecraft.client.model.geom.builders.CubeDeformation;
import net.minecraft.client.model.geom.builders.LayerDefinition;
import net.minecraft.client.model.geom.builders.MaterialDefinition;
import net.minecraft.client.model.geom.builders.MeshDefinition;
import net.minecraft.client.model.geom.builders.PartDefinition;
import net.minecraft.client.model.geom.builders.UVPair;
import net.minecraft.core.Direction;
import net.minecraft.server.Bootstrap;
import org.joml.Vector3fc;

/**
 * Emits every model layer from {@code LayerDefinitions.createRoots()} and every keyframe
 * {@code AnimationDefinition} under {@code net.minecraft.client.animation.definitions}.
 *
 * <p>Floats are printed as raw IEEE bits so the generator reproduces them exactly. Children are
 * printed in {@code PartDefinition.children} iteration order, which is the order
 * {@code PartDefinition.bake} collects into the baked {@code ModelPart} and therefore its render
 * order.
 */
public final class EntityModelReport {
    private static final String ANIMATION_PACKAGE = "net/minecraft/client/animation/definitions/";

    private EntityModelReport() {
    }

    public static void main(String[] args) throws Exception {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        Map<String, LayerDefinition> layers = new TreeMap<>();
        for (Map.Entry<ModelLayerLocation, LayerDefinition> entry : LayerDefinitions.createRoots().entrySet()) {
            if (layers.put(entry.getKey().toString(), entry.getValue()) != null) {
                throw new IllegalStateException("duplicate layer " + entry.getKey());
            }
        }
        Bootstrap.STDOUT.println("version=26.1 layers=" + layers.size());
        for (Map.Entry<String, LayerDefinition> entry : layers.entrySet()) {
            LayerDefinition layer = entry.getValue();
            MaterialDefinition material = (MaterialDefinition) field(LayerDefinition.class, "material").get(layer);
            MeshDefinition mesh = (MeshDefinition) field(LayerDefinition.class, "mesh").get(layer);
            Bootstrap.STDOUT.println(
                "layer " + entry.getKey() + " "
                    + field(MaterialDefinition.class, "xTexSize").getInt(material) + " "
                    + field(MaterialDefinition.class, "yTexSize").getInt(material)
            );
            printPart(0, "root", mesh.getRoot());
        }
        for (String className : animationClasses(args[0])) {
            Class<?> type = Class.forName(className);
            for (Field field : type.getDeclaredFields()) {
                if (!Modifier.isStatic(field.getModifiers()) || field.getType() != AnimationDefinition.class) {
                    continue;
                }
                field.setAccessible(true);
                printAnimation(type.getSimpleName() + "." + field.getName(), (AnimationDefinition) field.get(null));
            }
        }
    }

    private static List<String> animationClasses(String clientJar) throws Exception {
        List<String> classes = new ArrayList<>();
        try (JarFile jar = new JarFile(clientJar)) {
            Enumeration<JarEntry> entries = jar.entries();
            while (entries.hasMoreElements()) {
                String name = entries.nextElement().getName();
                if (name.startsWith(ANIMATION_PACKAGE) && name.endsWith(".class") && !name.contains("$")
                    && name.indexOf('/', ANIMATION_PACKAGE.length()) < 0) {
                    classes.add(name.substring(0, name.length() - ".class".length()).replace('/', '.'));
                }
            }
        }
        classes.sort(null);
        return classes;
    }

    @SuppressWarnings("unchecked")
    private static void printPart(int depth, String name, PartDefinition part) throws Exception {
        PartPose pose = (PartPose) field(PartDefinition.class, "partPose").get(part);
        Bootstrap.STDOUT.println(
            "part " + depth + " " + name + " "
                + bits(pose.x()) + " " + bits(pose.y()) + " " + bits(pose.z()) + " "
                + bits(pose.xRot()) + " " + bits(pose.yRot()) + " " + bits(pose.zRot()) + " "
                + bits(pose.xScale()) + " " + bits(pose.yScale()) + " " + bits(pose.zScale())
        );
        for (CubeDefinition cube : (List<CubeDefinition>) field(PartDefinition.class, "cubes").get(part)) {
            Vector3fc origin = (Vector3fc) field(CubeDefinition.class, "origin").get(cube);
            Vector3fc dimensions = (Vector3fc) field(CubeDefinition.class, "dimensions").get(cube);
            CubeDeformation grow = (CubeDeformation) field(CubeDefinition.class, "grow").get(cube);
            UVPair texCoord = (UVPair) field(CubeDefinition.class, "texCoord").get(cube);
            UVPair texScale = (UVPair) field(CubeDefinition.class, "texScale").get(cube);
            int faces = 0;
            for (Direction direction : (Set<Direction>) field(CubeDefinition.class, "visibleFaces").get(cube)) {
                faces |= 1 << direction.get3DDataValue();
            }
            Bootstrap.STDOUT.println(
                "cube "
                    + bits(origin.x()) + " " + bits(origin.y()) + " " + bits(origin.z()) + " "
                    + bits(dimensions.x()) + " " + bits(dimensions.y()) + " " + bits(dimensions.z()) + " "
                    + bits(field(CubeDeformation.class, "growX").getFloat(grow)) + " "
                    + bits(field(CubeDeformation.class, "growY").getFloat(grow)) + " "
                    + bits(field(CubeDeformation.class, "growZ").getFloat(grow)) + " "
                    + bits((float) (int) texCoord.u()) + " " + bits((float) (int) texCoord.v()) + " "
                    + bits(texScale.u()) + " " + bits(texScale.v()) + " "
                    + (field(CubeDefinition.class, "mirror").getBoolean(cube) ? 1 : 0) + " " + faces
            );
        }
        for (Map.Entry<String, PartDefinition> child : part.getChildren()) {
            printPart(depth + 1, child.getKey(), child.getValue());
        }
    }

    private static void printAnimation(String name, AnimationDefinition definition) {
        Bootstrap.STDOUT.println(
            "animation " + name + " " + bits(definition.lengthInSeconds()) + " " + (definition.looping() ? 1 : 0)
        );
        for (Map.Entry<String, List<AnimationChannel>> bone : definition.boneAnimations().entrySet()) {
            Bootstrap.STDOUT.println("bone " + bone.getKey());
            for (AnimationChannel channel : bone.getValue()) {
                Bootstrap.STDOUT.println("channel " + target(channel.target()));
                for (Keyframe keyframe : channel.keyframes()) {
                    Bootstrap.STDOUT.println(
                        "key " + bits(keyframe.timestamp()) + " "
                            + vector(keyframe.preTarget()) + " " + vector(keyframe.postTarget()) + " "
                            + interpolation(keyframe.interpolation())
                    );
                }
            }
        }
    }

    private static String target(AnimationChannel.Target target) {
        if (target == AnimationChannel.Targets.POSITION) {
            return "position";
        }
        if (target == AnimationChannel.Targets.ROTATION) {
            return "rotation";
        }
        if (target == AnimationChannel.Targets.SCALE) {
            return "scale";
        }
        throw new IllegalStateException("unknown animation target " + target);
    }

    private static String interpolation(AnimationChannel.Interpolation interpolation) {
        if (interpolation == AnimationChannel.Interpolations.LINEAR) {
            return "linear";
        }
        if (interpolation == AnimationChannel.Interpolations.CATMULLROM) {
            return "catmullrom";
        }
        throw new IllegalStateException("unknown keyframe interpolation " + interpolation);
    }

    private static String vector(Vector3fc vector) {
        return bits(vector.x()) + " " + bits(vector.y()) + " " + bits(vector.z());
    }

    private static String bits(float value) {
        return Integer.toUnsignedString(Float.floatToRawIntBits(value));
    }

    private static Field field(Class<?> owner, String name) throws NoSuchFieldException {
        Field field = owner.getDeclaredField(name);
        field.setAccessible(true);
        return field;
    }
}
