//! 乐器种类 — 纯领域枚举，无 MIDI 程序号 / CC 映射。
//!
//! 128 种乐器覆盖 General MIDI 标准音色集，按族分组。

/// 乐器族（16 族，对应 General MIDI 分类）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrumentFamily {
    Piano,
    ChromaticPercussion,
    Organ,
    Guitar,
    Bass,
    Strings,
    Ensemble,
    Brass,
    Reed,
    Pipe,
    SynthLead,
    SynthPad,
    SynthEffects,
    Ethnic,
    Percussive,
    SoundEffects,
}

impl InstrumentFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Piano => "Piano",
            Self::ChromaticPercussion => "Chromatic Percussion",
            Self::Organ => "Organ",
            Self::Guitar => "Guitar",
            Self::Bass => "Bass",
            Self::Strings => "Strings",
            Self::Ensemble => "Ensemble",
            Self::Brass => "Brass",
            Self::Reed => "Reed",
            Self::Pipe => "Pipe",
            Self::SynthLead => "Synth Lead",
            Self::SynthPad => "Synth Pad",
            Self::SynthEffects => "Synth Effects",
            Self::Ethnic => "Ethnic",
            Self::Percussive => "Percussive",
            Self::SoundEffects => "Sound Effects",
        }
    }
}

/// 128 种乐器音色（General MIDI 编号 0-127）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrumentKind {
    // ── Piano (0-7) ──
    AcousticPiano,
    BrightAcousticPiano,
    ElectricGrandPiano,
    HonkyTonkPiano,
    ElectricPiano1,
    ElectricPiano2,
    Harpsichord,
    Clavinet,

    // ── Chromatic Percussion (8-15) ──
    Celesta,
    Glockenspiel,
    MusicBox,
    Vibraphone,
    Marimba,
    Xylophone,
    TubularBells,
    Dulcimer,

    // ── Organ (16-23) ──
    HammondOrgan,
    PercussiveOrgan,
    RockOrgan,
    ChurchOrgan,
    ReedOrgan,
    Accordion,
    Harmonica,
    Bandoneon,

    // ── Guitar (24-31) ──
    AcousticGuitarNylon,
    AcousticGuitarSteel,
    ElectricGuitarJazz,
    ElectricGuitarClean,
    ElectricGuitarMuted,
    OverdrivenGuitar,
    DistortionGuitar,
    GuitarHarmonics,

    // ── Bass (32-39) ──
    AcousticBass,
    ElectricBassFinger,
    ElectricBassPick,
    FretlessBass,
    SlapBass1,
    SlapBass2,
    SynthBass1,
    SynthBass2,

    // ── Strings (40-47) ──
    Violin,
    Viola,
    Cello,
    Contrabass,
    TremoloStrings,
    PizzicatoStrings,
    OrchestralHarp,
    Timpani,

    // ── Ensemble (48-55) ──
    StringEnsemble1,
    StringEnsemble2,
    SynthStrings1,
    SynthStrings2,
    ChoirAahs,
    VoiceOohs,
    SynthChoir,
    OrchestraHit,

    // ── Brass (56-63) ──
    Trumpet,
    Trombone,
    Tuba,
    MutedTrumpet,
    FrenchHorn,
    BrassSection,
    SynthBrass1,
    SynthBrass2,

    // ── Reed (64-71) ──
    SopranoSax,
    AltoSax,
    TenorSax,
    BaritoneSax,
    Oboe,
    EnglishHorn,
    Bassoon,
    Clarinet,

    // ── Pipe (72-79) ──
    Piccolo,
    Flute,
    Recorder,
    PanFlute,
    BlownBottle,
    Shakuhachi,
    Whistle,
    Ocarina,

    // ── Synth Lead (80-87) ──
    Lead1Square,
    Lead2Sawtooth,
    Lead3Calliope,
    Lead4Chiff,
    Lead5Charang,
    Lead6Voice,
    Lead7Fifths,
    Lead8BassLead,

    // ── Synth Pad (88-95) ──
    Pad1NewAge,
    Pad2Warm,
    Pad3Polysynth,
    Pad4Choir,
    Pad5Bowed,
    Pad6Metallic,
    Pad7Halo,
    Pad8Sweep,

    // ── Synth Effects (96-103) ──
    Fx1Rain,
    Fx2Soundtrack,
    Fx3Crystal,
    Fx4Atmosphere,
    Fx5Brightness,
    Fx6Goblins,
    Fx7Echoes,
    Fx8SciFi,

    // ── Ethnic (104-111) ──
    Sitar,
    Banjo,
    Shamisen,
    Koto,
    Kalimba,
    BagPipe,
    Fiddle,
    Shanai,

    // ── Percussive (112-119) ──
    TinkleBell,
    Agogo,
    SteelDrums,
    Woodblock,
    TaikoDrum,
    MelodicTom,
    SynthDrum,
    ReverseCymbal,

    // ── Sound Effects (120-127) ──
    GuitarFretNoise,
    BreathNoise,
    Seashore,
    BirdTweet,
    TelephoneRing,
    Helicopter,
    Applause,
    Gunshot,
}

