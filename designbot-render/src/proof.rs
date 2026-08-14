//! Built-in default **print proof** generator.
//!
//! `designbot proof <font> -o proof.pdf` introspects any font (axes, named
//! instances, charset, metrics, features) and emits a multi-page, color-managed
//! PDF proof — no per-repo script required. Designed as a superset by eye of
//! Google Fonts' diffenator2 `proof` view.
//!
//! US Letter landscape (792 × 612 pt), laid out on a 6-column Swiss modular
//! grid (overlay it with `--grid` when tuning layout; off by default).
//! Technical data is set in IBM Plex Mono (bundled, OFL).
//! See virtua-grotesk/documentation/proofs/PROOF_SPEC.md for the page plan.

use crate::Renderer;
use designbot_core::{Canvas, Color, DesignBotError, Grid, TextAlign};
use std::path::Path;

// --- bundled monospace for technical chrome (OFL, see assets/) -------------
const MONO_TTF: &[u8] = include_bytes!("../assets/IBMPlexMono-Regular.ttf");
const MONO: &str = "IBM Plex Mono";

// --- page geometry (US Letter landscape, points = px) ----------------------
const W: f64 = 792.0;
const H: f64 = 612.0;
const M: f64 = 54.0; // margin
const COLS: usize = 6; // Swiss modular columns
const GUTTER: f64 = 16.0;
const GRID_ROWS: u32 = 6;
/// One size for ALL monospace chrome (headlines, labels, captions, running
/// head, technical data) — the proof's information layer is uniform.
const MONO_SIZE: f64 = 8.5;

// --- Arabic proof data ------------------------------------------------------

/// Zero-width joiner: forces a letter into a positional form without adding
/// the ink that tatweel would.
const ZWJ: &str = "\u{200D}";

/// The letters in hija'i order with a latin label. The flag marks the
/// dual-joining ones; the right-joining letters (alef, dal, reh, waw and
/// relatives) have no initial or medial form, and the table leaves those
/// cells empty rather than showing a fake.
const AR_LETTERS: &[(&str, &str, bool)] = &[
    ("\u{0627}", "alef", false),
    ("\u{0628}", "beh", true),
    ("\u{062A}", "teh", true),
    ("\u{062B}", "theh", true),
    ("\u{062C}", "jeem", true),
    ("\u{062D}", "hah", true),
    ("\u{062E}", "khah", true),
    ("\u{062F}", "dal", false),
    ("\u{0630}", "thal", false),
    ("\u{0631}", "reh", false),
    ("\u{0632}", "zain", false),
    ("\u{0633}", "seen", true),
    ("\u{0634}", "sheen", true),
    ("\u{0635}", "sad", true),
    ("\u{0636}", "dad", true),
    ("\u{0637}", "tah", true),
    ("\u{0638}", "zah", true),
    ("\u{0639}", "ain", true),
    ("\u{063A}", "ghain", true),
    ("\u{0641}", "feh", true),
    ("\u{0642}", "qaf", true),
    ("\u{0643}", "kaf", true),
    ("\u{0644}", "lam", true),
    ("\u{0645}", "meem", true),
    ("\u{0646}", "noon", true),
    ("\u{0647}", "heh", true),
    ("\u{0648}", "waw", false),
    ("\u{064A}", "yeh", true),
];

/// The vowel marks, in the order they are usually taught.
const AR_HARAKAT: &[(&str, &str)] = &[
    ("\u{064E}", "fatha"),
    ("\u{064F}", "damma"),
    ("\u{0650}", "kasra"),
    ("\u{0652}", "sukun"),
    ("\u{0651}", "shadda"),
    ("\u{064B}", "fathatan"),
    ("\u{064C}", "dammatan"),
    ("\u{064D}", "kasratan"),
];

/// Bases spanning the skeletons: bare stroke, tooth, bowl, loop, wide sweep,
/// ascender, descender — so one page shows every anchor height in the font.
const AR_MARK_BASES: &[&str] = &[
    "\u{0627}", "\u{0628}", "\u{062C}", "\u{062F}", "\u{0631}", "\u{0633}",
    "\u{0635}", "\u{0637}", "\u{0639}", "\u{0641}", "\u{0643}", "\u{0644}",
    "\u{0645}", "\u{0646}", "\u{0647}", "\u{0648}", "\u{064A}",
];

/// Words where two dotted letters meet. Dot clusters collide here first,
/// because the dots sit nearest the joining seam.
const AR_DOT_RUNS: &[(&str, &str)] = &[
    ("\u{0628}\u{064A}\u{062A}", "bayt"),
    ("\u{0628}\u{0646}\u{062A}", "bint"),
    ("\u{062A}\u{0628}\u{064A}\u{0646}", "tabyin"),
    ("\u{0646}\u{062A}\u{064A}\u{062C}\u{0629}", "natija"),
    ("\u{062B}\u{0642}\u{0627}\u{0641}\u{0629}", "thaqafa"),
    ("\u{062A}\u{0641}\u{062A}\u{064A}\u{0634}", "taftish"),
    ("\u{064A}\u{0628}\u{062F}\u{0623}", "yabda"),
    ("\u{0634}\u{062E}\u{0635}", "shakhs"),
];

/// The abjad sequence: every letter once, the Arabic equivalent of a pangram.
const AR_TEXT_PLAIN: &str = "\u{0623}\u{0628}\u{062C}\u{062F} \u{0647}\u{0648}\u{0632} \u{062D}\u{0637}\u{064A} \u{0643}\u{0644}\u{0645}\u{0646} \u{0633}\u{0639}\u{0641}\u{0635} \u{0642}\u{0631}\u{0634}\u{062A} \u{062B}\u{062E}\u{0630} \u{0636}\u{0638}\u{063A}";

/// Fully vocalised, so mark stacking is exercised in running context.
const AR_TEXT_VOCAL: &str = "\u{0627}\u{0644}\u{0652}\u{062D}\u{064E}\u{0645}\u{0652}\u{062F}\u{064F} \u{0644}\u{0650}\u{0644}\u{0651}\u{064E}\u{0647}\u{0650} \u{0631}\u{064E}\u{0628}\u{0651}\u{0650} \u{0627}\u{0644}\u{0652}\u{0639}\u{064E}\u{0627}\u{0644}\u{064E}\u{0645}\u{0650}\u{064A}\u{0646}\u{064E}";

/// The four lam-alef ligatures, which every Arabic font must form.
const AR_LAM_ALEF: &[(&str, &str)] = &[
    ("\u{0644}\u{0627}", "lam-alef"),
    ("\u{0644}\u{0622}", "lam-alef madda"),
    ("\u{0644}\u{0623}", "lam-alef hamza above"),
    ("\u{0644}\u{0625}", "lam-alef hamza below"),
];

/// Al-Fatiha in the Uthmani orthography, byte-exact from tanzil via
/// api.alquran.cloud. The canonical Arabic specimen text and the densest
/// ordinary test of mark stacking: alef wasla, superscript alef, shadda
/// over fatha, and tanwin all appear in seven short lines.
const AR_FATIHA: &[&str] = &[
    "بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ",
    "ٱلْحَمْدُ لِلَّهِ رَبِّ ٱلْعَٰلَمِينَ",
    "ٱلرَّحْمَٰنِ ٱلرَّحِيمِ",
    "مَٰلِكِ يَوْمِ ٱلدِّينِ",
    "إِيَّاكَ نَعْبُدُ وَإِيَّاكَ نَسْتَعِينُ",
    "ٱهْدِنَا ٱلصِّرَٰطَ ٱلْمُسْتَقِيمَ",
    "صِرَٰطَ ٱلَّذِينَ أَنْعَمْتَ عَلَيْهِمْ غَيْرِ ٱلْمَغْضُوبِ عَلَيْهِمْ وَلَا ٱلضَّآلِّينَ",
];

/// Three short surahs in the simple orthography, which drops the Quranic
/// annotation marks and so stays inside an ordinary Arabic character set.
const AR_SURAHS: &[(&str, &str)] = &[
    ("Al-Ikhlas 112", "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ قُلْ هُوَ اللَّهُ أَحَدٌ اللَّهُ الصَّمَدُ لَمْ يَلِدْ وَلَمْ يُولَدْ وَلَمْ يَكُنْ لَهُ كُفُوًا أَحَدٌ"),
    ("Al-Asr 103", "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ وَالْعَصْرِ إِنَّ الْإِنْسَانَ لَفِي خُسْرٍ إِلَّا الَّذِينَ آمَنُوا وَعَمِلُوا الصَّالِحَاتِ وَتَوَاصَوْا بِالْحَقِّ وَتَوَاصَوْا بِالصَّبْرِ"),
    ("Ash-Sharh 94", "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ أَلَمْ نَشْرَحْ لَكَ صَدْرَكَ وَوَضَعْنَا عَنْكَ وِزْرَكَ الَّذِي أَنْقَضَ ظَهْرَكَ وَرَفَعْنَا لَكَ ذِكْرَكَ فَإِنَّ مَعَ الْعُسْرِ يُسْرًا إِنَّ مَعَ الْعُسْرِ يُسْرًا فَإِذَا فَرَغْتَ فَانْصَبْ وَإِلَىٰ رَبِّكَ فَارْغَبْ"),
];

