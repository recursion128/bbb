import com.ibm.icu.text.SimpleDateFormat;
import com.ibm.icu.util.Calendar;
import com.ibm.icu.util.TimeZone;
import com.ibm.icu.util.ULocale;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Date;
import java.util.LinkedHashSet;

/**
 * Golden-output generator that mirrors vanilla 26.1
 * net.minecraft.client.renderer.item.properties.select.LocalTime exactly:
 * ULocale(localeId) -> Calendar.getInstance(tz, locale) ->
 * SimpleDateFormat(pattern, locale).setCalendar(calendar) -> format(date).
 */
public class LocalTimeGolden {
    public static void main(String[] args) throws Exception {
        PrintStream out = new PrintStream(System.out, true, StandardCharsets.UTF_8);
        switch (args[0]) {
            case "goldens" -> goldens(out);
            case "weekdata" -> weekdata(out);
            case "numsys" -> numsys(out, args[1]);
            default -> throw new IllegalArgumentException(args[0]);
        }
    }

    // stdin TSV: pattern \t locale \t tz \t epochMillis ; appends \t OK \t result (or ERROR \t exception)
    static void goldens(PrintStream out) throws Exception {
        BufferedReader in = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
        String line;
        while ((line = in.readLine()) != null) {
            if (line.isBlank()) {
                continue;
            }
            String[] f = line.split("\t", -1);
            String pattern = f[0];
            String localeId = f[1];
            String tzId = f[2];
            long millis = Long.parseLong(f[3]);
            String result;
            try {
                ULocale locale = new ULocale(localeId);
                Calendar calendar = tzId.isEmpty()
                        ? Calendar.getInstance(locale)
                        : Calendar.getInstance(TimeZone.getTimeZone(tzId), locale);
                SimpleDateFormat fmt = new SimpleDateFormat(pattern, locale);
                fmt.setCalendar(calendar);
                result = "OK\t" + fmt.format(new Date(millis));
            } catch (Exception e) {
                result = "ERROR\t" + e.getClass().getSimpleName();
            }
            out.println(line + "\t" + result);
        }
    }

    // region \t firstDayOfWeek(1=Sunday..7=Saturday) \t minimalDaysInFirstWeek
    static void weekdata(PrintStream out) {
        LinkedHashSet<String> regions = new LinkedHashSet<>();
        regions.add("001");
        for (String r : ULocale.getISOCountries()) {
            regions.add(r);
        }
        for (String region : regions) {
            Calendar.WeekData wd = Calendar.getWeekDataForRegion(region);
            out.println(region + "\t" + wd.firstDayOfWeek + "\t" + wd.minimalDaysInFirstWeek);
        }
    }

    // locale \t numbering-system-name \t calendar-type
    static void numsys(PrintStream out, String localeListPath) throws Exception {
        for (String id : Files.readAllLines(Path.of(localeListPath), StandardCharsets.UTF_8)) {
            if (id.isBlank()) {
                continue;
            }
            ULocale locale = new ULocale(id.trim());
            String nu = com.ibm.icu.text.NumberingSystem.getInstance(locale).getName();
            String cal = Calendar.getInstance(locale).getType();
            out.println(id.trim() + "\t" + nu + "\t" + cal);
        }
    }
}