impl InstrumentKind {
    /// 所属乐器族。
    pub fn family(&self) -> InstrumentFamily {
        let n = self.index();
        match n / 8 {
            0 => InstrumentFamily::Piano,
            1 => InstrumentFamily::ChromaticPercussion,
            2 => InstrumentFamily::Organ,
            3 => InstrumentFamily::Guitar,
            4 => InstrumentFamily::Bass,
            5 => InstrumentFamily::Strings,
            6 => InstrumentFamily::Ensemble,
            7 => InstrumentFamily::Brass,
            8 => InstrumentFamily::Reed,
            9 => InstrumentFamily::Pipe,
            10 => InstrumentFamily::SynthLead,
            11 => InstrumentFamily::SynthPad,
            12 => InstrumentFamily::SynthEffects,
            13 => InstrumentFamily::Ethnic,
            14 => InstrumentFamily::Percussive,
            _ => InstrumentFamily::SoundEffects,
        }
    }

    /// 序号（0-127），便于外部映射（如 MIDI 程序号）。
    pub fn index(&self) -> u8 {
        *self as u8
    }

    /// 人类可读名称。
    pub fn display_name(&self) -> &'static str {
        DISPLAY_NAMES[self.index() as usize]
    }

    /// 短标识符（用于词法解析）。
    pub fn as_str(&self) -> &'static str {
        SHORT_NAMES[self.index() as usize]
    }

    pub fn from_str(s: &str) -> Option<Self> {
        SHORT_NAMES.iter().position(|&name| name == s).map(|i| Self::from_index(i as u8))
    }

    pub fn from_index(idx: u8) -> Self {
        // SAFETY: InstrumentKind is `#[repr(u8)]`-compatible by construction
        // (128 variants, 0-127). We use transmute-like manual mapping.
        ALL_INSTRUMENTS[idx as usize % 128]
    }
}

