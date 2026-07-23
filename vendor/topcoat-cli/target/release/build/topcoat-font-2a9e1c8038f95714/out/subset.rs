/// A character subset a font family can ship.
///
/// One variant is generated per distinct subset in the vendored Fontsource
/// catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Subset {
    /// The `adlam` subset.
    Adlam,
    /// The `ahom` subset.
    Ahom,
    /// The `anatolian-hieroglyphs` subset.
    AnatolianHieroglyphs,
    /// The `arabic` subset.
    Arabic,
    /// The `armenian` subset.
    Armenian,
    /// The `avestan` subset.
    Avestan,
    /// The `balinese` subset.
    Balinese,
    /// The `bamum` subset.
    Bamum,
    /// The `bassa-vah` subset.
    BassaVah,
    /// The `batak` subset.
    Batak,
    /// The `bengali` subset.
    Bengali,
    /// The `beria-erfe` subset.
    BeriaErfe,
    /// The `bhaiksuki` subset.
    Bhaiksuki,
    /// The `brahmi` subset.
    Brahmi,
    /// The `braille` subset.
    Braille,
    /// The `buginese` subset.
    Buginese,
    /// The `buhid` subset.
    Buhid,
    /// The `canadian-aboriginal` subset.
    CanadianAboriginal,
    /// The `carian` subset.
    Carian,
    /// The `caucasian-albanian` subset.
    CaucasianAlbanian,
    /// The `chakma` subset.
    Chakma,
    /// The `cham` subset.
    Cham,
    /// The `cherokee` subset.
    Cherokee,
    /// The `chinese-hongkong` subset.
    ChineseHongkong,
    /// The `chinese-simplified` subset.
    ChineseSimplified,
    /// The `chinese-traditional` subset.
    ChineseTraditional,
    /// The `chorasmian` subset.
    Chorasmian,
    /// The `coptic` subset.
    Coptic,
    /// The `cuneiform` subset.
    Cuneiform,
    /// The `cypriot` subset.
    Cypriot,
    /// The `cypro-minoan` subset.
    CyproMinoan,
    /// The `cyrillic` subset.
    Cyrillic,
    /// The `cyrillic-ext` subset.
    CyrillicExt,
    /// The `deseret` subset.
    Deseret,
    /// The `devanagari` subset.
    Devanagari,
    /// The `dives-akuru` subset.
    DivesAkuru,
    /// The `dogra` subset.
    Dogra,
    /// The `duployan` subset.
    Duployan,
    /// The `egyptian-hieroglyphs` subset.
    EgyptianHieroglyphs,
    /// The `elbasan` subset.
    Elbasan,
    /// The `elymaic` subset.
    Elymaic,
    /// The `emoji` subset.
    Emoji,
    /// The `ethiopic` subset.
    Ethiopic,
    /// The `georgian` subset.
    Georgian,
    /// The `glagolitic` subset.
    Glagolitic,
    /// The `gothic` subset.
    Gothic,
    /// The `grantha` subset.
    Grantha,
    /// The `greek` subset.
    Greek,
    /// The `greek-ext` subset.
    GreekExt,
    /// The `gujarati` subset.
    Gujarati,
    /// The `gunjala-gondi` subset.
    GunjalaGondi,
    /// The `gurmukhi` subset.
    Gurmukhi,
    /// The `hanifi-rohingya` subset.
    HanifiRohingya,
    /// The `hanunoo` subset.
    Hanunoo,
    /// The `hatran` subset.
    Hatran,
    /// The `hebrew` subset.
    Hebrew,
    /// The `imperial-aramaic` subset.
    ImperialAramaic,
    /// The `indic-siyaq-numbers` subset.
    IndicSiyaqNumbers,
    /// The `inscriptional-pahlavi` subset.
    InscriptionalPahlavi,
    /// The `inscriptional-parthian` subset.
    InscriptionalParthian,
    /// The `japanese` subset.
    Japanese,
    /// The `javanese` subset.
    Javanese,
    /// The `kaithi` subset.
    Kaithi,
    /// The `kana-extended` subset.
    KanaExtended,
    /// The `kannada` subset.
    Kannada,
    /// The `kawi` subset.
    Kawi,
    /// The `kayah-li` subset.
    KayahLi,
    /// The `kharoshthi` subset.
    Kharoshthi,
    /// The `khitan-small-script` subset.
    KhitanSmallScript,
    /// The `khmer` subset.
    Khmer,
    /// The `khojki` subset.
    Khojki,
    /// The `khudawadi` subset.
    Khudawadi,
    /// The `kirat-rai` subset.
    KiratRai,
    /// The `korean` subset.
    Korean,
    /// The `lao` subset.
    Lao,
    /// The `latin` subset.
    Latin,
    /// The `latin-ext` subset.
    LatinExt,
    /// The `lepcha` subset.
    Lepcha,
    /// The `limbu` subset.
    Limbu,
    /// The `linear-a` subset.
    LinearA,
    /// The `linear-b` subset.
    LinearB,
    /// The `lisu` subset.
    Lisu,
    /// The `lycian` subset.
    Lycian,
    /// The `lydian` subset.
    Lydian,
    /// The `mahajani` subset.
    Mahajani,
    /// The `makasar` subset.
    Makasar,
    /// The `malayalam` subset.
    Malayalam,
    /// The `mandaic` subset.
    Mandaic,
    /// The `manichaean` subset.
    Manichaean,
    /// The `marchen` subset.
    Marchen,
    /// The `masaram-gondi` subset.
    MasaramGondi,
    /// The `math` subset.
    Math,
    /// The `mayan-numerals` subset.
    MayanNumerals,
    /// The `medefaidrin` subset.
    Medefaidrin,
    /// The `meetei-mayek` subset.
    MeeteiMayek,
    /// The `mende-kikakui` subset.
    MendeKikakui,
    /// The `meroitic` subset.
    Meroitic,
    /// The `meroitic-cursive` subset.
    MeroiticCursive,
    /// The `meroitic-hieroglyphs` subset.
    MeroiticHieroglyphs,
    /// The `miao` subset.
    Miao,
    /// The `modi` subset.
    Modi,
    /// The `mongolian` subset.
    Mongolian,
    /// The `mro` subset.
    Mro,
    /// The `multani` subset.
    Multani,
    /// The `music` subset.
    Music,
    /// The `myanmar` subset.
    Myanmar,
    /// The `nabataean` subset.
    Nabataean,
    /// The `nag-mundari` subset.
    NagMundari,
    /// The `nandinagari` subset.
    Nandinagari,
    /// The `new-tai-lue` subset.
    NewTaiLue,
    /// The `newa` subset.
    Newa,
    /// The `nko` subset.
    Nko,
    /// The `nushu` subset.
    Nushu,
    /// The `nyiakeng-puachue-hmong` subset.
    NyiakengPuachueHmong,
    /// The `ogham` subset.
    Ogham,
    /// The `ol-chiki` subset.
    OlChiki,
    /// The `old-hungarian` subset.
    OldHungarian,
    /// The `old-italic` subset.
    OldItalic,
    /// The `old-north-arabian` subset.
    OldNorthArabian,
    /// The `old-permic` subset.
    OldPermic,
    /// The `old-persian` subset.
    OldPersian,
    /// The `old-sogdian` subset.
    OldSogdian,
    /// The `old-south-arabian` subset.
    OldSouthArabian,
    /// The `old-turkic` subset.
    OldTurkic,
    /// The `old-uyghur` subset.
    OldUyghur,
    /// The `oriya` subset.
    Oriya,
    /// The `osage` subset.
    Osage,
    /// The `osmanya` subset.
    Osmanya,
    /// The `ottoman-siyaq-numbers` subset.
    OttomanSiyaqNumbers,
    /// The `pahawh-hmong` subset.
    PahawhHmong,
    /// The `palmyrene` subset.
    Palmyrene,
    /// The `pau-cin-hau` subset.
    PauCinHau,
    /// The `phags-pa` subset.
    PhagsPa,
    /// The `phoenician` subset.
    Phoenician,
    /// The `psalter-pahlavi` subset.
    PsalterPahlavi,
    /// The `rejang` subset.
    Rejang,
    /// The `runic` subset.
    Runic,
    /// The `samaritan` subset.
    Samaritan,
    /// The `saurashtra` subset.
    Saurashtra,
    /// The `sharada` subset.
    Sharada,
    /// The `shavian` subset.
    Shavian,
    /// The `siddham` subset.
    Siddham,
    /// The `signwriting` subset.
    Signwriting,
    /// The `sinhala` subset.
    Sinhala,
    /// The `sogdian` subset.
    Sogdian,
    /// The `sora-sompeng` subset.
    SoraSompeng,
    /// The `soyombo` subset.
    Soyombo,
    /// The `sundanese` subset.
    Sundanese,
    /// The `sunuwar` subset.
    Sunuwar,
    /// The `syloti-nagri` subset.
    SylotiNagri,
    /// The `symbols` subset.
    Symbols,
    /// The `symbols2` subset.
    Symbols2,
    /// The `syriac` subset.
    Syriac,
    /// The `tagalog` subset.
    Tagalog,
    /// The `tagbanwa` subset.
    Tagbanwa,
    /// The `tai-le` subset.
    TaiLe,
    /// The `tai-tham` subset.
    TaiTham,
    /// The `tai-viet` subset.
    TaiViet,
    /// The `takri` subset.
    Takri,
    /// The `tamil` subset.
    Tamil,
    /// The `tamil-supplement` subset.
    TamilSupplement,
    /// The `tangsa` subset.
    Tangsa,
    /// The `tangut` subset.
    Tangut,
    /// The `telugu` subset.
    Telugu,
    /// The `thaana` subset.
    Thaana,
    /// The `thai` subset.
    Thai,
    /// The `tibetan` subset.
    Tibetan,
    /// The `tifinagh` subset.
    Tifinagh,
    /// The `tirhuta` subset.
    Tirhuta,
    /// The `todhri` subset.
    Todhri,
    /// The `toto` subset.
    Toto,
    /// The `ugaritic` subset.
    Ugaritic,
    /// The `vai` subset.
    Vai,
    /// The `vietnamese` subset.
    Vietnamese,
    /// The `vithkuqi` subset.
    Vithkuqi,
    /// The `wancho` subset.
    Wancho,
    /// The `warang-citi` subset.
    WarangCiti,
    /// The `yezidi` subset.
    Yezidi,
    /// The `yi` subset.
    Yi,
    /// The `zanabazar-square` subset.
    ZanabazarSquare,
    /// The `znamenny` subset.
    Znamenny,
}

