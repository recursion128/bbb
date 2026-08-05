#!/usr/bin/env python3
"""Emit the local_time golden corpus TSV: pattern \t locale \t tz \t epochMillis."""
import datetime as dt

def ms(y, mo, d, h=0, mi=0, s=0, msec=0):
    t = dt.datetime(y, mo, d, h, mi, s, msec * 1000, tzinfo=dt.timezone.utc)
    return int(t.timestamp() * 1000)

T_MAIN = ms(2026, 8, 5, 12, 34, 56, 789)     # summer, DST on in northern zones
T_WINTER = ms(2026, 1, 1, 0, 30, 0, 0)       # year boundary, DST off
T_WEEK_EDGE = ms(2025, 12, 29, 23, 59, 59, 999)
T_JAN4 = ms(2026, 1, 4, 12, 0, 0, 0)         # week-rule discriminator
T_NOON_SH = ms(2026, 8, 5, 4, 0, 0, 0)       # 12:00 Asia/Shanghai
T_MIDNIGHT_SH = ms(2026, 8, 4, 16, 0, 0, 0)  # 00:00 Asia/Shanghai
T_Q1 = ms(2026, 2, 14, 9, 0, 0, 0)
T_Q2 = ms(2026, 5, 14, 9, 0, 0, 0)
T_Q4 = ms(2026, 11, 14, 9, 0, 0, 0)

ALL_TIMES = [T_MAIN, T_WINTER, T_WEEK_EDGE, T_JAN4]

SYMBOL_SWEEP = [
    "G", "GGGG", "GGGGG", "y", "yy", "yyy", "yyyy", "yyyyyy", "yyyyyyy", "u", "uu",
    "Y", "YY", "YYYY", "M", "MM", "MMM", "MMMM", "MMMMM", "L", "LL", "LLL", "LLLL", "LLLLL",
    "w", "ww", "W", "d", "dd", "D", "DDD", "F", "g", "E", "EE", "EEE", "EEEE", "EEEEE",
    "EEEEEE", "EEEEEEE", "e", "ee", "eee", "eeee", "eeeee", "eeeeee", "c", "cc", "ccc",
    "cccc", "ccccc", "cccccc", "a", "aa", "aaa", "aaaa", "aaaaa", "aaaaaa", "b", "bbbb",
    "bbbbb", "B", "BBBB", "h", "hh", "H", "HH", "k", "kk", "K", "KK", "m", "mm", "s", "ss",
    "S", "SS", "SSS", "SSSS", "SSSSSS", "A", "z", "zz", "zzz", "zzzz", "zzzzz", "Z", "ZZ",
    "ZZZ", "ZZZZ", "ZZZZZ", "O", "OO", "OOOO", "v", "vv", "vvv", "vvvv", "V", "VV", "VVV",
    "VVVV", "VVVVV", "x", "xx", "xxx", "xxxx", "xxxxx", "X", "XX", "XXX", "XXXX", "XXXXX",
    "Q", "QQ", "QQQ", "QQQQ", "QQQQQ", "q", "qq", "qqq", "qqqq", "qqqqq",
]

ZONE_PATTERNS = [p for p in SYMBOL_SWEEP if p[0] in "zZOvVxX"]
TEXT_PATTERNS = [p for p in SYMBOL_SWEEP if p[0] in "GMLEecabqQ"]
WEEK_PATTERNS = ["w", "ww", "W", "Y", "YY", "YYYY", "e", "ee", "c", "cc"]

TEXT_LOCALES = [
    "", "root", "en_us", "en_gb", "zh_cn", "zh_tw", "ja_jp", "ko_kr", "de_de", "fr_fr",
    "ru_ru", "ar_sa", "th_th", "fa_ir", "pt_br", "es_es", "hi_in", "fi_fi", "tr_tr",
    "vi_vn", "he_il", "id_id", "uk_ua", "el_gr", "cs_cz", "sv_se", "da_dk", "nl_nl",
    "pl_pl", "hu_hu", "it_it", "ro_ro",
]
WEEK_LOCALES = [
    "", "en_us", "en_gb", "en_au", "de_de", "fr_fr", "ar_sa", "zh_cn", "pt_br", "ru_ru",
    "th_th", "he_il", "fa_ir", "hi_in", "bn_bd",
]
ZONES = [
    "Asia/Shanghai", "America/Los_Angeles", "Europe/London", "UTC", "GMT+05:30",
    "Asia/Kolkata", "Etc/GMT-2", "America/St_Johns", "Asia/Tokyo", "Europe/Berlin",
    "Asia/Seoul", "Europe/Dublin", "America/Sao_Paulo", "Australia/Adelaide",
]
COMPOSITES = [
    "yyyy-MM-dd HH:mm:ss", "yyyy.MM.dd G 'at' HH:mm:ss zzz", "EEE, MMM d, ''yy",
    "h:mm a", "hh 'o''clock' a, zzzz", "K:mm a, z", "yyyyy.MMMMM.dd GGG hh:mm aaa",
    "EEE, d MMM yyyy HH:mm:ss Z", "yyMMddHHmmssZ", "yyyy-MM-dd'T'HH:mm:ss.SSSZ",
    "yyyy-MM-dd'T'HH:mm:ss.SSSXXX", "YYYY-'W'ww-u", "QQQQ yyyy", "MMMM d, y G",
    "'week' w 'of' YYYY", "d MMMM y", "H:mm:ss.SSS A", "M/d/yy F E g D",
]
ERROR_PROBES = ["j", "J", "C", "l", "p", "i", "n", "t", "R", "T", "f", "o", "r", "I",
                "wwww", "WW", "FF", "kkk", "KKK", "hhh", "HHH", "mmm", "sss", "ddd",
                "DDDD", "ggg", "AA", "uuuu", "yyyyyyyyyyy", "VVVVVV", "OOO", "xxxxxx",
                "XXXXXX", "zzzzzz", "ZZZZZZ", "eeeeeee", "ccccccc", "MMMMMM", "LLLLLL",
                "GGGGGG", "QQQQQQ", "qqqqqq", "bbbbbb", "BBBBB", "BB", "SSSSSSSSSSS"]

