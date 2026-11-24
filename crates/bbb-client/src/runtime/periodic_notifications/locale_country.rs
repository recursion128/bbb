#[cfg(not(target_arch = "wasm32"))]
use isocountry::CountryCode;

pub(in crate::runtime) fn host_country_iso3() -> Option<String> {
    country_iso3_from_locale(raw_host_locale().as_deref())
}

pub(super) fn country_iso3_from_locale(locale: Option<&str>) -> Option<String> {
    let locale = locale?
        .split(['.', '@'])
        .next()
        .unwrap_or_default()
        .replace('_', "-");
    locale.split('-').skip(1).find_map(resolve_country_iso3)
}

#[cfg(not(target_arch = "wasm32"))]
fn raw_host_locale() -> Option<String> {
    sys_locale::get_locale()
}

#[cfg(target_arch = "wasm32")]
fn raw_host_locale() -> Option<String> {
    web_sys::window().and_then(|win| win.navigator().language())
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_country_iso3(subtag: &str) -> Option<String> {
    match subtag.len() {
        2 if subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) => {
            CountryCode::for_alpha2_caseless(subtag).ok()
        }
        3 if subtag.bytes().all(|byte| byte.is_ascii_alphabetic()) => {
            CountryCode::for_alpha3_caseless(subtag).ok()
        }
        _ => None,
    }
    .map(|country| country.alpha3().to_string())
}