// 通过偏移量获取枚举值的辅助表
const ALL_INSTRUMENTS: [InstrumentKind; 128] = [
    InstrumentKind::AcousticPiano,
    InstrumentKind::BrightAcousticPiano,
    InstrumentKind::ElectricGrandPiano,
    InstrumentKind::HonkyTonkPiano,
    InstrumentKind::ElectricPiano1,
    InstrumentKind::ElectricPiano2,
    InstrumentKind::Harpsichord,
    InstrumentKind::Clavinet,
    InstrumentKind::Celesta,
    InstrumentKind::Glockenspiel,
    InstrumentKind::MusicBox,
    InstrumentKind::Vibraphone,
    InstrumentKind::Marimba,
    InstrumentKind::Xylophone,
    InstrumentKind::TubularBells,
    InstrumentKind::Dulcimer,
    InstrumentKind::HammondOrgan,
    InstrumentKind::PercussiveOrgan,
    InstrumentKind::RockOrgan,
    InstrumentKind::ChurchOrgan,
    InstrumentKind::ReedOrgan,
    InstrumentKind::Accordion,
    InstrumentKind::Harmonica,
    InstrumentKind::Bandoneon,
    InstrumentKind::AcousticGuitarNylon,
    InstrumentKind::AcousticGuitarSteel,
    InstrumentKind::ElectricGuitarJazz,
    InstrumentKind::ElectricGuitarClean,
    InstrumentKind::ElectricGuitarMuted,
    InstrumentKind::OverdrivenGuitar,
    InstrumentKind::DistortionGuitar,
    InstrumentKind::GuitarHarmonics,
    InstrumentKind::AcousticBass,
    InstrumentKind::ElectricBassFinger,
    InstrumentKind::ElectricBassPick,
    InstrumentKind::FretlessBass,
    InstrumentKind::SlapBass1,
    InstrumentKind::SlapBass2,
    InstrumentKind::SynthBass1,
    InstrumentKind::SynthBass2,
    InstrumentKind::Violin,
    InstrumentKind::Viola,
    InstrumentKind::Cello,
    InstrumentKind::Contrabass,
    InstrumentKind::TremoloStrings,
    InstrumentKind::PizzicatoStrings,
    InstrumentKind::OrchestralHarp,
    InstrumentKind::Timpani,
    InstrumentKind::StringEnsemble1,
    InstrumentKind::StringEnsemble2,
    InstrumentKind::SynthStrings1,
    InstrumentKind::SynthStrings2,
    InstrumentKind::ChoirAahs,
    InstrumentKind::VoiceOohs,
    InstrumentKind::SynthChoir,
    InstrumentKind::OrchestraHit,
    InstrumentKind::Trumpet,
    InstrumentKind::Trombone,
    InstrumentKind::Tuba,
    InstrumentKind::MutedTrumpet,
    InstrumentKind::FrenchHorn,
    InstrumentKind::BrassSection,
    InstrumentKind::SynthBrass1,
    InstrumentKind::SynthBrass2,
    InstrumentKind::SopranoSax,
    InstrumentKind::AltoSax,
    InstrumentKind::TenorSax,
    InstrumentKind::BaritoneSax,
    InstrumentKind::Oboe,
    InstrumentKind::EnglishHorn,
    InstrumentKind::Bassoon,
    InstrumentKind::Clarinet,
    InstrumentKind::Piccolo,
    InstrumentKind::Flute,
    InstrumentKind::Recorder,
    InstrumentKind::PanFlute,
    InstrumentKind::BlownBottle,
    InstrumentKind::Shakuhachi,
    InstrumentKind::Whistle,
    InstrumentKind::Ocarina,
    InstrumentKind::Lead1Square,
    InstrumentKind::Lead2Sawtooth,
    InstrumentKind::Lead3Calliope,
    InstrumentKind::Lead4Chiff,
    InstrumentKind::Lead5Charang,
    InstrumentKind::Lead6Voice,
    InstrumentKind::Lead7Fifths,
    InstrumentKind::Lead8BassLead,
    InstrumentKind::Pad1NewAge,
    InstrumentKind::Pad2Warm,
    InstrumentKind::Pad3Polysynth,
    InstrumentKind::Pad4Choir,
    InstrumentKind::Pad5Bowed,
    InstrumentKind::Pad6Metallic,
    InstrumentKind::Pad7Halo,
    InstrumentKind::Pad8Sweep,
    InstrumentKind::Fx1Rain,
    InstrumentKind::Fx2Soundtrack,
    InstrumentKind::Fx3Crystal,
    InstrumentKind::Fx4Atmosphere,
    InstrumentKind::Fx5Brightness,
    InstrumentKind::Fx6Goblins,
    InstrumentKind::Fx7Echoes,
    InstrumentKind::Fx8SciFi,
    InstrumentKind::Sitar,
    InstrumentKind::Banjo,
    InstrumentKind::Shamisen,
    InstrumentKind::Koto,
    InstrumentKind::Kalimba,
    InstrumentKind::BagPipe,
    InstrumentKind::Fiddle,
    InstrumentKind::Shanai,
    InstrumentKind::TinkleBell,
    InstrumentKind::Agogo,
    InstrumentKind::SteelDrums,
    InstrumentKind::Woodblock,
    InstrumentKind::TaikoDrum,
    InstrumentKind::MelodicTom,
    InstrumentKind::SynthDrum,
    InstrumentKind::ReverseCymbal,
    InstrumentKind::GuitarFretNoise,
    InstrumentKind::BreathNoise,
    InstrumentKind::Seashore,
    InstrumentKind::BirdTweet,
    InstrumentKind::TelephoneRing,
    InstrumentKind::Helicopter,
    InstrumentKind::Applause,
    InstrumentKind::Gunshot,
];