/// Surah Yusuf, ayat 1-46, simple orthography, byte-exact from tanzil via
/// api.alquran.cloud. The waqf (pause) marks are dropped: they are later
/// recitation annotations rather than part of the words, and no ordinary
/// Arabic character set encodes them. Long, fully vocalised running text is
/// the test that catches what short samples miss — line after line of mark
/// stacking, and colour that only shows at reading size.
const AR_YUSUF: &str = "بِسْمِ اللَّهِ الرَّحْمَٰنِ الرَّحِيمِ الر تِلْكَ آيَاتُ الْكِتَابِ الْمُبِينِ إِنَّا أَنْزَلْنَاهُ قُرْآنًا عَرَبِيًّا لَعَلَّكُمْ تَعْقِلُونَ نَحْنُ نَقُصُّ عَلَيْكَ أَحْسَنَ الْقَصَصِ بِمَا أَوْحَيْنَا إِلَيْكَ هَٰذَا الْقُرْآنَ وَإِنْ كُنْتَ مِنْ قَبْلِهِ لَمِنَ الْغَافِلِينَ إِذْ قَالَ يُوسُفُ لِأَبِيهِ يَا أَبَتِ إِنِّي رَأَيْتُ أَحَدَ عَشَرَ كَوْكَبًا وَالشَّمْسَ وَالْقَمَرَ رَأَيْتُهُمْ لِي سَاجِدِينَ قَالَ يَا بُنَيَّ لَا تَقْصُصْ رُؤْيَاكَ عَلَىٰ إِخْوَتِكَ فَيَكِيدُوا لَكَ كَيْدًا إِنَّ الشَّيْطَانَ لِلْإِنْسَانِ عَدُوٌّ مُبِينٌ وَكَذَٰلِكَ يَجْتَبِيكَ رَبُّكَ وَيُعَلِّمُكَ مِنْ تَأْوِيلِ الْأَحَادِيثِ وَيُتِمُّ نِعْمَتَهُ عَلَيْكَ وَعَلَىٰ آلِ يَعْقُوبَ كَمَا أَتَمَّهَا عَلَىٰ أَبَوَيْكَ مِنْ قَبْلُ إِبْرَاهِيمَ وَإِسْحَاقَ إِنَّ رَبَّكَ عَلِيمٌ حَكِيمٌ لَقَدْ كَانَ فِي يُوسُفَ وَإِخْوَتِهِ آيَاتٌ لِلسَّائِلِينَ إِذْ قَالُوا لَيُوسُفُ وَأَخُوهُ أَحَبُّ إِلَىٰ أَبِينَا مِنَّا وَنَحْنُ عُصْبَةٌ إِنَّ أَبَانَا لَفِي ضَلَالٍ مُبِينٍ اقْتُلُوا يُوسُفَ أَوِ اطْرَحُوهُ أَرْضًا يَخْلُ لَكُمْ وَجْهُ أَبِيكُمْ وَتَكُونُوا مِنْ بَعْدِهِ قَوْمًا صَالِحِينَ قَالَ قَائِلٌ مِنْهُمْ لَا تَقْتُلُوا يُوسُفَ وَأَلْقُوهُ فِي غَيَابَتِ الْجُبِّ يَلْتَقِطْهُ بَعْضُ السَّيَّارَةِ إِنْ كُنْتُمْ فَاعِلِينَ قَالُوا يَا أَبَانَا مَا لَكَ لَا تَأْمَنَّا عَلَىٰ يُوسُفَ وَإِنَّا لَهُ لَنَاصِحُونَ أَرْسِلْهُ مَعَنَا غَدًا يَرْتَعْ وَيَلْعَبْ وَإِنَّا لَهُ لَحَافِظُونَ قَالَ إِنِّي لَيَحْزُنُنِي أَنْ تَذْهَبُوا بِهِ وَأَخَافُ أَنْ يَأْكُلَهُ الذِّئْبُ وَأَنْتُمْ عَنْهُ غَافِلُونَ قَالُوا لَئِنْ أَكَلَهُ الذِّئْبُ وَنَحْنُ عُصْبَةٌ إِنَّا إِذًا لَخَاسِرُونَ فَلَمَّا ذَهَبُوا بِهِ وَأَجْمَعُوا أَنْ يَجْعَلُوهُ فِي غَيَابَتِ الْجُبِّ وَأَوْحَيْنَا إِلَيْهِ لَتُنَبِّئَنَّهُمْ بِأَمْرِهِمْ هَٰذَا وَهُمْ لَا يَشْعُرُونَ وَجَاءُوا أَبَاهُمْ عِشَاءً يَبْكُونَ قَالُوا يَا أَبَانَا إِنَّا ذَهَبْنَا نَسْتَبِقُ وَتَرَكْنَا يُوسُفَ عِنْدَ مَتَاعِنَا فَأَكَلَهُ الذِّئْبُ وَمَا أَنْتَ بِمُؤْمِنٍ لَنَا وَلَوْ كُنَّا صَادِقِينَ وَجَاءُوا عَلَىٰ قَمِيصِهِ بِدَمٍ كَذِبٍ قَالَ بَلْ سَوَّلَتْ لَكُمْ أَنْفُسُكُمْ أَمْرًا فَصَبْرٌ جَمِيلٌ وَاللَّهُ الْمُسْتَعَانُ عَلَىٰ مَا تَصِفُونَ وَجَاءَتْ سَيَّارَةٌ فَأَرْسَلُوا وَارِدَهُمْ فَأَدْلَىٰ دَلْوَهُ قَالَ يَا بُشْرَىٰ هَٰذَا غُلَامٌ وَأَسَرُّوهُ بِضَاعَةً وَاللَّهُ عَلِيمٌ بِمَا يَعْمَلُونَ وَشَرَوْهُ بِثَمَنٍ بَخْسٍ دَرَاهِمَ مَعْدُودَةٍ وَكَانُوا فِيهِ مِنَ الزَّاهِدِينَ وَقَالَ الَّذِي اشْتَرَاهُ مِنْ مِصْرَ لِامْرَأَتِهِ أَكْرِمِي مَثْوَاهُ عَسَىٰ أَنْ يَنْفَعَنَا أَوْ نَتَّخِذَهُ وَلَدًا وَكَذَٰلِكَ مَكَّنَّا لِيُوسُفَ فِي الْأَرْضِ وَلِنُعَلِّمَهُ مِنْ تَأْوِيلِ الْأَحَادِيثِ وَاللَّهُ غَالِبٌ عَلَىٰ أَمْرِهِ وَلَٰكِنَّ أَكْثَرَ النَّاسِ لَا يَعْلَمُونَ وَلَمَّا بَلَغَ أَشُدَّهُ آتَيْنَاهُ حُكْمًا وَعِلْمًا وَكَذَٰلِكَ نَجْزِي الْمُحْسِنِينَ وَرَاوَدَتْهُ الَّتِي هُوَ فِي بَيْتِهَا عَنْ نَفْسِهِ وَغَلَّقَتِ الْأَبْوَابَ وَقَالَتْ هَيْتَ لَكَ قَالَ مَعَاذَ اللَّهِ إِنَّهُ رَبِّي أَحْسَنَ مَثْوَايَ إِنَّهُ لَا يُفْلِحُ الظَّالِمُونَ وَلَقَدْ هَمَّتْ بِهِ وَهَمَّ بِهَا لَوْلَا أَنْ رَأَىٰ بُرْهَانَ رَبِّهِ كَذَٰلِكَ لِنَصْرِفَ عَنْهُ السُّوءَ وَالْفَحْشَاءَ إِنَّهُ مِنْ عِبَادِنَا الْمُخْلَصِينَ وَاسْتَبَقَا الْبَابَ وَقَدَّتْ قَمِيصَهُ مِنْ دُبُرٍ وَأَلْفَيَا سَيِّدَهَا لَدَى الْبَابِ قَالَتْ مَا جَزَاءُ مَنْ أَرَادَ بِأَهْلِكَ سُوءًا إِلَّا أَنْ يُسْجَنَ أَوْ عَذَابٌ أَلِيمٌ قَالَ هِيَ رَاوَدَتْنِي عَنْ نَفْسِي وَشَهِدَ شَاهِدٌ مِنْ أَهْلِهَا إِنْ كَانَ قَمِيصُهُ قُدَّ مِنْ قُبُلٍ فَصَدَقَتْ وَهُوَ مِنَ الْكَاذِبِينَ وَإِنْ كَانَ قَمِيصُهُ قُدَّ مِنْ دُبُرٍ فَكَذَبَتْ وَهُوَ مِنَ الصَّادِقِينَ فَلَمَّا رَأَىٰ قَمِيصَهُ قُدَّ مِنْ دُبُرٍ قَالَ إِنَّهُ مِنْ كَيْدِكُنَّ إِنَّ كَيْدَكُنَّ عَظِيمٌ يُوسُفُ أَعْرِضْ عَنْ هَٰذَا وَاسْتَغْفِرِي لِذَنْبِكِ إِنَّكِ كُنْتِ مِنَ الْخَاطِئِينَ وَقَالَ نِسْوَةٌ فِي الْمَدِينَةِ امْرَأَتُ الْعَزِيزِ تُرَاوِدُ فَتَاهَا عَنْ نَفْسِهِ قَدْ شَغَفَهَا حُبًّا إِنَّا لَنَرَاهَا فِي ضَلَالٍ مُبِينٍ فَلَمَّا سَمِعَتْ بِمَكْرِهِنَّ أَرْسَلَتْ إِلَيْهِنَّ وَأَعْتَدَتْ لَهُنَّ مُتَّكَأً وَآتَتْ كُلَّ وَاحِدَةٍ مِنْهُنَّ سِكِّينًا وَقَالَتِ اخْرُجْ عَلَيْهِنَّ فَلَمَّا رَأَيْنَهُ أَكْبَرْنَهُ وَقَطَّعْنَ أَيْدِيَهُنَّ وَقُلْنَ حَاشَ لِلَّهِ مَا هَٰذَا بَشَرًا إِنْ هَٰذَا إِلَّا مَلَكٌ كَرِيمٌ قَالَتْ فَذَٰلِكُنَّ الَّذِي لُمْتُنَّنِي فِيهِ وَلَقَدْ رَاوَدْتُهُ عَنْ نَفْسِهِ فَاسْتَعْصَمَ وَلَئِنْ لَمْ يَفْعَلْ مَا آمُرُهُ لَيُسْجَنَنَّ وَلَيَكُونًا مِنَ الصَّاغِرِينَ قَالَ رَبِّ السِّجْنُ أَحَبُّ إِلَيَّ مِمَّا يَدْعُونَنِي إِلَيْهِ وَإِلَّا تَصْرِفْ عَنِّي كَيْدَهُنَّ أَصْبُ إِلَيْهِنَّ وَأَكُنْ مِنَ الْجَاهِلِينَ فَاسْتَجَابَ لَهُ رَبُّهُ فَصَرَفَ عَنْهُ كَيْدَهُنَّ إِنَّهُ هُوَ السَّمِيعُ الْعَلِيمُ ثُمَّ بَدَا لَهُمْ مِنْ بَعْدِ مَا رَأَوُا الْآيَاتِ لَيَسْجُنُنَّهُ حَتَّىٰ حِينٍ وَدَخَلَ مَعَهُ السِّجْنَ فَتَيَانِ قَالَ أَحَدُهُمَا إِنِّي أَرَانِي أَعْصِرُ خَمْرًا وَقَالَ الْآخَرُ إِنِّي أَرَانِي أَحْمِلُ فَوْقَ رَأْسِي خُبْزًا تَأْكُلُ الطَّيْرُ مِنْهُ نَبِّئْنَا بِتَأْوِيلِهِ إِنَّا نَرَاكَ مِنَ الْمُحْسِنِينَ قَالَ لَا يَأْتِيكُمَا طَعَامٌ تُرْزَقَانِهِ إِلَّا نَبَّأْتُكُمَا بِتَأْوِيلِهِ قَبْلَ أَنْ يَأْتِيَكُمَا ذَٰلِكُمَا مِمَّا عَلَّمَنِي رَبِّي إِنِّي تَرَكْتُ مِلَّةَ قَوْمٍ لَا يُؤْمِنُونَ بِاللَّهِ وَهُمْ بِالْآخِرَةِ هُمْ كَافِرُونَ وَاتَّبَعْتُ مِلَّةَ آبَائِي إِبْرَاهِيمَ وَإِسْحَاقَ وَيَعْقُوبَ مَا كَانَ لَنَا أَنْ نُشْرِكَ بِاللَّهِ مِنْ شَيْءٍ ذَٰلِكَ مِنْ فَضْلِ اللَّهِ عَلَيْنَا وَعَلَى النَّاسِ وَلَٰكِنَّ أَكْثَرَ النَّاسِ لَا يَشْكُرُونَ يَا صَاحِبَيِ السِّجْنِ أَأَرْبَابٌ مُتَفَرِّقُونَ خَيْرٌ أَمِ اللَّهُ الْوَاحِدُ الْقَهَّارُ مَا تَعْبُدُونَ مِنْ دُونِهِ إِلَّا أَسْمَاءً سَمَّيْتُمُوهَا أَنْتُمْ وَآبَاؤُكُمْ مَا أَنْزَلَ اللَّهُ بِهَا مِنْ سُلْطَانٍ إِنِ الْحُكْمُ إِلَّا لِلَّهِ أَمَرَ أَلَّا تَعْبُدُوا إِلَّا إِيَّاهُ ذَٰلِكَ الدِّينُ الْقَيِّمُ وَلَٰكِنَّ أَكْثَرَ النَّاسِ لَا يَعْلَمُونَ يَا صَاحِبَيِ السِّجْنِ أَمَّا أَحَدُكُمَا فَيَسْقِي رَبَّهُ خَمْرًا وَأَمَّا الْآخَرُ فَيُصْلَبُ فَتَأْكُلُ الطَّيْرُ مِنْ رَأْسِهِ قُضِيَ الْأَمْرُ الَّذِي فِيهِ تَسْتَفْتِيَانِ وَقَالَ لِلَّذِي ظَنَّ أَنَّهُ نَاجٍ مِنْهُمَا اذْكُرْنِي عِنْدَ رَبِّكَ فَأَنْسَاهُ الشَّيْطَانُ ذِكْرَ رَبِّهِ فَلَبِثَ فِي السِّجْنِ بِضْعَ سِنِينَ وَقَالَ الْمَلِكُ إِنِّي أَرَىٰ سَبْعَ بَقَرَاتٍ سِمَانٍ يَأْكُلُهُنَّ سَبْعٌ عِجَافٌ وَسَبْعَ سُنْبُلَاتٍ خُضْرٍ وَأُخَرَ يَابِسَاتٍ يَا أَيُّهَا الْمَلَأُ أَفْتُونِي فِي رُؤْيَايَ إِنْ كُنْتُمْ لِلرُّؤْيَا تَعْبُرُونَ قَالُوا أَضْغَاثُ أَحْلَامٍ وَمَا نَحْنُ بِتَأْوِيلِ الْأَحْلَامِ بِعَالِمِينَ وَقَالَ الَّذِي نَجَا مِنْهُمَا وَادَّكَرَ بَعْدَ أُمَّةٍ أَنَا أُنَبِّئُكُمْ بِتَأْوِيلِهِ فَأَرْسِلُونِ يُوسُفُ أَيُّهَا الصِّدِّيقُ أَفْتِنَا فِي سَبْعِ بَقَرَاتٍ سِمَانٍ يَأْكُلُهُنَّ سَبْعٌ عِجَافٌ وَسَبْعِ سُنْبُلَاتٍ خُضْرٍ وَأُخَرَ يَابِسَاتٍ لَعَلِّي أَرْجِعُ إِلَى النَّاسِ لَعَلَّهُمْ يَعْلَمُونَ";