#[cfg(target_arch = "wasm32")]
fn resolve_country_iso3(subtag: &str) -> Option<String> {
    const ISO2_TO_ISO3: &[([u8; 2], [u8; 3])] = &[
        (*b"AD", *b"AND"),
        (*b"AE", *b"ARE"),
        (*b"AF", *b"AFG"),
        (*b"AG", *b"ATG"),
        (*b"AI", *b"AIA"),
        (*b"AL", *b"ALB"),
        (*b"AM", *b"ARM"),
        (*b"AO", *b"AGO"),
        (*b"AQ", *b"ATA"),
        (*b"AR", *b"ARG"),
        (*b"AS", *b"ASM"),
        (*b"AT", *b"AUT"),
        (*b"AU", *b"AUS"),
        (*b"AW", *b"ABW"),
        (*b"AX", *b"ALA"),
        (*b"AZ", *b"AZE"),
        (*b"BA", *b"BIH"),
        (*b"BB", *b"BRB"),
        (*b"BD", *b"BGD"),
        (*b"BE", *b"BEL"),
        (*b"BF", *b"BFA"),
        (*b"BG", *b"BGR"),
        (*b"BH", *b"BHR"),
        (*b"BI", *b"BDI"),
        (*b"BJ", *b"BEN"),
        (*b"BL", *b"BLM"),
        (*b"BM", *b"BMU"),
        (*b"BN", *b"BRN"),
        (*b"BO", *b"BOL"),
        (*b"BQ", *b"BES"),
        (*b"BR", *b"BRA"),
        (*b"BS", *b"BHS"),
        (*b"BT", *b"BTN"),
        (*b"BV", *b"BVT"),
        (*b"BW", *b"BWA"),
        (*b"BY", *b"BLR"),
        (*b"BZ", *b"BLZ"),
        (*b"CA", *b"CAN"),
        (*b"CC", *b"CCK"),
        (*b"CD", *b"COD"),
        (*b"CF", *b"CAF"),
        (*b"CG", *b"COG"),
        (*b"CH", *b"CHE"),
        (*b"CI", *b"CIV"),
        (*b"CK", *b"COK"),
        (*b"CL", *b"CHL"),
        (*b"CM", *b"CMR"),
        (*b"CN", *b"CHN"),
        (*b"CO", *b"COL"),
        (*b"CR", *b"CRI"),
        (*b"CU", *b"CUB"),
        (*b"CV", *b"CPV"),
        (*b"CW", *b"CUW"),
        (*b"CX", *b"CXR"),
        (*b"CY", *b"CYP"),
        (*b"CZ", *b"CZE"),
        (*b"DE", *b"DEU"),
        (*b"DJ", *b"DJI"),
        (*b"DK", *b"DNK"),
        (*b"DM", *b"DMA"),
        (*b"DO", *b"DOM"),
        (*b"DZ", *b"DZA"),
        (*b"EC", *b"ECU"),
        (*b"EE", *b"EST"),
        (*b"EG", *b"EGY"),
        (*b"EH", *b"ESH"),
        (*b"ER", *b"ERI"),
        (*b"ES", *b"ESP"),
        (*b"ET", *b"ETH"),
        (*b"FI", *b"FIN"),
        (*b"FJ", *b"FJI"),
        (*b"FK", *b"FLK"),
        (*b"FM", *b"FSM"),
        (*b"FO", *b"FRO"),
        (*b"FR", *b"FRA"),
        (*b"GA", *b"GAB"),
        (*b"GB", *b"GBR"),
        (*b"GD", *b"GRD"),
        (*b"GE", *b"GEO"),
        (*b"GF", *b"GUF"),
        (*b"GG", *b"GGY"),
        (*b"GH", *b"GHA"),
        (*b"GI", *b"GIB"),
        (*b"GL", *b"GRL"),
        (*b"GM", *b"GMB"),
        (*b"GN", *b"GIN"),
        (*b"GP", *b"GLP"),
        (*b"GQ", *b"GNQ"),
        (*b"GR", *b"GRC"),
        (*b"GS", *b"SGS"),
        (*b"GT", *b"GTM"),
        (*b"GU", *b"GUM"),
        (*b"GW", *b"GNB"),
        (*b"GY", *b"GUY"),
        (*b"HK", *b"HKG"),
        (*b"HM", *b"HMD"),
        (*b"HN", *b"HND"),
        (*b"HR", *b"HRV"),
        (*b"HT", *b"HTI"),
        (*b"HU", *b"HUN"),
        (*b"ID", *b"IDN"),
        (*b"IE", *b"IRL"),
        (*b"IL", *b"ISR"),
        (*b"IM", *b"IMN"),
        (*b"IN", *b"IND"),
        (*b"IO", *b"IOT"),
        (*b"IQ", *b"IRQ"),
        (*b"IR", *b"IRN"),
        (*b"IS", *b"ISL"),
        (*b"IT", *b"ITA"),
        (*b"JE", *b"JEY"),
        (*b"JM", *b"JAM"),
        (*b"JO", *b"JOR"),
        (*b"JP", *b"JPN"),
        (*b"KE", *b"KEN"),
        (*b"KG", *b"KGZ"),
        (*b"KH", *b"KHM"),
        (*b"KI", *b"KIR"),
        (*b"KM", *b"COM"),
        (*b"KN", *b"KNA"),
        (*b"KP", *b"PRK"),
        (*b"KR", *b"KOR"),
        (*b"KW", *b"KWT"),
        (*b"KY", *b"CYM"),
        (*b"KZ", *b"KAZ"),
        (*b"LA", *b"LAO"),
        (*b"LB", *b"LBN"),
        (*b"LC", *b"LCA"),
        (*b"LI", *b"LIE"),
        (*b"LK", *b"LKA"),
        (*b"LR", *b"LBR"),
        (*b"LS", *b"LSO"),
        (*b"LT", *b"LTU"),
        (*b"LU", *b"LUX"),
        (*b"LV", *b"LVA"),
        (*b"LY", *b"LBY"),
        (*b"MA", *b"MAR"),
        (*b"MC", *b"MCO"),
        (*b"MD", *b"MDA"),
        (*b"ME", *b"MNE"),
        (*b"MF", *b"MAF"),
        (*b"MG", *b"MDG"),
        (*b"MH", *b"MHL"),
        (*b"MK", *b"MKD"),
        (*b"ML", *b"MLI"),
        (*b"MM", *b"MMR"),
        (*b"MN", *b"MNG"),
        (*b"MO", *b"MAC"),
        (*b"MP", *b"MNP"),
        (*b"MQ", *b"MTQ"),
        (*b"MR", *b"MRT"),
        (*b"MS", *b"MSR"),
        (*b"MT", *b"MLT"),
        (*b"MU", *b"MUS"),
        (*b"MV", *b"MDV"),
        (*b"MW", *b"MWI"),
        (*b"MX", *b"MEX"),
        (*b"MY", *b"MYS"),
        (*b"MZ", *b"MOZ"),
        (*b"NA", *b"NAM"),
        (*b"NC", *b"NCL"),
        (*b"NE", *b"NER"),
        (*b"NF", *b"NFK"),
        (*b"NG", *b"NGA"),
        (*b"NI", *b"NIC"),
        (*b"NL", *b"NLD"),
        (*b"NO", *b"NOR"),
        (*b"NP", *b"NPL"),
        (*b"NR", *b"NRU"),
        (*b"NU", *b"NIU"),
        (*b"NZ", *b"NZL"),
        (*b"OM", *b"OMN"),
        (*b"PA", *b"PAN"),
        (*b"PE", *b"PER"),
        (*b"PF", *b"PYF"),
        (*b"PG", *b"PNG"),
        (*b"PH", *b"PHL"),
        (*b"PK", *b"PAK"),
        (*b"PL", *b"POL"),
        (*b"PM", *b"SPM"),
        (*b"PN", *b"PCN"),
        (*b"PR", *b"PRI"),
        (*b"PS", *b"PSE"),
        (*b"PT", *b"PRT"),
        (*b"PW", *b"PLW"),
        (*b"PY", *b"PRY"),
        (*b"QA", *b"QAT"),
        (*b"RE", *b"REU"),
        (*b"RO", *b"ROU"),
        (*b"RS", *b"SRB"),
        (*b"RU", *b"RUS"),
        (*b"RW", *b"RWA"),
        (*b"SA", *b"SAU"),
        (*b"SB", *b"SLB"),
        (*b"SC", *b"SYC"),
        (*b"SD", *b"SDN"),
        (*b"SE", *b"SWE"),
        (*b"SG", *b"SGP"),
        (*b"SH", *b"SHN"),
        (*b"SI", *b"SVN"),
        (*b"SJ", *b"SJM"),
        (*b"SK", *b"SVK"),
        (*b"SL", *b"SLE"),
        (*b"SM", *b"SMR"),
        (*b"SN", *b"SEN"),
        (*b"SO", *b"SOM"),
        (*b"SR", *b"SUR"),
        (*b"SS", *b"SSD"),
        (*b"ST", *b"STP"),
        (*b"SV", *b"SLV"),
        (*b"SX", *b"SXM"),
        (*b"SY", *b"SYR"),
        (*b"SZ", *b"SWZ"),
        (*b"TC", *b"TCA"),
        (*b"TD", *b"TCD"),
        (*b"TF", *b"ATF"),
        (*b"TG", *b"TGO"),
        (*b"TH", *b"THA"),
        (*b"TJ", *b"TJK"),
        (*b"TK", *b"TKL"),
        (*b"TL", *b"TLS"),
        (*b"TM", *b"TKM"),
        (*b"TN", *b"TUN"),
        (*b"TO", *b"TON"),
        (*b"TR", *b"TUR"),
        (*b"TT", *b"TTO"),
        (*b"TV", *b"TUV"),
        (*b"TW", *b"TWN"),
        (*b"TZ", *b"TZA"),
        (*b"UA", *b"UKR"),
        (*b"UG", *b"UGA"),
        (*b"UM", *b"UMI"),
        (*b"US", *b"USA"),
        (*b"UY", *b"URY"),
        (*b"UZ", *b"UZB"),
        (*b"VA", *b"VAT"),
        (*b"VC", *b"VCT"),
        (*b"VE", *b"VEN"),
        (*b"VG", *b"VGB"),
        (*b"VI", *b"VIR"),
        (*b"VN", *b"VNM"),
        (*b"VU", *b"VUT"),
        (*b"WF", *b"WLF"),
        (*b"WS", *b"WSM"),
        (*b"YE", *b"YEM"),
        (*b"YT", *b"MYT"),
        (*b"ZA", *b"ZAF"),
        (*b"ZM", *b"ZMB"),
        (*b"ZW", *b"ZWE"),
    ];

    match subtag.as_bytes() {
        [a, b] if a.is_ascii_alphabetic() && b.is_ascii_alphabetic() => {
            let key = [a.to_ascii_uppercase(), b.to_ascii_uppercase()];
            ISO2_TO_ISO3
                .binary_search_by_key(&key, |(alpha2, _)| *alpha2)
                .ok()
                .map(|idx| String::from_utf8_lossy(&ISO2_TO_ISO3[idx].1).into_owned())
        }
        [a, b, c]
            if a.is_ascii_alphabetic() && b.is_ascii_alphabetic() && c.is_ascii_alphabetic() =>
        {
            let iso3 = [
                a.to_ascii_uppercase(),
                b.to_ascii_uppercase(),
                c.to_ascii_uppercase(),
            ];
            ISO2_TO_ISO3
                .iter()
                .any(|(_, candidate)| *candidate == iso3)
                .then(|| String::from_utf8_lossy(&iso3).into_owned())
        }
        _ => None,
    }
}
