import java.nio.file.Path;
import net.minecraft.SharedConstants;
import net.minecraft.data.CachedOutput;
import net.minecraft.data.PackOutput;
import net.minecraft.data.info.RegistryDumpReport;
import net.minecraft.server.Bootstrap;
import net.minecraft.util.Util;

public final class StaticRegistryReport {
    private StaticRegistryReport() {
    }

    public static void main(String[] args) {
        SharedConstants.tryDetectVersion();
        Bootstrap.bootStrap();
        try {
            // RegistryDumpReport.java:37-45 obtains each holder's actual raw ID.
            new RegistryDumpReport(new PackOutput(Path.of(args[0])))
                .run(CachedOutput.NO_CACHE).join();
        } finally {
            Util.shutdownExecutors();
        }
    }
}