/// Arabic script ranges: the base block, the supplements used by Persian and
/// Urdu, and the presentation forms.
fn is_arabic(cp: u32) -> bool {
    matches!(cp,
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF
        | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

/// Drop every vowel mark, leaving the rasm — the bare consonant skeleton that
/// ordinary Arabic prose is actually set in. Setting the same passage both
/// ways is the only way to see what the marks cost: the bare column is the
/// spacing and colour on their own, the vocalised one is what the marks do to
/// them.
fn strip_harakat(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !matches!(*c as u32,
                0x064B..=0x0655 | 0x0656..=0x065F | 0x0670 | 0x06D6..=0x06ED)
        })
        .collect()
}

/// Whether the font really supports Arabic, rather than happening to carry a
/// stray codepoint from the block. A handful of latin fonts encode an Arabic
/// comma or percent sign for punctuation coverage; one of those must not pull
/// in eleven pages the font cannot fill. The test is the letters themselves:
/// the proof needs most of the alphabet before its tables mean anything.
fn covers_arabic(cmap: &[(u32, u16)]) -> bool {
    let have = AR_LETTERS
        .iter()
        .filter(|(ch, _, _)| {
            ch.chars()
                .next()
                .is_some_and(|c| cmap.binary_search_by_key(&(c as u32), |&(cp, _)| cp).is_ok())
        })
        .count();
    have * 4 >= AR_LETTERS.len() * 3
}

/// The combining marks inside those ranges — harakat, hamza and the Quranic
/// annotations. They need a dotted circle to be legible on their own.
fn is_arabic_mark(cp: u32) -> bool {
    matches!(cp,
        0x064B..=0x065F | 0x0670 | 0x06D6..=0x06DC | 0x06DF..=0x06E4
        | 0x06E7..=0x06E8 | 0x06EA..=0x06ED | 0x0610..=0x061A | 0x08D3..=0x08FF)
}

/// Width of a single grid column.
fn col_w() -> f64 {
    (W - 2.0 * M - (COLS as f64 - 1.0) * GUTTER) / COLS as f64
}
/// Left edge of grid column `i` (0-based).
fn col_x(i: usize) -> f64 {
    M + i as f64 * (col_w() + GUTTER)
}
/// Width of a text block spanning `n` grid columns.
fn span_w(n: usize) -> f64 {
    n as f64 * col_w() + (n as f64 - 1.0) * GUTTER
}

// --- grid row geometry (y-up), so content can snap to the modular grid ------
fn grid_rh() -> f64 {
    (H - 2.0 * M - (GRID_ROWS as f64 - 1.0) * GUTTER) / GRID_ROWS as f64
}
/// Bottom edge (lower y) of grid row `r`, 0 = bottom row.
fn grid_row_bottom(r: u32) -> f64 {
    M + r as f64 * (grid_rh() + GUTTER)
}
/// Top edge (higher y) of grid row `r`.
fn grid_row_top(r: u32) -> f64 {
    grid_row_bottom(r) + grid_rh()
}

fn ink() -> Color {
    Color::rgb(0x23, 0x23, 0x23)
}
fn paper() -> Color {
    Color::rgb(0xff, 0xff, 0xff)
}
fn rule() -> Color {
    Color::rgb(0xcc, 0xcc, 0xcc)
}
// The information layer is pure black on white — no gray.
fn faint() -> Color {
    ink()
}
fn hair() -> Color {
    ink()
}
fn grid_red() -> Color {
    Color::rgb(0xe6, 0x9a, 0x9a) // light red guide grid
}

