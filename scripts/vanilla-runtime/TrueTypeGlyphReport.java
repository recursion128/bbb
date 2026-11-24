import com.google.gson.Gson;
import com.mojang.blaze3d.font.GlyphBitmap;
import com.mojang.blaze3d.font.GlyphInfo;
import com.mojang.blaze3d.font.TrueTypeGlyphProvider;
import com.mojang.blaze3d.font.UnbakedGlyph;
import com.mojang.blaze3d.platform.NativeImage;
import java.nio.ByteBuffer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.gui.font.glyphs.BakedGlyph;
import net.minecraft.client.gui.font.providers.FreeTypeUtil;
import org.lwjgl.PointerBuffer;
import org.lwjgl.system.MemoryStack;
import org.lwjgl.system.MemoryUtil;
import org.lwjgl.util.freetype.FT_Face;
import org.lwjgl.util.freetype.FreeType;

/** Executes the official provider's metrics and NativeImage's grayscale copy. */
public final class TrueTypeGlyphReport {
    private TrueTypeGlyphReport() {
    }

    private static Object number(float value) {
        return Float.isFinite(value) ? value : Float.toString(value);
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 5) {
            throw new IllegalArgumentException("usage: TrueTypeGlyphReport <ttf> <size> <oversample> <shift-x> <shift-y>");
        }
        byte[] bytes = Files.readAllBytes(Path.of(args[0]));
        ByteBuffer memory = MemoryUtil.memAlloc(bytes.length).put(bytes).flip();
        FT_Face face;
        try (MemoryStack stack = MemoryStack.stackPush()) {
            PointerBuffer pointer = stack.mallocPointer(1);
            FreeTypeUtil.assertError(FreeType.FT_New_Memory_Face(FreeTypeUtil.getLibrary(), memory, 0L, pointer), "Initializing face");
            face = FT_Face.create(pointer.get());
        }
        if (!"TrueType".equals(FreeType.FT_Get_Font_Format(face))) {
            throw new IllegalArgumentException("not a TrueType face");
        }
        FreeTypeUtil.assertError(FreeType.FT_Select_Charmap(face, FreeType.FT_ENCODING_UNICODE), "Unicode charmap");
        List<Map<String, Object>> report = new ArrayList<>();
        try (TrueTypeGlyphProvider provider = new TrueTypeGlyphProvider(
            memory, face, Float.parseFloat(args[1]), Float.parseFloat(args[2]),
            Float.parseFloat(args[3]), Float.parseFloat(args[4]), ""
        )) {
            for (int codepoint : provider.getSupportedGlyphs().intStream().sorted().toArray()) {
                UnbakedGlyph glyph = provider.getGlyph(codepoint);
                Map<String, Object> observation = new LinkedHashMap<>();
                observation.put("codepoint", codepoint);
                observation.put("advance", number(glyph.info().getAdvance()));
                glyph.bake(new UnbakedGlyph.Stitcher() {
                    public BakedGlyph getMissing() {
                        return null;
                    }
                    public BakedGlyph stitch(GlyphInfo info, GlyphBitmap bitmap) {
                        int width = bitmap.getPixelWidth();
                        int height = bitmap.getPixelHeight();
                        observation.put("width", width);
                        observation.put("height", height);
                        observation.put("bearing_left", number(bitmap.getBearingLeft()));
                        observation.put("bearing_top", number(bitmap.getBearingTop()));
                        int index = FreeType.FT_Get_Char_Index(face, codepoint);
                        try (NativeImage image = new NativeImage(NativeImage.Format.LUMINANCE, width, height, false)) {
                            if (!image.copyFromFont(face, index)) {
                                throw new IllegalStateException("glyph bitmap copy failed");
                            }
                            List<Integer> coverage = new ArrayList<>();
                            for (int y = 0; y < height; y++) {
                                for (int x = 0; x < width; x++) {
                                    coverage.add(Byte.toUnsignedInt(image.getLuminanceOrAlpha(x, y)));
                                }
                            }
                            observation.put("coverage", coverage);
                        }
                        return null;
                    }
                });
                report.add(observation);
            }
        } finally {
            FreeTypeUtil.destroy();
        }
        System.out.println(new Gson().toJson(report));
    }
}