const SHORT_NAMES: &[&str] = &[
    "piano", "bright_piano", "electric_grand", "honky_tonk", "electric_piano", "electric_piano2",
    "harpsichord", "clavinet", "celesta", "glockenspiel", "music_box", "vibraphone", "marimba",
    "xylophone", "tubular_bells", "dulcimer", "hammond", "percussive_organ", "rock_organ",
    "church_organ", "reed_organ", "accordion", "harmonica", "bandoneon", "guitar_nylon", "guitar",
    "guitar_jazz", "guitar_clean", "guitar_muted", "overdrive", "distortion", "guitar_harmonics",
    "bass", "bass_finger", "bass_pick", "fretless_bass", "slap_bass", "slap_bass2", "synth_bass",
    "synth_bass2", "violin", "viola", "cello", "contrabass", "tremolo_strings", "pizzicato",
    "harp", "timpani", "strings", "strings2", "synth_strings", "synth_strings2", "choir", "voice",
    "synth_choir", "orchestra_hit", "trumpet", "trombone", "tuba", "muted_trumpet", "french_horn",
    "brass", "synth_brass", "synth_brass2", "soprano_sax", "alto_sax", "tenor_sax", "baritone_sax",
    "oboe", "english_horn", "bassoon", "clarinet", "piccolo", "flute", "recorder", "pan_flute",
    "blown_bottle", "shakuhachi", "whistle", "ocarina", "lead_square", "lead_sawtooth",
    "lead_calliope", "lead_chiff", "lead_charang", "lead_voice", "lead_fifths", "lead_bass",
    "pad_new_age", "pad_warm", "pad_polysynth", "pad_choir", "pad_bowed", "pad_metallic",
    "pad_halo", "pad_sweep", "fx_rain", "fx_soundtrack", "fx_crystal", "fx_atmosphere",
    "fx_brightness", "fx_goblins", "fx_echoes", "fx_scifi", "sitar", "banjo", "shamisen", "koto",
    "kalimba", "bagpipe", "fiddle", "shanai", "tinkle_bell", "agogo", "steel_drums", "woodblock",
    "taiko", "melodic_tom", "synth_drum", "reverse_cymbal", "guitar_fret_noise", "breath_noise",
    "seashore", "bird_tweet", "telephone", "helicopter", "applause", "gunshot",
];