lines = []
def emit(pattern, locale, tz, millis):
    lines.append(f"{pattern}\t{locale}\t{tz}\t{millis}")

# 1. Full symbol sweep on the anchor combos.
for locale in ["en_us", "zh_cn", "", "th_th", "fa_ir", "ar_sa"]:
    for p in SYMBOL_SWEEP:
        emit(p, locale, "Asia/Shanghai", T_MAIN)

# 2. Zone-name surface across zones/seasons/locales.
for locale in ["en_us", "zh_cn", "ru_ru", "de_de", ""]:
    for tz in ZONES:
        for p in ZONE_PATTERNS:
            for t in [T_MAIN, T_WINTER]:
                emit(p, locale, tz, t)

# 3. Text symbols across the wider locale set.
for locale in TEXT_LOCALES:
    for p in TEXT_PATTERNS:
        emit(p, locale, "Asia/Shanghai", T_MAIN)

# 4. Week machinery across locales and edge dates.
for locale in WEEK_LOCALES:
    for p in WEEK_PATTERNS:
        for t in ALL_TIMES:
            emit(p, locale, "Asia/Shanghai", t)
            emit(p, locale, "America/Los_Angeles", t)

# 5. Numbering systems and calendars.
for locale in ["ar_sa", "fa_ir", "th_th", "hi_in", "bn_bd", "zh_CN@numbers=hanidec",
               "th_TH@numbers=thai", "ja_JP@calendar=japanese", "th_TH@calendar=gregorian",
               "fa_IR@calendar=gregorian", "he_IL@calendar=hebrew", "zh_TW@calendar=roc",
               "am_ET@calendar=ethiopic", "hi_IN@calendar=indian", "ar_SA@calendar=islamic-umalqura"]:
    for p in ["y", "yy", "yyyy", "G", "GGGG", "M", "MM", "MMMM", "d", "dd", "HH:mm:ss",
              "w", "e", "D", "F", "yyyy-MM-dd G HH:mm"]:
        emit(p, locale, "Asia/Shanghai", T_MAIN)

# 6. Day-period edges (noon/midnight local).
for locale in ["en_us", "zh_cn", ""]:
    for p in ["a", "aaaa", "aaaaa", "b", "bbbb", "bbbbb", "h a", "K a", "k", "H"]:
        emit(p, locale, "Asia/Shanghai", T_NOON_SH)
        emit(p, locale, "Asia/Shanghai", T_MIDNIGHT_SH)

# 7. Quarter dates.
for locale in ["en_us", "zh_cn", "de_de", "fr_fr", "ru_ru", "ja_jp", ""]:
    for p in ["Q", "QQ", "QQQ", "QQQQ", "QQQQQ", "q", "qq", "qqq", "qqqq", "qqqqq"]:
        for t in [T_Q1, T_Q2, T_MAIN, T_Q4]:
            emit(p, locale, "Asia/Shanghai", t)

# 8. Composite realistic patterns.
for locale in ["en_us", "zh_cn", "ja_jp", "de_de", ""]:
    for p in COMPOSITES:
        emit(p, locale, "Asia/Shanghai", T_MAIN)
        emit(p, locale, "America/Los_Angeles", T_WINTER)

# 9. Quoting and literal edges.
for p in ["'literal' yyyy", "''", "'''y'''", "'it''s' h a", "yyyy'年'M'月'd'日'",
          "'unterminated", "a'b", "'--'HH'--'"]:
    emit(p, "en_us", "Asia/Shanghai", T_MAIN)

# 10. Locale-id parsing edges.
for locale in ["EN_US", "zh-CN", "zh_hans_cn", "sr_latn_rs", "sr_cyrl_rs", "xx_yy",
               "nonsense", "en_US_POSIX", "no_no", "nb_no", "iw_il", "in_id", "tl_ph",
               "be_latn", "es_419"]:
    emit("MMMM y EEEE a", locale, "Asia/Shanghai", T_MAIN)

# 11. Unknown/invalid pattern letters and overlong widths.
for p in ERROR_PROBES:
    emit(p, "en_us", "Asia/Shanghai", T_MAIN)

# 12. Fixed-offset and odd time zone ids (vanilla TimeZone.getTimeZone semantics).
for tz in ["GMT+3", "GMT-0230", "GMT+05:30", "UTC", "GMT", "Etc/UTC", "Etc/GMT+5",
           "America/Argentina/Buenos_Aires", "Asia/Calcutta", "US/Pacific",
           "Antarctica/Troll", "Pacific/Chatham", "Australia/Lord_Howe", "Asia/Kathmandu"]:
    for p in ["z", "zzzz", "v", "vvvv", "V", "VV", "VVV", "VVVV", "O", "OOOO", "X", "XXXXX",
              "Z", "ZZZZ", "ZZZZZ", "HH:mm"]:
        emit(p, "en_us", tz, T_MAIN)
        emit(p, "zh_cn", tz, T_MAIN)

seen = set()
out = []
for line in lines:
    if line not in seen:
        seen.add(line)
        out.append(line)
print("\n".join(out))