impl Subset {
    /// The Fontsource subset id, e.g. `"latin-ext"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Adlam => "adlam",
            Self::Ahom => "ahom",
            Self::AnatolianHieroglyphs => "anatolian-hieroglyphs",
            Self::Arabic => "arabic",
            Self::Armenian => "armenian",
            Self::Avestan => "avestan",
            Self::Balinese => "balinese",
            Self::Bamum => "bamum",
            Self::BassaVah => "bassa-vah",
            Self::Batak => "batak",
            Self::Bengali => "bengali",
            Self::BeriaErfe => "beria-erfe",
            Self::Bhaiksuki => "bhaiksuki",
            Self::Brahmi => "brahmi",
            Self::Braille => "braille",
            Self::Buginese => "buginese",
            Self::Buhid => "buhid",
            Self::CanadianAboriginal => "canadian-aboriginal",
            Self::Carian => "carian",
            Self::CaucasianAlbanian => "caucasian-albanian",
            Self::Chakma => "chakma",
            Self::Cham => "cham",
            Self::Cherokee => "cherokee",
            Self::ChineseHongkong => "chinese-hongkong",
            Self::ChineseSimplified => "chinese-simplified",
            Self::ChineseTraditional => "chinese-traditional",
            Self::Chorasmian => "chorasmian",
            Self::Coptic => "coptic",
            Self::Cuneiform => "cuneiform",
            Self::Cypriot => "cypriot",
            Self::CyproMinoan => "cypro-minoan",
            Self::Cyrillic => "cyrillic",
            Self::CyrillicExt => "cyrillic-ext",
            Self::Deseret => "deseret",
            Self::Devanagari => "devanagari",
            Self::DivesAkuru => "dives-akuru",
            Self::Dogra => "dogra",
            Self::Duployan => "duployan",
            Self::EgyptianHieroglyphs => "egyptian-hieroglyphs",
            Self::Elbasan => "elbasan",
            Self::Elymaic => "elymaic",
            Self::Emoji => "emoji",
            Self::Ethiopic => "ethiopic",
            Self::Georgian => "georgian",
            Self::Glagolitic => "glagolitic",
            Self::Gothic => "gothic",
            Self::Grantha => "grantha",
            Self::Greek => "greek",
            Self::GreekExt => "greek-ext",
            Self::Gujarati => "gujarati",
            Self::GunjalaGondi => "gunjala-gondi",
            Self::Gurmukhi => "gurmukhi",
            Self::HanifiRohingya => "hanifi-rohingya",
            Self::Hanunoo => "hanunoo",
            Self::Hatran => "hatran",
            Self::Hebrew => "hebrew",
            Self::ImperialAramaic => "imperial-aramaic",
            Self::IndicSiyaqNumbers => "indic-siyaq-numbers",
            Self::InscriptionalPahlavi => "inscriptional-pahlavi",
            Self::InscriptionalParthian => "inscriptional-parthian",
            Self::Japanese => "japanese",
            Self::Javanese => "javanese",
            Self::Kaithi => "kaithi",
            Self::KanaExtended => "kana-extended",
            Self::Kannada => "kannada",
            Self::Kawi => "kawi",
            Self::KayahLi => "kayah-li",
            Self::Kharoshthi => "kharoshthi",
            Self::KhitanSmallScript => "khitan-small-script",
            Self::Khmer => "khmer",
            Self::Khojki => "khojki",
            Self::Khudawadi => "khudawadi",
            Self::KiratRai => "kirat-rai",
            Self::Korean => "korean",
            Self::Lao => "lao",
            Self::Latin => "latin",
            Self::LatinExt => "latin-ext",
            Self::Lepcha => "lepcha",
            Self::Limbu => "limbu",
            Self::LinearA => "linear-a",
            Self::LinearB => "linear-b",
            Self::Lisu => "lisu",
            Self::Lycian => "lycian",
            Self::Lydian => "lydian",
            Self::Mahajani => "mahajani",
            Self::Makasar => "makasar",
            Self::Malayalam => "malayalam",
            Self::Mandaic => "mandaic",
            Self::Manichaean => "manichaean",
            Self::Marchen => "marchen",
            Self::MasaramGondi => "masaram-gondi",
            Self::Math => "math",
            Self::MayanNumerals => "mayan-numerals",
            Self::Medefaidrin => "medefaidrin",
            Self::MeeteiMayek => "meetei-mayek",
            Self::MendeKikakui => "mende-kikakui",
            Self::Meroitic => "meroitic",
            Self::MeroiticCursive => "meroitic-cursive",
            Self::MeroiticHieroglyphs => "meroitic-hieroglyphs",
            Self::Miao => "miao",
            Self::Modi => "modi",
            Self::Mongolian => "mongolian",
            Self::Mro => "mro",
            Self::Multani => "multani",
            Self::Music => "music",
            Self::Myanmar => "myanmar",
            Self::Nabataean => "nabataean",
            Self::NagMundari => "nag-mundari",
            Self::Nandinagari => "nandinagari",
            Self::NewTaiLue => "new-tai-lue",
            Self::Newa => "newa",
            Self::Nko => "nko",
            Self::Nushu => "nushu",
            Self::NyiakengPuachueHmong => "nyiakeng-puachue-hmong",
            Self::Ogham => "ogham",
            Self::OlChiki => "ol-chiki",
            Self::OldHungarian => "old-hungarian",
            Self::OldItalic => "old-italic",
            Self::OldNorthArabian => "old-north-arabian",
            Self::OldPermic => "old-permic",
            Self::OldPersian => "old-persian",
            Self::OldSogdian => "old-sogdian",
            Self::OldSouthArabian => "old-south-arabian",
            Self::OldTurkic => "old-turkic",
            Self::OldUyghur => "old-uyghur",
            Self::Oriya => "oriya",
            Self::Osage => "osage",
            Self::Osmanya => "osmanya",
            Self::OttomanSiyaqNumbers => "ottoman-siyaq-numbers",
            Self::PahawhHmong => "pahawh-hmong",
            Self::Palmyrene => "palmyrene",
            Self::PauCinHau => "pau-cin-hau",
            Self::PhagsPa => "phags-pa",
            Self::Phoenician => "phoenician",
            Self::PsalterPahlavi => "psalter-pahlavi",
            Self::Rejang => "rejang",
            Self::Runic => "runic",
            Self::Samaritan => "samaritan",
            Self::Saurashtra => "saurashtra",
            Self::Sharada => "sharada",
            Self::Shavian => "shavian",
            Self::Siddham => "siddham",
            Self::Signwriting => "signwriting",
            Self::Sinhala => "sinhala",
            Self::Sogdian => "sogdian",
            Self::SoraSompeng => "sora-sompeng",
            Self::Soyombo => "soyombo",
            Self::Sundanese => "sundanese",
            Self::Sunuwar => "sunuwar",
            Self::SylotiNagri => "syloti-nagri",
            Self::Symbols => "symbols",
            Self::Symbols2 => "symbols2",
            Self::Syriac => "syriac",
            Self::Tagalog => "tagalog",
            Self::Tagbanwa => "tagbanwa",
            Self::TaiLe => "tai-le",
            Self::TaiTham => "tai-tham",
            Self::TaiViet => "tai-viet",
            Self::Takri => "takri",
            Self::Tamil => "tamil",
            Self::TamilSupplement => "tamil-supplement",
            Self::Tangsa => "tangsa",
            Self::Tangut => "tangut",
            Self::Telugu => "telugu",
            Self::Thaana => "thaana",
            Self::Thai => "thai",
            Self::Tibetan => "tibetan",
            Self::Tifinagh => "tifinagh",
            Self::Tirhuta => "tirhuta",
            Self::Todhri => "todhri",
            Self::Toto => "toto",
            Self::Ugaritic => "ugaritic",
            Self::Vai => "vai",
            Self::Vietnamese => "vietnamese",
            Self::Vithkuqi => "vithkuqi",
            Self::Wancho => "wancho",
            Self::WarangCiti => "warang-citi",
            Self::Yezidi => "yezidi",
            Self::Yi => "yi",
            Self::ZanabazarSquare => "zanabazar-square",
            Self::Znamenny => "znamenny",
        }
    }

    /// Parse a Fontsource subset id, returning `None` if it is not in the
    /// vendored catalog.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "adlam" => Self::Adlam,
            "ahom" => Self::Ahom,
            "anatolian-hieroglyphs" => Self::AnatolianHieroglyphs,
            "arabic" => Self::Arabic,
            "armenian" => Self::Armenian,
            "avestan" => Self::Avestan,
            "balinese" => Self::Balinese,
            "bamum" => Self::Bamum,
            "bassa-vah" => Self::BassaVah,
            "batak" => Self::Batak,
            "bengali" => Self::Bengali,
            "beria-erfe" => Self::BeriaErfe,
            "bhaiksuki" => Self::Bhaiksuki,
            "brahmi" => Self::Brahmi,
            "braille" => Self::Braille,
            "buginese" => Self::Buginese,
            "buhid" => Self::Buhid,
            "canadian-aboriginal" => Self::CanadianAboriginal,
            "carian" => Self::Carian,
            "caucasian-albanian" => Self::CaucasianAlbanian,
            "chakma" => Self::Chakma,
            "cham" => Self::Cham,
            "cherokee" => Self::Cherokee,
            "chinese-hongkong" => Self::ChineseHongkong,
            "chinese-simplified" => Self::ChineseSimplified,
            "chinese-traditional" => Self::ChineseTraditional,
            "chorasmian" => Self::Chorasmian,
            "coptic" => Self::Coptic,
            "cuneiform" => Self::Cuneiform,
            "cypriot" => Self::Cypriot,
            "cypro-minoan" => Self::CyproMinoan,
            "cyrillic" => Self::Cyrillic,
            "cyrillic-ext" => Self::CyrillicExt,
            "deseret" => Self::Deseret,
            "devanagari" => Self::Devanagari,
            "dives-akuru" => Self::DivesAkuru,
            "dogra" => Self::Dogra,
            "duployan" => Self::Duployan,
            "egyptian-hieroglyphs" => Self::EgyptianHieroglyphs,
            "elbasan" => Self::Elbasan,
            "elymaic" => Self::Elymaic,
            "emoji" => Self::Emoji,
            "ethiopic" => Self::Ethiopic,
            "georgian" => Self::Georgian,
            "glagolitic" => Self::Glagolitic,
            "gothic" => Self::Gothic,
            "grantha" => Self::Grantha,
            "greek" => Self::Greek,
            "greek-ext" => Self::GreekExt,
            "gujarati" => Self::Gujarati,
            "gunjala-gondi" => Self::GunjalaGondi,
            "gurmukhi" => Self::Gurmukhi,
            "hanifi-rohingya" => Self::HanifiRohingya,
            "hanunoo" => Self::Hanunoo,
            "hatran" => Self::Hatran,
            "hebrew" => Self::Hebrew,
            "imperial-aramaic" => Self::ImperialAramaic,
            "indic-siyaq-numbers" => Self::IndicSiyaqNumbers,
            "inscriptional-pahlavi" => Self::InscriptionalPahlavi,
            "inscriptional-parthian" => Self::InscriptionalParthian,
            "japanese" => Self::Japanese,
            "javanese" => Self::Javanese,
            "kaithi" => Self::Kaithi,
            "kana-extended" => Self::KanaExtended,
            "kannada" => Self::Kannada,
            "kawi" => Self::Kawi,
            "kayah-li" => Self::KayahLi,
            "kharoshthi" => Self::Kharoshthi,
            "khitan-small-script" => Self::KhitanSmallScript,
            "khmer" => Self::Khmer,
            "khojki" => Self::Khojki,
            "khudawadi" => Self::Khudawadi,
            "kirat-rai" => Self::KiratRai,
            "korean" => Self::Korean,
            "lao" => Self::Lao,
            "latin" => Self::Latin,
            "latin-ext" => Self::LatinExt,
            "lepcha" => Self::Lepcha,
            "limbu" => Self::Limbu,
            "linear-a" => Self::LinearA,
            "linear-b" => Self::LinearB,
            "lisu" => Self::Lisu,
            "lycian" => Self::Lycian,
            "lydian" => Self::Lydian,
            "mahajani" => Self::Mahajani,
            "makasar" => Self::Makasar,
            "malayalam" => Self::Malayalam,
            "mandaic" => Self::Mandaic,
            "manichaean" => Self::Manichaean,
            "marchen" => Self::Marchen,
            "masaram-gondi" => Self::MasaramGondi,
            "math" => Self::Math,
            "mayan-numerals" => Self::MayanNumerals,
            "medefaidrin" => Self::Medefaidrin,
            "meetei-mayek" => Self::MeeteiMayek,
            "mende-kikakui" => Self::MendeKikakui,
            "meroitic" => Self::Meroitic,
            "meroitic-cursive" => Self::MeroiticCursive,
            "meroitic-hieroglyphs" => Self::MeroiticHieroglyphs,
            "miao" => Self::Miao,
            "modi" => Self::Modi,
            "mongolian" => Self::Mongolian,
            "mro" => Self::Mro,
            "multani" => Self::Multani,
            "music" => Self::Music,
            "myanmar" => Self::Myanmar,
            "nabataean" => Self::Nabataean,
            "nag-mundari" => Self::NagMundari,
            "nandinagari" => Self::Nandinagari,
            "new-tai-lue" => Self::NewTaiLue,
            "newa" => Self::Newa,
            "nko" => Self::Nko,
            "nushu" => Self::Nushu,
            "nyiakeng-puachue-hmong" => Self::NyiakengPuachueHmong,
            "ogham" => Self::Ogham,
            "ol-chiki" => Self::OlChiki,
            "old-hungarian" => Self::OldHungarian,
            "old-italic" => Self::OldItalic,
            "old-north-arabian" => Self::OldNorthArabian,
            "old-permic" => Self::OldPermic,
            "old-persian" => Self::OldPersian,
            "old-sogdian" => Self::OldSogdian,
            "old-south-arabian" => Self::OldSouthArabian,
            "old-turkic" => Self::OldTurkic,
            "old-uyghur" => Self::OldUyghur,
            "oriya" => Self::Oriya,
            "osage" => Self::Osage,
            "osmanya" => Self::Osmanya,
            "ottoman-siyaq-numbers" => Self::OttomanSiyaqNumbers,
            "pahawh-hmong" => Self::PahawhHmong,
            "palmyrene" => Self::Palmyrene,
            "pau-cin-hau" => Self::PauCinHau,
            "phags-pa" => Self::PhagsPa,
            "phoenician" => Self::Phoenician,
            "psalter-pahlavi" => Self::PsalterPahlavi,
            "rejang" => Self::Rejang,
            "runic" => Self::Runic,
            "samaritan" => Self::Samaritan,
            "saurashtra" => Self::Saurashtra,
            "sharada" => Self::Sharada,
            "shavian" => Self::Shavian,
            "siddham" => Self::Siddham,
            "signwriting" => Self::Signwriting,
            "sinhala" => Self::Sinhala,
            "sogdian" => Self::Sogdian,
            "sora-sompeng" => Self::SoraSompeng,
            "soyombo" => Self::Soyombo,
            "sundanese" => Self::Sundanese,
            "sunuwar" => Self::Sunuwar,
            "syloti-nagri" => Self::SylotiNagri,
            "symbols" => Self::Symbols,
            "symbols2" => Self::Symbols2,
            "syriac" => Self::Syriac,
            "tagalog" => Self::Tagalog,
            "tagbanwa" => Self::Tagbanwa,
            "tai-le" => Self::TaiLe,
            "tai-tham" => Self::TaiTham,
            "tai-viet" => Self::TaiViet,
            "takri" => Self::Takri,
            "tamil" => Self::Tamil,
            "tamil-supplement" => Self::TamilSupplement,
            "tangsa" => Self::Tangsa,
            "tangut" => Self::Tangut,
            "telugu" => Self::Telugu,
            "thaana" => Self::Thaana,
            "thai" => Self::Thai,
            "tibetan" => Self::Tibetan,
            "tifinagh" => Self::Tifinagh,
            "tirhuta" => Self::Tirhuta,
            "todhri" => Self::Todhri,
            "toto" => Self::Toto,
            "ugaritic" => Self::Ugaritic,
            "vai" => Self::Vai,
            "vietnamese" => Self::Vietnamese,
            "vithkuqi" => Self::Vithkuqi,
            "wancho" => Self::Wancho,
            "warang-citi" => Self::WarangCiti,
            "yezidi" => Self::Yezidi,
            "yi" => Self::Yi,
            "zanabazar-square" => Self::ZanabazarSquare,
            "znamenny" => Self::Znamenny,
            _ => return None,
        })
    }

    /// Parse a variant name, e.g. `"LatinExt"`, returning `None` if it is
    /// not in the vendored catalog.
    #[must_use]
    pub fn from_variant(name: &str) -> Option<Self> {
        Some(match name {
            "Adlam" => Self::Adlam,
            "Ahom" => Self::Ahom,
            "AnatolianHieroglyphs" => Self::AnatolianHieroglyphs,
            "Arabic" => Self::Arabic,
            "Armenian" => Self::Armenian,
            "Avestan" => Self::Avestan,
            "Balinese" => Self::Balinese,
            "Bamum" => Self::Bamum,
            "BassaVah" => Self::BassaVah,
            "Batak" => Self::Batak,
            "Bengali" => Self::Bengali,
            "BeriaErfe" => Self::BeriaErfe,
            "Bhaiksuki" => Self::Bhaiksuki,
            "Brahmi" => Self::Brahmi,
            "Braille" => Self::Braille,
            "Buginese" => Self::Buginese,
            "Buhid" => Self::Buhid,
            "CanadianAboriginal" => Self::CanadianAboriginal,
            "Carian" => Self::Carian,
            "CaucasianAlbanian" => Self::CaucasianAlbanian,
            "Chakma" => Self::Chakma,
            "Cham" => Self::Cham,
            "Cherokee" => Self::Cherokee,
            "ChineseHongkong" => Self::ChineseHongkong,
            "ChineseSimplified" => Self::ChineseSimplified,
            "ChineseTraditional" => Self::ChineseTraditional,
            "Chorasmian" => Self::Chorasmian,
            "Coptic" => Self::Coptic,
            "Cuneiform" => Self::Cuneiform,
            "Cypriot" => Self::Cypriot,
            "CyproMinoan" => Self::CyproMinoan,
            "Cyrillic" => Self::Cyrillic,
            "CyrillicExt" => Self::CyrillicExt,
            "Deseret" => Self::Deseret,
            "Devanagari" => Self::Devanagari,
            "DivesAkuru" => Self::DivesAkuru,
            "Dogra" => Self::Dogra,
            "Duployan" => Self::Duployan,
            "EgyptianHieroglyphs" => Self::EgyptianHieroglyphs,
            "Elbasan" => Self::Elbasan,
            "Elymaic" => Self::Elymaic,
            "Emoji" => Self::Emoji,
            "Ethiopic" => Self::Ethiopic,
            "Georgian" => Self::Georgian,
            "Glagolitic" => Self::Glagolitic,
            "Gothic" => Self::Gothic,
            "Grantha" => Self::Grantha,
            "Greek" => Self::Greek,
            "GreekExt" => Self::GreekExt,
            "Gujarati" => Self::Gujarati,
            "GunjalaGondi" => Self::GunjalaGondi,
            "Gurmukhi" => Self::Gurmukhi,
            "HanifiRohingya" => Self::HanifiRohingya,
            "Hanunoo" => Self::Hanunoo,
            "Hatran" => Self::Hatran,
            "Hebrew" => Self::Hebrew,
            "ImperialAramaic" => Self::ImperialAramaic,
            "IndicSiyaqNumbers" => Self::IndicSiyaqNumbers,
            "InscriptionalPahlavi" => Self::InscriptionalPahlavi,
            "InscriptionalParthian" => Self::InscriptionalParthian,
            "Japanese" => Self::Japanese,
            "Javanese" => Self::Javanese,
            "Kaithi" => Self::Kaithi,
            "KanaExtended" => Self::KanaExtended,
            "Kannada" => Self::Kannada,
            "Kawi" => Self::Kawi,
            "KayahLi" => Self::KayahLi,
            "Kharoshthi" => Self::Kharoshthi,
            "KhitanSmallScript" => Self::KhitanSmallScript,
            "Khmer" => Self::Khmer,
            "Khojki" => Self::Khojki,
            "Khudawadi" => Self::Khudawadi,
            "KiratRai" => Self::KiratRai,
            "Korean" => Self::Korean,
            "Lao" => Self::Lao,
            "Latin" => Self::Latin,
            "LatinExt" => Self::LatinExt,
            "Lepcha" => Self::Lepcha,
            "Limbu" => Self::Limbu,
            "LinearA" => Self::LinearA,
            "LinearB" => Self::LinearB,
            "Lisu" => Self::Lisu,
            "Lycian" => Self::Lycian,
            "Lydian" => Self::Lydian,
            "Mahajani" => Self::Mahajani,
            "Makasar" => Self::Makasar,
            "Malayalam" => Self::Malayalam,
            "Mandaic" => Self::Mandaic,
            "Manichaean" => Self::Manichaean,
            "Marchen" => Self::Marchen,
            "MasaramGondi" => Self::MasaramGondi,
            "Math" => Self::Math,
            "MayanNumerals" => Self::MayanNumerals,
            "Medefaidrin" => Self::Medefaidrin,
            "MeeteiMayek" => Self::MeeteiMayek,
            "MendeKikakui" => Self::MendeKikakui,
            "Meroitic" => Self::Meroitic,
            "MeroiticCursive" => Self::MeroiticCursive,
            "MeroiticHieroglyphs" => Self::MeroiticHieroglyphs,
            "Miao" => Self::Miao,
            "Modi" => Self::Modi,
            "Mongolian" => Self::Mongolian,
            "Mro" => Self::Mro,
            "Multani" => Self::Multani,
            "Music" => Self::Music,
            "Myanmar" => Self::Myanmar,
            "Nabataean" => Self::Nabataean,
            "NagMundari" => Self::NagMundari,
            "Nandinagari" => Self::Nandinagari,
            "NewTaiLue" => Self::NewTaiLue,
            "Newa" => Self::Newa,
            "Nko" => Self::Nko,
            "Nushu" => Self::Nushu,
            "NyiakengPuachueHmong" => Self::NyiakengPuachueHmong,
            "Ogham" => Self::Ogham,
            "OlChiki" => Self::OlChiki,
            "OldHungarian" => Self::OldHungarian,
            "OldItalic" => Self::OldItalic,
            "OldNorthArabian" => Self::OldNorthArabian,
            "OldPermic" => Self::OldPermic,
            "OldPersian" => Self::OldPersian,
            "OldSogdian" => Self::OldSogdian,
            "OldSouthArabian" => Self::OldSouthArabian,
            "OldTurkic" => Self::OldTurkic,
            "OldUyghur" => Self::OldUyghur,
            "Oriya" => Self::Oriya,
            "Osage" => Self::Osage,
            "Osmanya" => Self::Osmanya,
            "OttomanSiyaqNumbers" => Self::OttomanSiyaqNumbers,
            "PahawhHmong" => Self::PahawhHmong,
            "Palmyrene" => Self::Palmyrene,
            "PauCinHau" => Self::PauCinHau,
            "PhagsPa" => Self::PhagsPa,
            "Phoenician" => Self::Phoenician,
            "PsalterPahlavi" => Self::PsalterPahlavi,
            "Rejang" => Self::Rejang,
            "Runic" => Self::Runic,
            "Samaritan" => Self::Samaritan,
            "Saurashtra" => Self::Saurashtra,
            "Sharada" => Self::Sharada,
            "Shavian" => Self::Shavian,
            "Siddham" => Self::Siddham,
            "Signwriting" => Self::Signwriting,
            "Sinhala" => Self::Sinhala,
            "Sogdian" => Self::Sogdian,
            "SoraSompeng" => Self::SoraSompeng,
            "Soyombo" => Self::Soyombo,
            "Sundanese" => Self::Sundanese,
            "Sunuwar" => Self::Sunuwar,
            "SylotiNagri" => Self::SylotiNagri,
            "Symbols" => Self::Symbols,
            "Symbols2" => Self::Symbols2,
            "Syriac" => Self::Syriac,
            "Tagalog" => Self::Tagalog,
            "Tagbanwa" => Self::Tagbanwa,
            "TaiLe" => Self::TaiLe,
            "TaiTham" => Self::TaiTham,
            "TaiViet" => Self::TaiViet,
            "Takri" => Self::Takri,
            "Tamil" => Self::Tamil,
            "TamilSupplement" => Self::TamilSupplement,
            "Tangsa" => Self::Tangsa,
            "Tangut" => Self::Tangut,
            "Telugu" => Self::Telugu,
            "Thaana" => Self::Thaana,
            "Thai" => Self::Thai,
            "Tibetan" => Self::Tibetan,
            "Tifinagh" => Self::Tifinagh,
            "Tirhuta" => Self::Tirhuta,
            "Todhri" => Self::Todhri,
            "Toto" => Self::Toto,
            "Ugaritic" => Self::Ugaritic,
            "Vai" => Self::Vai,
            "Vietnamese" => Self::Vietnamese,
            "Vithkuqi" => Self::Vithkuqi,
            "Wancho" => Self::Wancho,
            "WarangCiti" => Self::WarangCiti,
            "Yezidi" => Self::Yezidi,
            "Yi" => Self::Yi,
            "ZanabazarSquare" => Self::ZanabazarSquare,
            "Znamenny" => Self::Znamenny,
            _ => return None,
        })
    }
}