const DISPLAY_NAMES: &[&str] = &[
    "Acoustic Grand Piano", "Bright Acoustic Piano", "Electric Grand Piano", "Honky-tonk Piano",
    "Electric Piano 1", "Electric Piano 2", "Harpsichord", "Clavinet", "Celesta", "Glockenspiel",
    "Music Box", "Vibraphone", "Marimba", "Xylophone", "Tubular Bells", "Dulcimer",
    "Hammond Organ", "Percussive Organ", "Rock Organ", "Church Organ", "Reed Organ", "Accordion",
    "Harmonica", "Bandoneon", "Acoustic Guitar (nylon)", "Acoustic Guitar (steel)",
    "Electric Guitar (jazz)", "Electric Guitar (clean)", "Electric Guitar (muted)",
    "Overdriven Guitar", "Distortion Guitar", "Guitar Harmonics", "Acoustic Bass",
    "Electric Bass (finger)", "Electric Bass (pick)", "Fretless Bass", "Slap Bass 1",
    "Slap Bass 2", "Synth Bass 1", "Synth Bass 2", "Violin", "Viola", "Cello", "Contrabass",
    "Tremolo Strings", "Pizzicato Strings", "Orchestral Harp", "Timpani", "String Ensemble 1",
    "String Ensemble 2", "Synth Strings 1", "Synth Strings 2", "Choir Aahs", "Voice Oohs",
    "Synth Choir", "Orchestra Hit", "Trumpet", "Trombone", "Tuba", "Muted Trumpet", "French Horn",
    "Brass Section", "Synth Brass 1", "Synth Brass 2", "Soprano Sax", "Alto Sax", "Tenor Sax",
    "Baritone Sax", "Oboe", "English Horn", "Bassoon", "Clarinet", "Piccolo", "Flute", "Recorder",
    "Pan Flute", "Blown Bottle", "Shakuhachi", "Whistle", "Ocarina", "Lead 1 (Square)",
    "Lead 2 (Sawtooth)", "Lead 3 (Calliope)", "Lead 4 (Chiff)", "Lead 5 (Charang)", "Lead 6 (Voice)",
    "Lead 7 (Fifths)", "Lead 8 (Bass + Lead)", "Pad 1 (New Age)", "Pad 2 (Warm)",
    "Pad 3 (Polysynth)", "Pad 4 (Choir)", "Pad 5 (Bowed)", "Pad 6 (Metallic)", "Pad 7 (Halo)",
    "Pad 8 (Sweep)", "FX 1 (Rain)", "FX 2 (Soundtrack)", "FX 3 (Crystal)", "FX 4 (Atmosphere)",
    "FX 5 (Brightness)", "FX 6 (Goblins)", "FX 7 (Echoes)", "FX 8 (Sci-Fi)", "Sitar", "Banjo",
    "Shamisen", "Koto", "Kalimba", "Bag Pipe", "Fiddle", "Shanai", "Tinkle Bell", "Agogo",
    "Steel Drums", "Woodblock", "Taiko Drum", "Melodic Tom", "Synth Drum", "Reverse Cymbal",
    "Guitar Fret Noise", "Breath Noise", "Seashore", "Bird Tweet", "Telephone Ring", "Helicopter",
    "Applause", "Gunshot",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_family() {
        assert_eq!(InstrumentKind::AcousticPiano.family(), InstrumentFamily::Piano);
        assert_eq!(InstrumentKind::Violin.family(), InstrumentFamily::Strings);
        assert_eq!(InstrumentKind::Trumpet.family(), InstrumentFamily::Brass);
        assert_eq!(InstrumentKind::Gunshot.family(), InstrumentFamily::SoundEffects);
    }

    #[test]
    fn test_index_roundtrip() {
        for i in 0u8..128 {
            let inst = InstrumentKind::from_index(i);
            assert_eq!(inst.index(), i);
        }
    }

    #[test]
    fn test_from_str() {
        assert_eq!(InstrumentKind::from_str("piano"), Some(InstrumentKind::AcousticPiano));
        assert_eq!(InstrumentKind::from_str("violin"), Some(InstrumentKind::Violin));
        assert_eq!(InstrumentKind::from_str("nonexistent"), None);
    }

    #[test]
    fn test_display_name() {
        assert_eq!(InstrumentKind::AcousticPiano.display_name(), "Acoustic Grand Piano");
        assert_eq!(InstrumentKind::Trumpet.display_name(), "Trumpet");
    }

    #[test]
    fn test_family_coverage() {
        assert_eq!(InstrumentKind::SynthDrum.family(), InstrumentFamily::Percussive);
        assert_eq!(InstrumentKind::MelodicTom.family(), InstrumentFamily::Percussive);
        assert_eq!(InstrumentKind::Gunshot.family(), InstrumentFamily::SoundEffects);
        assert_eq!(InstrumentKind::Celesta.family(), InstrumentFamily::ChromaticPercussion);
        assert_eq!(InstrumentKind::Harpsichord.family(), InstrumentFamily::Piano);
        assert_eq!(InstrumentKind::Contrabass.family(), InstrumentFamily::Strings);
        assert_eq!(InstrumentKind::FrenchHorn.family(), InstrumentFamily::Brass);
    }

    #[test]
    fn test_index_boundary() {
        let first = InstrumentKind::from_index(0);
        assert_eq!(first, InstrumentKind::AcousticPiano);
        let last = InstrumentKind::from_index(127);
        assert_eq!(last, InstrumentKind::Gunshot);
    }

    #[test]
    fn test_from_str_known() {
        assert!(InstrumentKind::from_str("piano").is_some());
        assert!(InstrumentKind::from_str("violin").is_some());
        assert!(InstrumentKind::from_str("cello").is_some());
        assert!(InstrumentKind::from_str("trumpet").is_some());
        assert!(InstrumentKind::from_str("bass").is_some());
    }

    #[test]
    fn test_from_str_unknown() {
        assert!(InstrumentKind::from_str("").is_none());
        assert!(InstrumentKind::from_str("xyz").is_none());
        assert!(InstrumentKind::from_str("PIANO").is_none());
    }

    #[test]
    fn test_as_str_consistency() {
        let kinds = [
            InstrumentKind::AcousticGuitarNylon,
            InstrumentKind::Violin,
            InstrumentKind::Trumpet,
            InstrumentKind::ElectricGuitarClean,
            InstrumentKind::Gunshot,
        ];
        for k in &kinds {
            let s = k.as_str();
            assert!(InstrumentKind::from_str(s).is_some(), "from_str({}) failed", s);
        }
    }
}