/// Format a float axis value without a trailing `.0` (400.0 -> "400").
fn num(v: f32) -> String {
    if v.fract().abs() < 1e-4 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

/// A paragraph of neutral prose to exercise the font in running text. Repeated
/// to fill tall columns; the same text across columns makes size / leading /
/// tracking differences directly comparable.
const SAMPLE: &str = "Typography is the craft of arranging letters so that language becomes visible. A typeface earns its keep in running text, where the rhythm of repeated forms, the fit between letters, and the balance of black and white decide whether a page invites reading or resists it. Grotesk designs strip ornament away and let structure carry the voice: even strokes, open counters, and a steady cadence from one word to the next. Set at reading sizes, the plain letters gather into a quiet, legible texture. This proof tests that texture across sizes, leading, and spacing before the design is trusted with real words. ";

fn filled(min_chars: usize) -> String {
    let mut s = String::new();
    while s.len() < min_chars {
        s.push_str(SAMPLE);
    }
    s
}

// --- introspection ---------------------------------------------------------

struct Axis {
    tag: String,
    min: f32,
    default: f32,
    max: f32,
}

struct NamedInstance {
    name: String,
    values: Vec<f32>,
}

struct FontFacts {
    family: String,
    version: String,
    upm: u16,
    glyph_count: u16,
    encoded: usize,
    axes: Vec<Axis>,
    instances: Vec<NamedInstance>,
    features: Vec<String>,
    /// (codepoint, glyph id), sorted by codepoint.
    cmap: Vec<(u32, u16)>,
    // vertical metrics, font units
    cap_height: i64,
    x_height: i64,
    ascent: i64,
    descent: i64,
}

fn tag_str(t: swash::Tag) -> String {
    String::from_utf8_lossy(&t.to_be_bytes()).trim().to_string()
}

fn introspect(data: &[u8]) -> Result<FontFacts, DesignBotError> {
    use swash::{FontRef, StringId};

    let font = FontRef::from_index(data, 0)
        .ok_or_else(|| DesignBotError::FontError("could not parse font".into()))?;

    let metrics = font.metrics(&[]);

    let (mut family, mut family_legacy, mut version) = (None, None, None);
    for s in font.localized_strings() {
        match s.id() {
            StringId::TypographicFamily if family.is_none() => family = Some(s.to_string()),
            StringId::Family if family_legacy.is_none() => family_legacy = Some(s.to_string()),
            StringId::Version if version.is_none() => version = Some(s.to_string()),
            _ => {}
        }
    }
    let family = family.or(family_legacy).unwrap_or_else(|| "Unknown".into());
    let version = version
        .map(|v| v.trim().trim_start_matches("Version").trim().to_string())
        .unwrap_or_default();

    let axes: Vec<Axis> = font
        .variations()
        .map(|v| Axis {
            tag: tag_str(v.tag()),
            min: v.min_value(),
            default: v.default_value(),
            max: v.max_value(),
        })
        .collect();

    let instances: Vec<NamedInstance> = font
        .instances()
        .map(|i| NamedInstance {
            name: i.name(None).map(|n| n.to_string()).unwrap_or_default(),
            values: i.values().collect(),
        })
        .collect();

    let mut features: Vec<String> = Vec::new();
    for f in font.features() {
        let t = tag_str(f.tag());
        if !features.contains(&t) {
            features.push(t);
        }
    }
    features.sort();

    let mut cmap: Vec<(u32, u16)> = Vec::new();
    font.charmap().enumerate(|cp, gid| {
        if gid != 0 {
            cmap.push((cp, gid));
        }
    });
    cmap.sort_by_key(|&(cp, _)| cp);
    cmap.dedup_by_key(|&mut (cp, _)| cp);
    let encoded = cmap.len();

    Ok(FontFacts {
        family,
        version,
        upm: metrics.units_per_em,
        glyph_count: metrics.glyph_count,
        encoded,
        axes,
        instances,
        features,
        cmap,
        cap_height: metrics.cap_height.round() as i64,
        x_height: metrics.x_height.round() as i64,
        ascent: metrics.ascent.round() as i64,
        descent: metrics.descent.round() as i64,
    })
}

/// Best-effort short git hash of the repo containing `path`; empty on failure.
fn git_hash(path: &Path) -> String {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::process::Command::new("git")
        .args(["-C"])
        .arg(dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Best-effort ISO date (YYYY-MM-DD) via the system `date`; empty on failure.
fn today() -> String {
    std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_default()
}

// --- proof builder ---------------------------------------------------------

struct Proof<'a> {
    ctx: Canvas,
    facts: &'a FontFacts,
    date: String,
    git: String,
    folio: usize,
    grid: bool,
    /// Advance width of the family name at 100 pt (for fitting the cover title).
    name_w_100: f64,
}

impl<'a> Proof<'a> {
    fn fam(&self) -> String {
        self.facts.family.clone()
    }

    /// Light-red Swiss modular grid overlay (guide while designing the proof).
    fn grid_overlay(&mut self) {
        Grid::modular(COLS as u32, GRID_ROWS)
            .margin(M)
            .gutter(GUTTER)
            .color(grid_red())
            .stroke_width(0.5)
            .draw(&mut self.ctx, W, H);
    }

    /// Small self-identifying header on every interior page + a hairline rule.
    fn running_head(&mut self, section: &str) {
        let y = H - 38.0;
        let fam = self.fam();
        let left = format!("{}  ·  {}", fam, section);
        let right = format!("{}   ·   {}", self.date, self.folio);
        self.ctx
            .no_stroke()
            .fill(faint())
            .font(MONO)
            .clear_font_variations()
            .font_size(MONO_SIZE)
            .tracking(0.2)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(&left, M, y);
        self.ctx.text_align(TextAlign::Right);
        self.ctx.text(&right, W - M, y);
        self.ctx.stroke(rule()).stroke_width(0.5);
        self.ctx.line(M, y - 7.0, W - M, y - 7.0);
    }

    /// Page headline — small monospace, never the proofed font (informational
    /// chrome stays in the mono information layer).
    fn page_title(&mut self, title: &str) {
        self.ctx
            .no_stroke()
            .fill(ink())
            .font(MONO)
            .clear_font_variations()
            .clear_font_features()
            .font_size(MONO_SIZE)
            .tracking(0.0)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(title, M, H - 64.0);
    }

    /// Start a fresh interior sheet: white, grid overlay, running head, title.
    fn new_sheet(&mut self, section: &str, title: &str) {
        self.folio += 1;
        self.ctx.new_page();
        self.ctx.background(paper());
        self.ctx.clear_font_features(); // pages start feature-free
        if self.grid {
            self.grid_overlay();
        }
        self.running_head(section);
        self.page_title(title);
    }

    /// A small monospace caption (black, mixed case).
    fn mono_caption(&mut self, text: &str, x: f64, y: f64) {
        self.ctx
            .no_stroke()
            .font(MONO)
            .clear_font_variations()
            .clear_font_features()
            .fill(ink())
            .font_size(MONO_SIZE)
            .tracking(0.0)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(text, x, y);
    }

    /// The wght axis (min, default, max), if present.
    fn wght_range(&self) -> Option<(f32, f32, f32)> {
        self.facts
            .axes
            .iter()
            .find(|a| a.tag == "wght")
            .map(|a| (a.min, a.default, a.max))
    }

    /// A monospace field for the cover: a column title sitting just above a
    /// hairline bar that is snapped to a grid line, with value lines below it.
    fn field(&mut self, x: f64, bar_y: f64, label: &str, values: &[String]) {
        self.ctx
            .no_stroke()
            .font(MONO)
            .clear_font_variations()
            .clear_font_features()
            .text_align(TextAlign::Left)
            .auto_line_height()
            .tracking(0.0)
            .fill(ink())
            .font_size(MONO_SIZE);
        // title above the bar
        self.ctx.text(label, x, bar_y + 7.0);
        // bar on the grid line, one column wide
        self.ctx.stroke(ink()).stroke_width(0.75);
        self.ctx.line(x, bar_y, x + col_w(), bar_y);
        // values below the bar
        self.ctx.no_stroke().fill(ink());
        let mut y = bar_y - 16.0;
        for v in values {
            self.ctx.text(v, x, y);
            y -= 13.0;
        }
    }

    // ---- pages ----

    fn cover(&mut self) {
        self.ctx.background(paper());
        if self.grid {
            self.grid_overlay();
        }

        // Weight waterfall of the family name: one 100-unit wght step per grid
        // row, top (min) to bottom (max), tightly (negatively) tracked. Sized to
        // fit a grid row's height, capped so the heaviest row never overflows
        // the content width.
        let fam = self.fam();
        let target = W - 2.0 * M;
        let gaps = (fam.chars().count().saturating_sub(1)) as f64;
        let track_frac = -0.025; // tight, negative tracking at display size
        // Size so the cap-height fills the grid row — small, even margins top
        // and bottom to match the small side margins — capped by the width so a
        // long family name can't overflow the page.
        let size_cap = grid_rh() * (self.facts.upm as f64 / self.facts.cap_height as f64) * 0.9;
        let size_w = target / (self.name_w_100 / 100.0 + track_frac * gaps);
        let size = size_cap.min(size_w);
        let track = track_frac * size;
        let cap_px = size * self.facts.cap_height as f64 / self.facts.upm as f64;

        // weights stepped by 100 across the wght axis (default 400 if no axis)
        let weights: Vec<f32> = match self.wght_range() {
            Some((min, _, max)) => {
                let mut v = Vec::new();
                let mut w = min;
                while w <= max + 0.5 {
                    v.push(w);
                    w += 100.0;
                }
                v
            }
            None => vec![self.facts.axes.first().map(|a| a.default).unwrap_or(400.0)],
        };
        // top four grid rows (indices 5..=2); row 1 is skipped, row 0 holds the
        // column values.
        for (i, &w) in weights.iter().take(4).enumerate() {
            let r = (GRID_ROWS - 1) - i as u32;
            let center = (grid_row_bottom(r) + grid_row_top(r)) / 2.0;
            let baseline = center - cap_px / 2.0;
            self.ctx
                .no_stroke()
                .fill(ink())
                .font(&fam)
                .clear_font_variations()
                .font_variation("wght", w)
                .font_size(size)
                .tracking(track)
                .auto_line_height()
                .text_align(TextAlign::Left);
            self.ctx.text(&fam, M, baseline);
        }

        // Technical data columns at the bottom: title + bar on the bottom edge
        // of grid row 5 (index 1), values filling grid row 6 (index 0).
        let bar_y = grid_row_bottom(1);
        let axes: Vec<String> = self
            .facts
            .axes
            .iter()
            .map(|a| format!("{} {}–{}", a.tag, num(a.min), num(a.max)))
            .collect();
        let instances: Vec<String> = self
            .facts
            .instances
            .iter()
            .map(|i| {
                let v = i.values.iter().map(|x| num(*x)).collect::<Vec<_>>().join("/");
                if v.is_empty() {
                    i.name.clone()
                } else {
                    format!("{} {}", i.name, v)
                }
            })
            .collect();
        let character = vec![
            format!("{} glyphs", self.facts.glyph_count),
            format!("{} encoded", self.facts.encoded),
        ];
        // features wrapped 2 per line
        let features: Vec<String> = self
            .facts
            .features
            .chunks(2)
            .map(|c| c.join(" "))
            .collect();
        let metrics = vec![
            format!("{} upm", self.facts.upm),
            format!("cap {}", self.facts.cap_height),
            format!("x-height {}", self.facts.x_height),
            format!("ascender {}", self.facts.ascent),
            format!("descender {}", -self.facts.descent.abs()),
        ];
        let mut meta = Vec::new();
        if !self.facts.version.is_empty() {
            meta.push(format!("version {}", self.facts.version));
        }
        if !self.git.is_empty() {
            meta.push(format!("commit {}", self.git));
        }
        if !self.date.is_empty() {
            meta.push(format!("generated {}", self.date));
        }

        self.field(col_x(0), bar_y, "Axes", &axes);
        self.field(col_x(1), bar_y, "Instances", &instances);
        self.field(col_x(2), bar_y, "Character", &character);
        self.field(col_x(3), bar_y, "Features", &features);
        self.field(col_x(4), bar_y, "Metrics", &metrics);
        self.field(col_x(5), bar_y, "Build", &meta);
    }

    fn char_set(&mut self) {
        self.new_sheet("Character Set", "Character Set");
        let glyphs: Vec<u32> = self
            .facts
            .cmap
            .iter()
            .map(|&(cp, _)| cp)
            .filter(|&cp| cp >= 0x20 && char::from_u32(cp).is_some())
            .collect();
        let count = glyphs.len().max(1);

        let content_w = W - 2.0 * M;
        let top = H - 104.0;
        let content_h = top - M;

        // Columns are a multiple of the layout grid (COLS) so cells fill the
        // page width exactly and align to it; pick the multiple whose cells
        // come out closest to square. Rows then follow, and the cell height is
        // derived to fit every glyph on this one page.
        let ncols = {
            let ideal = (1.5 * count as f64).sqrt() / COLS as f64;
            (ideal.round().max(1.0) as usize * COLS).max(COLS)
        };
        let nrows = count.div_ceil(ncols);
        let cell_w = content_w / ncols as f64;
        let cell_h = content_h / nrows as f64;
        let glyph_size = (cell_h * 0.52).min(cell_w * 0.72);
        let labels = cell_h >= 20.0;

        for (idx, &cp) in glyphs.iter().enumerate() {
            let ch = char::from_u32(cp).unwrap();
            let col = idx % ncols;
            let row = idx / ncols;
            let x = M + col as f64 * cell_w;
            let cell_bottom = top - (row + 1) as f64 * cell_h;
            let cx = x + cell_w / 2.0;

            self.ctx.no_fill().stroke(rule()).stroke_width(0.3);
            self.ctx.rect(x, cell_bottom, cell_w, cell_h);

            self.ctx
                .no_stroke()
                .fill(ink())
                .font(&self.facts.family)
                .clear_font_variations()
                .font_variation("wght", 400.0)
                .font_size(glyph_size)
                .tracking(0.0)
                .auto_line_height()
                .text_align(TextAlign::Center);
            let base = cell_bottom + if labels { cell_h * 0.40 } else { cell_h * 0.32 };
            self.ctx.text(&ch.to_string(), cx, base);

            if labels {
                self.ctx
                    .no_stroke()
                    .font(MONO)
                    .clear_font_variations()
                    .fill(hair())
                    .font_size(4.5)
                    .text_align(TextAlign::Center);
                self.ctx.text(&format!("{:04X}", cp), cx, cell_bottom + 2.5);
            }
        }
    }

    fn waterfall(&mut self) {
        self.new_sheet("Waterfall", "Waterfall");
        let sizes = [72.0, 54.0, 42.0, 32.0, 24.0, 18.0, 14.0, 12.0, 10.0, 9.0, 8.0];
        let sample = "Hamburgefonstiv";
        let mut y = H - 152.0;
        self.ctx
            .font(&self.facts.family)
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .tracking(0.0)
            .auto_line_height()
            .text_align(TextAlign::Left)
            .no_stroke();
        for &s in &sizes {
            if y - s < M {
                break;
            }
            self.ctx.font(MONO).fill(faint()).font_size(MONO_SIZE);
            self.ctx.text(&format!("{}", s as i64), M, y);
            self.ctx.font(&self.facts.family).fill(ink()).font_size(s);
            self.ctx.text(sample, M + 34.0, y);
            y -= s * 1.26 + 5.0;
        }
    }

    /// A single column of running SAMPLE text at a given size/leading/tracking,
    /// spanning `span` grid columns from column `start`, with a mono caption.
    fn text_column(&mut self, start: usize, span: usize, size: f64, leading: f64, track: f64) {
        let x = col_x(start);
        let w = span_w(span);
        let top = H - 100.0;
        let bh = top - M;

        // mono caption above the column
        let cap = format!(
            "{}/{}  ·  tracking {:+.1}",
            num(size as f32),
            num(leading as f32),
            track
        );
        self.ctx
            .no_stroke()
            .font(MONO)
            .clear_font_variations()
            .fill(faint())
            .font_size(MONO_SIZE)
            .tracking(0.4)
            .auto_line_height()
            .text_align(TextAlign::Left);
        self.ctx.text(&cap.to_uppercase(), x, top + 12.0);

        // the text block
        self.ctx
            .fill(ink())
            .font(&self.facts.family)
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .font_size(size)
            .line_height(leading)
            .tracking(track)
            .text_align(TextAlign::Left);
        self.ctx.text_box(&filled(1400), x, M, w, bh);
    }

    /// Three columns at ascending reading sizes with matched leading.
    fn text_sizes(&mut self) {
        self.new_sheet("Text · Reading Sizes", "Text — Reading Sizes");
        self.text_column(0, 2, 8.5, 12.0, 0.0);
        self.text_column(2, 2, 10.0, 14.5, 0.0);
        self.text_column(4, 2, 12.5, 17.5, 0.0);
    }

    /// Same size, three leadings — the effect of line spacing on color.
    fn text_leading(&mut self) {
        self.new_sheet("Text · Leading", "Text — Leading");
        self.text_column(0, 2, 10.0, 12.0, 0.0);
        self.text_column(2, 2, 10.0, 14.5, 0.0);
        self.text_column(4, 2, 10.0, 17.5, 0.0);
    }

    /// Same size/leading, three tracking values — tighter to looser.
    fn text_tracking(&mut self) {
        self.new_sheet("Text · Tracking", "Text — Tracking");
        self.text_column(0, 2, 10.5, 15.0, -0.4);
        self.text_column(2, 2, 10.5, 15.0, 0.0);
        self.text_column(4, 2, 10.5, 15.0, 0.6);
    }

    /// Spacing proof — control strings with kerning OFF, so raw sidebearings
    /// are what you judge. Each letter is set between its category's controls
    /// (H/O for caps, n/o for lowercase, 0/1 for figures).
    fn spacing(&mut self) {
        self.new_sheet("Spacing", "Spacing — kerning off");
        let fam = self.fam();
        // Each letter set between its controls on both sides (H/O, n/o, 0/1),
        // so both sidebearings read at a glance. Kerning + ligatures off.
        let caps: String = ('A'..='Z').map(|c| format!("H{c}HO{c}O ")).collect();
        let lc: String = ('a'..='z').map(|c| format!("n{c}no{c}o ")).collect();
        let digs: String = ('0'..='9').map(|c| format!("0{c}01{c}1 ")).collect();
        let w = W - 2.0 * M;
        // Fixed, well-separated group positions (text_box does not clip height).
        let groups = [("Capitals", caps, H - 116.0), ("Lowercase", lc, H - 268.0), ("Figures", digs, H - 420.0)];
        for (label, s, cap_y) in groups {
            self.mono_caption(label, M, cap_y);
            self.ctx
                .no_stroke()
                .fill(ink())
                .font(&fam)
                .clear_font_variations()
                .font_variation("wght", 400.0)
                .clear_font_features()
                .font_feature("kern", 0)
                .font_feature("liga", 0)
                .font_size(18.0)
                .line_height(25.0)
                .tracking(0.0)
                .text_align(TextAlign::Left);
            self.ctx.text_box(&s, M, cap_y - 130.0, w, 118.0);
        }
    }

    /// Figures — proportional vs tabular (tnum), a tabular column-alignment
    /// test, and currency.
    fn figures(&mut self) {
        self.new_sheet("Figures", "Figures & Numerals");
        let fam = self.fam();
        let digits = "0 1 2 3 4 5 6 7 8 9";

        // Proportional (default) vs tabular (tnum) at display size.
        self.mono_caption("Proportional (default)", M, H - 116.0);
        self.set_body(&fam, 38.0);
        self.ctx.clear_font_features();
        self.ctx.text(digits, M, H - 160.0);

        self.mono_caption("Tabular (tnum)", M, H - 208.0);
        self.set_body(&fam, 38.0);
        self.ctx.clear_font_features().font_feature("tnum", 1);
        self.ctx.text(digits, M, H - 252.0);

        // Tabular column test: a right-aligned price stack; with tnum every
        // figure is the same width so decimals line up.
        let prices = "1,204.50\n38.05\n1,899,000.00\n7.25\n640.80";
        self.mono_caption("Tabular column · decimals align", col_x(4), H - 116.0);
        self.set_body(&fam, 16.0);
        self.ctx
            .clear_font_features()
            .font_feature("tnum", 1)
            .line_height(22.0)
            .text_align(TextAlign::Right);
        self.ctx.text_box(prices, col_x(4), H - 300.0, span_w(2), 170.0);

        // Currency.
        self.mono_caption("Currency", M, M + 84.0);
        self.set_body(&fam, 26.0);
        self.ctx.clear_font_features();
        self.ctx.text("$1,234.56  €1,234.56  £1,234.56  ¥1,234  ¢99", M, M + 44.0);
    }

    /// Accents & diacritics — composed letters in lowercase and caps, plus real
    /// words, to check mark placement and cap-height vs lowercase accents.
    fn accents(&mut self) {
        self.new_sheet("Diacritics", "Accents & Diacritics");
        let fam = self.fam();
        let lc = [
            "à á â ã ä å ā ă ą",
            "è é ê ë ē ĕ ė ę ě",
            "ì í î ï ĩ ī ĭ į ı",
            "ò ó ô õ ö ø ō ŏ ő",
            "ù ú û ü ũ ū ŭ ů ű",
            "ç ć ĉ ċ č   ñ ń ņ ň   š ś ş   ž ź ż   ý ÿ   ł đ",
        ];
        let uc = [
            "À Á Â Ã Ä Å Ā Ă Ą",
            "È É Ê Ë Ē Ĕ Ė Ę Ě",
            "Ò Ó Ô Õ Ö Ø   Ç Ć Č   Ñ Ń Ň   Š Ž Ý",
        ];
        let words = "café · résumé · naïve · Zürich · Škoda · piñata · œuvre · Straße";

        let mut y = H - 116.0;
        self.mono_caption("Lowercase", M, y);
        y -= 30.0;
        for row in lc {
            self.set_body(&fam, 24.0);
            self.ctx.text(row, M, y);
            y -= 34.0;
        }
        y -= 12.0;
        self.mono_caption("Capitals", M, y);
        y -= 30.0;
        for row in uc {
            self.set_body(&fam, 24.0);
            self.ctx.text(row, M, y);
            y -= 34.0;
        }
        y -= 12.0;
        self.mono_caption("In words", M, y);
        y -= 28.0;
        self.set_body(&fam, 22.0);
        self.ctx.text(words, M, y);
    }

    /// Kerning — classic problem pairs (kern on), then the same words set with
    /// kerning off and on so pair adjustments are directly visible.
    fn kerning(&mut self) {
        self.new_sheet("Kerning", "Kerning");
        let fam = self.fam();
        let pairs = [
            "AV AW AY AT AU VA WA YA",
            "To Ta Te Tr Tu Ty Tw",
            "Yo Ya Ve Vo We Wo Pa",
            "r. r, y. y, w, f) P. F.",
        ];
        let mut y = H - 118.0;
        self.mono_caption("Problem pairs · kerning on", M, y);
        y -= 34.0;
        for row in pairs {
            self.set_body(&fam, 30.0);
            self.ctx.clear_font_features(); // kern on (default)
            self.ctx.text(row, M, y);
            y -= 40.0;
        }

        let words = "Toronto  Affinity  Voyage  Water  Yellow  LAWYER";
        y -= 16.0;
        self.mono_caption("Kerning off", M, y);
        y -= 32.0;
        self.set_body(&fam, 26.0);
        self.ctx.clear_font_features().font_feature("kern", 0);
        self.ctx.text(words, M, y);
        y -= 44.0;
        self.mono_caption("Kerning on", M, y);
        y -= 32.0;
        self.set_body(&fam, 26.0);
        self.ctx.clear_font_features();
        self.ctx.text(words, M, y);
    }

    /// Weight waterfall — the same line at every step of the wght axis, same
    /// size, to read the weight progression and spot interpolation kinks.
    fn weight_waterfall(&mut self) {
        self.new_sheet("Weight", "Weight Waterfall");
        let fam = self.fam();
        let sample = "Hamburgefonstiv 0123";
        let size = 38.0;
        let Some((min, _def, max)) = self.wght_range() else {
            return;
        };
        let steps = 7usize;
        let mut y = H - 150.0;
        for i in 0..steps {
            let wght = min + (max - min) * (i as f64 / (steps - 1) as f64) as f32;
            self.ctx.font(MONO).clear_font_variations().clear_font_features();
            self.ctx.fill(faint()).font_size(MONO_SIZE).text_align(TextAlign::Left);
            self.ctx.text(&format!("{}", wght.round() as i64), M, y);
            self.ctx
                .font(&fam)
                .clear_font_variations()
                .font_variation("wght", wght)
                .fill(ink())
                .font_size(size)
                .tracking(0.0)
                .auto_line_height();
            self.ctx.text(sample, M + 44.0, y);
            y -= size * 1.32 + 4.0;
        }
    }

    /// Interpolation grid — each test glyph shown across the wght axis so kinks,
    /// reversals, and drifting overshoots jump out (glyph rows × weight columns).
    fn interpolation(&mut self) {
        self.new_sheet("Interpolation", "Interpolation");
        let fam = self.fam();
        let glyphs = ['o', 'n', 'H', 'a', 'e', 'g', 'R', '&', '2', '@'];
        let Some((min, _def, max)) = self.wght_range() else {
            return;
        };
        let ncols = 6usize;
        let content_w = W - 2.0 * M;
        let cell_w = content_w / ncols as f64;
        let top = H - 132.0;
        let row_h = (top - M) / glyphs.len() as f64;
        let gsize = (row_h * 0.62).min(cell_w * 0.5);

        // weight column headers
        for c in 0..ncols {
            let wght = min + (max - min) * (c as f64 / (ncols - 1) as f64) as f32;
            let cx = M + c as f64 * cell_w + cell_w / 2.0;
            self.ctx
                .no_stroke()
                .font(MONO)
                .clear_font_variations()
                .clear_font_features()
                .fill(faint())
                .font_size(MONO_SIZE)
                .tracking(0.3)
                .text_align(TextAlign::Center);
            self.ctx.text(&format!("{}", wght.round() as i64), cx, top + 10.0);
        }

        for (r, &ch) in glyphs.iter().enumerate() {
            let cy_top = top - r as f64 * row_h;
            let base = cy_top - row_h + row_h * 0.30;
            for c in 0..ncols {
                let wght = min + (max - min) * (c as f64 / (ncols - 1) as f64) as f32;
                let cx = M + c as f64 * cell_w + cell_w / 2.0;
                self.ctx
                    .no_stroke()
                    .fill(ink())
                    .font(&fam)
                    .clear_font_variations()
                    .font_variation("wght", wght)
                    .font_size(gsize)
                    .tracking(0.0)
                    .auto_line_height()
                    .text_align(TextAlign::Center);
                self.ctx.text(&ch.to_string(), cx, base);
            }
        }
    }

    // --- Arabic ------------------------------------------------------------

    /// Every encoded Arabic codepoint, laid out like the main character set.
    /// Covers the base block plus the presentation forms, so a missing or
    /// misencoded glyph shows up as a gap.
    fn arabic_char_set(&mut self) {
        self.new_sheet("Arabic", "Arabic Character Set");
        let glyphs: Vec<u32> = self
            .facts
            .cmap
            .iter()
            .map(|&(cp, _)| cp)
            .filter(|&cp| is_arabic(cp) && char::from_u32(cp).is_some())
            .collect();
        let count = glyphs.len().max(1);

        let content_w = W - 2.0 * M;
        let top = H - 104.0;
        let content_h = top - M;
        let ncols = {
            let ideal = (1.5 * count as f64).sqrt() / COLS as f64;
            (ideal.round().max(1.0) as usize * COLS).max(COLS)
        };
        let nrows = count.div_ceil(ncols);
        let cell_w = content_w / ncols as f64;
        let cell_h = content_h / nrows as f64;
        let glyph_size = (cell_h * 0.52).min(cell_w * 0.72);
        let labels = cell_h >= 20.0;

        for (idx, &cp) in glyphs.iter().enumerate() {
            let ch = char::from_u32(cp).unwrap();
            let x = M + (idx % ncols) as f64 * cell_w;
            let cell_bottom = top - (idx / ncols + 1) as f64 * cell_h;
            let cx = x + cell_w / 2.0;

            self.ctx.no_fill().stroke(rule()).stroke_width(0.3);
            self.ctx.rect(x, cell_bottom, cell_w, cell_h);

            // Combining marks render on a dotted circle so they are visible
            // and their attachment height is legible on their own.
            let shown = if is_arabic_mark(cp) {
                format!("\u{25CC}{}", ch)
            } else {
                ch.to_string()
            };
            self.ctx
                .no_stroke()
                .fill(ink())
                .font(&self.facts.family)
                .clear_font_variations()
                .font_variation("wght", 400.0)
                .font_size(glyph_size)
                .tracking(0.0)
                .auto_line_height()
                .text_align(TextAlign::Center);
            let base = cell_bottom + if labels { cell_h * 0.40 } else { cell_h * 0.32 };
            self.ctx.text(&shown, cx, base);

            if labels {
                self.ctx
                    .no_stroke()
                    .font(MONO)
                    .clear_font_variations()
                    .fill(hair())
                    .font_size(4.5)
                    .text_align(TextAlign::Center);
                self.ctx.text(&format!("{:04X}", cp), cx, cell_bottom + 2.5);
            }
        }
    }

    /// Right-aligned body text. Arabic reads from the right edge, so every
    /// Arabic row on these pages is set flush right against the same margin.
    fn set_body_rtl(&mut self, fam: &str, size: f64) {
        self.ctx
            .no_stroke()
            .fill(ink())
            .font(fam)
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .font_size(size)
            .tracking(0.0)
            .auto_line_height()
            .text_align(TextAlign::Right);
    }

    /// One letter in a chosen positional form, using ZWJ rather than tatweel
    /// so the cell shows the letterform and nothing else. `prev` means a
    /// letter joins on the reading-preceding side (giving final or medial).
    fn positional(ch: &str, prev: bool, next: bool) -> String {
        format!(
            "{}{}{}",
            if prev { ZWJ } else { "" },
            ch,
            if next { ZWJ } else { "" }
        )
    }

    /// The four positional forms of every letter, one row each. This is the
    /// page that catches a wrong or missing `init`/`medi`/`fina` substitution
    /// and any joining stub that does not meet its neighbour.
    fn arabic_joining(&mut self) {
        self.new_sheet("Arabic", "Joining — Positional Forms");
        let fam = self.fam();
        let headers = ["isolated", "final", "medial", "initial"];

        // Two half-page tables side by side, 14 letters each.
        let half = AR_LETTERS.len().div_ceil(2);
        let block_w = (W - 2.0 * M - GUTTER) / 2.0;
        let top = H - 118.0;
        let row_h = (top - M) / half as f64;

        for (bi, chunk) in AR_LETTERS.chunks(half).enumerate() {
            let x0 = M + bi as f64 * (block_w + GUTTER);
            let cell_w = block_w / 5.0;

            for (i, h) in headers.iter().enumerate() {
                self.ctx
                    .no_stroke()
                    .font(MONO)
                    .clear_font_variations()
                    .fill(hair())
                    .font_size(MONO_SIZE * 0.72)
                    .text_align(TextAlign::Center);
                self.ctx
                    .text(h, x0 + cell_w * (1.5 + i as f64), top + 8.0);
            }

            for (ri, &(ch, label, dual)) in chunk.iter().enumerate() {
                let base_y = top - (ri as f64 + 0.72) * row_h;

                self.ctx
                    .no_stroke()
                    .font(MONO)
                    .clear_font_variations()
                    .fill(faint())
                    .font_size(MONO_SIZE * 0.8)
                    .text_align(TextAlign::Left);
                self.ctx.text(label, x0, base_y);

                // isolated, final, medial, initial
                let forms = [
                    Some(Self::positional(ch, false, false)),
                    Some(Self::positional(ch, true, false)),
                    dual.then(|| Self::positional(ch, true, true)),
                    dual.then(|| Self::positional(ch, false, true)),
                ];
                for (i, form) in forms.iter().enumerate() {
                    let Some(form) = form else { continue };
                    self.ctx
                        .no_stroke()
                        .fill(ink())
                        .font(&fam)
                        .clear_font_variations()
                        .font_variation("wght", 400.0)
                        .font_size(row_h * 0.5)
                        .tracking(0.0)
                        .auto_line_height()
                        .text_align(TextAlign::Center);
                    self.ctx
                        .text(form, x0 + cell_w * (1.5 + i as f64), base_y);
                }
            }
        }
    }

    /// Each letter tripled and joined, then standing alone: ننن ن. The oldest
    /// Arabic spacing test there is. A run of one letter exposes rhythm no
    /// mixed word can — a tooth that is too wide, a join that sits at the
    /// wrong height, a counter that closes up when its neighbours arrive —
    /// and setting the isolated form beside it shows whether the joined and
    /// unjoined shapes belong to the same letter.
    fn arabic_repetition(&mut self) {
        self.new_sheet("Arabic", "Repetition — Joined Runs & Isolated");
        let fam = self.fam();

        let ncols = 3;
        let nrows = AR_LETTERS.len().div_ceil(ncols);
        let top = H - 118.0;
        let cell_w = (W - 2.0 * M) / ncols as f64;
        let cell_h = (top - M) / nrows as f64;

        for (i, &(ch, label, _)) in AR_LETTERS.iter().enumerate() {
            // columns run right to left, like the script
            let cx = M + (ncols - 1 - i / nrows) as f64 * cell_w;
            let base_y = top - ((i % nrows) as f64 + 0.62) * cell_h;

            self.ctx
                .no_stroke()
                .font(MONO)
                .clear_font_variations()
                .fill(faint())
                .font_size(MONO_SIZE * 0.75)
                .text_align(TextAlign::Left);
            self.ctx.text(label, cx, base_y);

            // three of the letter joined, a space, then the letter alone
            let run = format!("{ch}{ch}{ch} {ch}");
            self.ctx
                .no_stroke()
                .fill(ink())
                .font(&fam)
                .clear_font_variations()
                .font_variation("wght", 400.0)
                .font_size(cell_h * 0.40)
                .tracking(0.0)
                .auto_line_height()
                .text_align(TextAlign::Right);
            self.ctx.text(&run, cx + cell_w - 16.0, base_y);
        }
    }

    /// Every vowel mark on every skeleton. Each column is one mark, each row
    /// one base: scanning down a column shows whether the mark sits on a level
    /// line, and across a row whether the anchors agree with each other.
    fn arabic_marks(&mut self) {
        // Split across sheets so the cells stay big enough to judge a few
        // units of drift; a single page squeezes below-marks into the rule.
        let per_page = AR_MARK_BASES.len().div_ceil(2);
        for (page, bases) in AR_MARK_BASES.chunks(per_page).enumerate() {
            self.arabic_marks_sheet(bases, page + 1);
        }
    }

    fn arabic_marks_sheet(&mut self, bases: &[&str], page: usize) {
        self.new_sheet(
            "Arabic",
            &format!("Mark Attachment — Harakat on Every Skeleton ({page}/2)"),
        );
        let fam = self.fam();

        let top = H - 122.0;
        let label_w = 34.0;
        let cell_w = (W - 2.0 * M - label_w) / AR_HARAKAT.len() as f64;
        let row_h = (top - M) / bases.len() as f64;

        for (ci, &(_, name)) in AR_HARAKAT.iter().enumerate() {
            self.ctx
                .no_stroke()
                .font(MONO)
                .clear_font_variations()
                .fill(hair())
                .font_size(MONO_SIZE * 0.72)
                .text_align(TextAlign::Center);
            self.ctx.text(
                name,
                M + label_w + cell_w * (ci as f64 + 0.5),
                top + 8.0,
            );
        }

        for (ri, &base) in bases.iter().enumerate() {
            // The baseline rule is the reference: scanning down a column, every
            // above-mark should sit the same distance off it.
            let base_y = top - (ri as f64 + 0.62) * row_h;
            self.ctx.no_fill().stroke(hair()).stroke_width(0.25);
            self.ctx.line(M, base_y, W - M, base_y);

            for (ci, &(mark, _)) in AR_HARAKAT.iter().enumerate() {
                self.ctx
                    .no_stroke()
                    .fill(ink())
                    .font(&fam)
                    .clear_font_variations()
                    .font_variation("wght", 400.0)
                    .font_size(row_h * 0.52)
                    .tracking(0.0)
                    .auto_line_height()
                    .text_align(TextAlign::Center);
                self.ctx.text(
                    &format!("{}{}", base, mark),
                    M + label_w + cell_w * (ci as f64 + 0.5),
                    base_y,
                );
            }
        }
    }

    /// Dots against dots, and the lam-alef ligatures. Both are places where
    /// Arabic goes wrong quietly: clusters merge into a blob, or a required
    /// ligature silently fails to form.
    fn arabic_clusters(&mut self) {
        self.new_sheet("Arabic", "Dot Clusters & Ligatures");
        let fam = self.fam();
        let half = (W - 2.0 * M - GUTTER) / 2.0;
        // right column first: Arabic is read from the right edge of the page
        let cols: [(f64, &str, &[(&str, &str)]); 2] = [
            (W - M, "Adjacent dotted letters", AR_DOT_RUNS),
            (M + half, "Lam-alef — all four must ligate", AR_LAM_ALEF),
        ];
        let top = H - 128.0;

        for (right_edge, caption, rows) in cols {
            let label_x = right_edge - half;
            self.mono_caption(caption, label_x, top);
            let mut y = top - 48.0;
            let step = ((top - 48.0 - M) / AR_DOT_RUNS.len() as f64).min(58.0);
            for (seq, latin) in rows {
                self.set_body_rtl(&fam, step * 0.62);
                self.ctx.text(seq, right_edge, y);
                self.ctx
                    .no_stroke()
                    .font(MONO)
                    .clear_font_variations()
                    .fill(faint())
                    .font_size(MONO_SIZE * 0.8)
                    .text_align(TextAlign::Left);
                self.ctx.text(latin, label_x, y);
                y -= step;
            }
        }
    }

    /// Arabic in running text, plain and fully vocalised, down the sizes.
    /// Vocalised is where mark-to-mark stacking and vertical collisions show.
    fn arabic_text(&mut self) {
        self.new_sheet("Arabic", "Running Text & Waterfall");
        let fam = self.fam();
        let mut y = H - 126.0;

        self.mono_caption("Abjad — every letter once", M, y);
        y -= 40.0;
        for size in [30.0_f64, 22.0, 16.0] {
            self.set_body_rtl(&fam, size);
            self.ctx.text(AR_TEXT_PLAIN, W - M, y);
            y -= size * 1.7;
        }

        y -= 22.0;
        self.mono_caption("Vocalised — mark stacking in context", M, y);
        y -= 44.0;
        for size in [34.0_f64, 26.0, 20.0, 15.0, 11.0] {
            self.set_body_rtl(&fam, size);
            self.ctx.text(AR_TEXT_VOCAL, W - M, y);
            y -= size * 1.9;
        }
    }

    /// The same Arabic line at each named instance, so the eye can check that
    /// mark anchors do not drift sideways or vertically across the axis.
    fn arabic_weights(&mut self) {
        if self.facts.instances.is_empty() {
            return;
        }
        self.new_sheet("Arabic", "Weights — Anchor Drift Across the Axis");
        let fam = self.fam();
        let sample = "\u{0628}\u{064E}\u{0628}\u{064E}\u{0628}\u{064E} \u{0627}\u{064E} \u{0645}\u{064F}\u{062D}\u{064E}\u{0645}\u{0651}\u{064E}\u{062F}";

        let instances: Vec<(String, Vec<f32>)> = self
            .facts
            .instances
            .iter()
            .map(|i| (i.name.clone(), i.values.clone()))
            .collect();
        let top = H - 130.0;
        let step = (top - M) / instances.len() as f64;

        for (i, (name, values)) in instances.iter().enumerate() {
            let y = top - (i as f64 + 0.62) * step;
            self.ctx
                .no_stroke()
                .font(MONO)
                .clear_font_variations()
                .fill(faint())
                .font_size(MONO_SIZE * 0.8)
                .text_align(TextAlign::Left);
            self.ctx.text(name, M, y);

            self.ctx
                .no_stroke()
                .fill(ink())
                .font(&fam)
                .clear_font_variations()
                .font_size(step * 0.46)
                .tracking(0.0)
                .auto_line_height()
                .text_align(TextAlign::Right);
            for (axis, value) in self.facts.axes.iter().zip(values.iter()) {
                self.ctx.font_variation(&axis.tag, *value);
            }
            self.ctx.text(sample, W - M, y);
        }
    }

    /// Al-Fatiha, ayah per line, then the same ayah down the sizes. Fully
    /// vocalised scripture is the hardest ordinary test of mark stacking:
    /// shadda over fatha, tanwin, superscript alef and alef wasla all appear
    /// inside seven short lines, so whatever collides shows up here first.
    fn arabic_quran(&mut self) {
        self.new_sheet("Arabic", "Quranic Text — Al-Fatiha (Uthmani)");
        let fam = self.fam();
        let mut y = H - 128.0;

        for (i, ayah) in AR_FATIHA.iter().enumerate() {
            self.set_body_rtl(&fam, 26.0);
            self.ctx.text(ayah, W - M, y);
            self.ctx
                .no_stroke()
                .font(MONO)
                .clear_font_variations()
                .fill(faint())
                .font_size(MONO_SIZE * 0.8)
                .text_align(TextAlign::Left);
            self.ctx.text(&format!("{}", i + 1), M, y);
            y -= 44.0;
        }

        y -= 14.0;
        self.mono_caption("Down the sizes", M, y);
        y -= 32.0;
        for size in [20.0_f64, 15.0, 11.0, 8.0] {
            self.set_body_rtl(&fam, size);
            self.ctx.text(AR_FATIHA[1], W - M, y);
            y -= size * 2.0;
        }
    }

    /// Three short surahs as running paragraphs in the simple orthography.
    /// Wrapping is where line-breaking meets mark stacking.
    fn arabic_quran_running(&mut self) {
        self.new_sheet("Arabic", "Quranic Text — Running Paragraphs");
        let fam = self.fam();
        let top = H - 126.0;
        let block_h = (top - M) / AR_SURAHS.len() as f64;

        for (i, (label, text)) in AR_SURAHS.iter().enumerate() {
            let block_top = top - i as f64 * block_h;
            self.mono_caption(label, M, block_top);
            self.ctx
                .no_stroke()
                .fill(ink())
                .font(&fam)
                .clear_font_variations()
                .font_variation("wght", 400.0)
                .font_size(19.0)
                .tracking(0.0)
                .line_height(38.0)
                .text_align(TextAlign::Right);
            self.ctx.text_box(
                text,
                M,
                block_top - block_h + 22.0,
                W - 2.0 * M,
                block_h - 40.0,
            );
        }
    }

    /// Surah Yusuf at reading sizes. Short samples flatter a face; a long
    /// vocalised passage is where uneven colour, a mark that sits a touch too
    /// high, and spacing that only fails in aggregate become visible.
    fn arabic_long_text(&mut self) {
        let bare = strip_harakat(AR_YUSUF);
        for (size, leading) in [(13.0_f64, 30.0_f64), (9.5, 22.0)] {
            self.new_sheet(
                "Arabic",
                &format!(
                    "Long Text — Surah Yusuf, {}pt — vocalised vs bare",
                    num(size as f32)
                ),
            );
            let fam = self.fam();
            let top = H - 132.0;
            let half = (W - 2.0 * M - GUTTER) / 2.0;

            // The same passage twice, so the two columns can be read against
            // each other line for line: the right one carries its harakat,
            // the left one is stripped. Everything that differs between them
            // is the marks and nothing else — which is what makes a mark
            // sitting too high, or crowding its neighbour, obvious.
            for (ci, (text, label)) in
                [(AR_YUSUF, "vocalised"), (bare.as_str(), "bare")]
                    .iter()
                    .enumerate()
            {
                let x = M + (1 - ci) as f64 * (half + GUTTER);
                self.mono_caption(label, x, top + 14.0);
                self.ctx
                    .no_stroke()
                    .fill(ink())
                    .font(&fam)
                    .clear_font_variations()
                    .font_variation("wght", 400.0)
                    .font_size(size)
                    .tracking(0.0)
                    .line_height(leading)
                    .text_align(TextAlign::Right);
                self.ctx.text_box(text.trim(), x, M, half, top - M);
            }
        }
    }

    /// Shared body-text setup for the diacritic/kerning rows.
    fn set_body(&mut self, fam: &str, size: f64) {
        self.ctx
            .no_stroke()
            .fill(ink())
            .font(fam)
            .clear_font_variations()
            .font_variation("wght", 400.0)
            .font_size(size)
            .tracking(0.0)
            .auto_line_height()
            .text_align(TextAlign::Left);
    }
}

/// Generate the default print proof for `font_path`, writing a PDF to
/// `output_path`. `grid` overlays the Swiss guide grid on every page.
pub fn generate_proof(
    font_path: &Path,
    output_path: &str,
    grid: bool,
) -> Result<(), DesignBotError> {
    let data = std::fs::read(font_path).map_err(DesignBotError::IOError)?;
    let facts = introspect(&data)?;

    let mut r = Renderer::new(W as u32, H as u32);
    r.load_font(font_path)?;
    r.load_font_data(MONO_TTF.to_vec());
    // widest weight sets the fit, so the heaviest waterfall row never overflows
    let wght_tag = u32::from_be_bytes(*b"wght");
    let name_w_100 = r.text_width(&facts.family, Some(&facts.family), 100.0, &[(wght_tag, 700.0)]);

    let mut proof = Proof {
        ctx: Canvas::new(W, H),
        facts: &facts,
        date: today(),
        git: git_hash(font_path),
        folio: 1,
        grid,
        name_w_100,
    };
    proof.cover();
    proof.char_set();
    proof.waterfall();
    proof.text_sizes();
    proof.text_leading();
    proof.text_tracking();
    proof.spacing();
    proof.figures();
    proof.accents();
    proof.kerning();
    proof.weight_waterfall();
    proof.interpolation();

    // Arabic pages only when the font actually covers the script, so a
    // latin-only proof is unchanged.
    if covers_arabic(&facts.cmap) {
        proof.arabic_char_set();
        proof.arabic_joining();
        proof.arabic_repetition();
        proof.arabic_marks();
        proof.arabic_clusters();
        proof.arabic_text();
        proof.arabic_quran();
        proof.arabic_quran_running();
        proof.arabic_long_text();
        proof.arabic_weights();
    }

    r.render_to_pdf(&proof.ctx, output_path)?;
    Ok(())
}
