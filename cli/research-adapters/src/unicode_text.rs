//! The Unicode data questions are folded and screened with.

use research::question::UnicodeText;
use unicode_normalization::UnicodeNormalization;
use unicode_properties::GeneralCategory;
use unicode_properties::UnicodeGeneralCategory;

/// NFKC folding, and the general categories and default-ignorable code
/// points that render as nothing or as a line break.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnicodeTables;

impl UnicodeText for UnicodeTables {
    fn fold(&self, text: &str) -> String {
        text.nfkc().collect()
    }

    fn is_hidden(&self, character: char) -> bool {
        matches!(
            character.general_category(),
            GeneralCategory::Control
                | GeneralCategory::Format
                | GeneralCategory::PrivateUse
                | GeneralCategory::Unassigned
                | GeneralCategory::LineSeparator
                | GeneralCategory::ParagraphSeparator
        ) || is_default_ignorable(character)
    }
}

fn is_default_ignorable(character: char) -> bool {
    DEFAULT_IGNORABLE_CODE_POINTS
        .iter()
        .any(|&(first, last)| (first..=last).contains(&character))
}

// Neither Unicode crate exposes Default_Ignorable_Code_Point, so its ranges
// are copied from DerivedCoreProperties-17.0.0.txt. Regenerate them whenever
// the crates move to a new Unicode version, which
// the_ignorable_table_matches_the_crates_unicode_version detects.
//
// UNICODE LICENSE V3
//
// COPYRIGHT AND PERMISSION NOTICE
//
// Copyright © 1991-2025 Unicode, Inc.
//
// NOTICE TO USER: Carefully read the following legal agreement. BY
// DOWNLOADING, INSTALLING, COPYING OR OTHERWISE USING DATA FILES, AND/OR
// SOFTWARE, YOU UNEQUIVOCALLY ACCEPT, AND AGREE TO BE BOUND BY, ALL OF THE
// TERMS AND CONDITIONS OF THIS AGREEMENT. IF YOU DO NOT AGREE, DO NOT
// DOWNLOAD, INSTALL, COPY, DISTRIBUTE OR USE THE DATA FILES OR SOFTWARE.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of data files and any associated documentation (the "Data Files") or
// software and any associated documentation (the "Software") to deal in the
// Data Files or Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, and/or sell
// copies of the Data Files or Software, and to permit persons to whom the
// Data Files or Software are furnished to do so, provided that either (a)
// this copyright and permission notice appear with all copies of the Data
// Files or Software, or (b) this copyright and permission notice appear in
// associated Documentation.
//
// THE DATA FILES AND SOFTWARE ARE PROVIDED "AS IS", WITHOUT WARRANTY OF ANY
// KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
// THIRD PARTY RIGHTS.
//
// IN NO EVENT SHALL THE COPYRIGHT HOLDER OR HOLDERS INCLUDED IN THIS NOTICE
// BE LIABLE FOR ANY CLAIM, OR ANY SPECIAL INDIRECT OR CONSEQUENTIAL DAMAGES,
// OR ANY DAMAGES WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS,
// WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION,
// ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THE DATA
// FILES OR SOFTWARE.
//
// Except as contained in this notice, the name of a copyright holder shall
// not be used in advertising or otherwise to promote the sale, use or other
// dealings in these Data Files or Software without prior written
// authorization of the copyright holder.
#[cfg(test)]
const DEFAULT_IGNORABLE_UNICODE_VERSION: (u8, u8, u8) = (17, 0, 0);
const DEFAULT_IGNORABLE_CODE_POINTS: [(char, char); 27] = [
    ('\u{00AD}', '\u{00AD}'),
    ('\u{034F}', '\u{034F}'),
    ('\u{061C}', '\u{061C}'),
    ('\u{115F}', '\u{1160}'),
    ('\u{17B4}', '\u{17B5}'),
    ('\u{180B}', '\u{180D}'),
    ('\u{180E}', '\u{180E}'),
    ('\u{180F}', '\u{180F}'),
    ('\u{200B}', '\u{200F}'),
    ('\u{202A}', '\u{202E}'),
    ('\u{2060}', '\u{2064}'),
    ('\u{2065}', '\u{2065}'),
    ('\u{2066}', '\u{206F}'),
    ('\u{3164}', '\u{3164}'),
    ('\u{FE00}', '\u{FE0F}'),
    ('\u{FEFF}', '\u{FEFF}'),
    ('\u{FFA0}', '\u{FFA0}'),
    ('\u{FFF0}', '\u{FFF8}'),
    ('\u{1BCA0}', '\u{1BCA3}'),
    ('\u{1D173}', '\u{1D17A}'),
    ('\u{E0000}', '\u{E0000}'),
    ('\u{E0001}', '\u{E0001}'),
    ('\u{E0002}', '\u{E001F}'),
    ('\u{E0020}', '\u{E007F}'),
    ('\u{E0080}', '\u{E00FF}'),
    ('\u{E0100}', '\u{E01EF}'),
    ('\u{E01F0}', '\u{E0FFF}'),
];

#[cfg(test)]
mod tests {
    use research::question::plain_question_breach;
    use research::question::PlainQuestionRule;
    use research::question::UnicodeText;

    use super::UnicodeTables;
    use super::DEFAULT_IGNORABLE_UNICODE_VERSION;

    #[test]
    fn unicode_tables_hide_every_refused_class() {
        for hidden in [
            '\u{7}',
            '\u{202E}',
            '\u{E0041}',
            '\u{FE0F}',
            '\u{E0100}',
            '\u{034F}',
            '\u{3164}',
            '\u{2028}',
            '\u{E000}',
            '\u{0378}',
        ] {
            assert!(
                UnicodeTables.is_hidden(hidden),
                "{:04X}",
                u32::from(hidden)
            );
        }
        for shown in ['é', 'ï', 'Ж', 'я', 'a', ' ', '?', '注'] {
            assert!(!UnicodeTables.is_hidden(shown), "{shown}");
        }
    }

    #[test]
    fn unicode_tables_fold_fullwidth_punctuation() {
        assert_eq!(UnicodeTables.fold("\u{FF0E}"), ".");
        assert_eq!(UnicodeTables.fold("\u{FF0F}"), "/");
        assert_eq!(UnicodeTables.fold("\u{FF61}"), "\u{3002}");
    }

    #[test]
    fn the_ignorable_table_matches_the_crates_unicode_version() {
        let (major, minor, update) = DEFAULT_IGNORABLE_UNICODE_VERSION;
        assert_eq!(
            unicode_normalization::UNICODE_VERSION,
            DEFAULT_IGNORABLE_UNICODE_VERSION
        );
        assert_eq!(
            unicode_properties::UNICODE_VERSION,
            (u64::from(major), u64::from(minor), u64::from(update))
        );
    }

    #[test]
    fn the_real_tables_refuse_a_halfwidth_dotted_host() {
        assert_eq!(
            plain_question_breach("see evil\u{FF61}example/q", &UnicodeTables),
            Some(PlainQuestionRule::Link)
        );
    }
}
