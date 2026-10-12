//! File and directory names derived from display names.

/// Longest slug the app generates, in bytes.
const MAX_LENGTH: usize = 60;

/// Names Windows refuses as file names, whatever the extension.
const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Stems that mean something else inside a collection directory.
const MARKER_STEMS: &[&str] = &["collection", "folder"];

/// Turns a display name into a file-name stem: lower-case ASCII letters and
/// digits, words joined by `-`.
///
/// `Get pet by ID` gives `get-pet-by-id`. Common accented Latin letters are
/// replaced by their base letter; anything else that is not an ASCII letter
/// or digit separates words. An empty result becomes `untitled`.
pub fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_separator = false;
    for c in name.chars().flat_map(char::to_lowercase) {
        let replacement = transliterate(c);
        let piece: &str = match &replacement {
            Some(text) => text,
            None => "",
        };
        if piece.is_empty() {
            pending_separator = !slug.is_empty();
            continue;
        }
        if slug.len() + piece.len() + usize::from(pending_separator) > MAX_LENGTH {
            break;
        }
        if pending_separator {
            slug.push('-');
            pending_separator = false;
        }
        slug.push_str(piece);
    }
    if slug.is_empty() {
        slug.push_str("untitled");
    }
    if WINDOWS_RESERVED.contains(&slug.as_str()) || MARKER_STEMS.contains(&slug.as_str()) {
        slug.push_str("-item");
    }
    slug
}

/// The ASCII text for one lower-case character, or `None` if it separates words.
fn transliterate(c: char) -> Option<String> {
    if c.is_ascii_lowercase() || c.is_ascii_digit() {
        return Some(c.to_string());
    }
    let text = match c {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
        'ç' | 'ć' | 'č' => "c",
        'ď' | 'đ' => "d",
        'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ė' | 'ę' | 'ě' => "e",
        'ğ' => "g",
        'ì' | 'í' | 'î' | 'ï' | 'ī' | 'į' | 'ı' => "i",
        'ł' | 'ľ' => "l",
        'ñ' | 'ń' | 'ň' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ő' => "o",
        'ř' => "r",
        'ś' | 'š' | 'ş' | 'ș' => "s",
        'ť' | 'ț' => "t",
        'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'ů' | 'ű' | 'ų' => "u",
        'ý' | 'ÿ' => "y",
        'ź' | 'ż' | 'ž' => "z",
        'ß' => "ss",
        'æ' => "ae",
        'œ' => "oe",
        _ => return None,
    };
    Some(text.to_owned())
}

/// Picks a stem based on `slug` that is not in `taken`, adding `-2`, `-3`,
/// ... when needed. `taken` holds the stems already used in the directory,
/// lower-cased, so the result is also unique on case-insensitive filesystems.
pub fn unique_stem(slug: &str, taken: &[String]) -> String {
    if !taken.iter().any(|t| t == slug) {
        return slug.to_owned();
    }
    let mut number = 2usize;
    loop {
        let candidate = format!("{slug}-{number}");
        if !taken.contains(&candidate) {
            return candidate;
        }
        number += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_become_lowercase_words_joined_by_dashes() {
        assert_eq!(slugify("Get pet by ID"), "get-pet-by-id");
        assert_eq!(slugify("  List   pets!! "), "list-pets");
        assert_eq!(slugify("GET /users/:id (v2)"), "get-users-id-v2");
        assert_eq!(slugify("snake_case.and.dots"), "snake-case-and-dots");
    }

    #[test]
    fn accented_letters_keep_their_base_letter() {
        assert_eq!(slugify("Città più vicine"), "citta-piu-vicine");
        assert_eq!(slugify("Straße Æther Œuvre"), "strasse-aether-oeuvre");
        assert_eq!(slugify("ÀÉÎÕÜ"), "aeiou");
    }

    #[test]
    fn names_without_usable_characters_become_untitled() {
        assert_eq!(slugify(""), "untitled");
        assert_eq!(slugify("   "), "untitled");
        assert_eq!(slugify("日本語"), "untitled");
        assert_eq!(slugify("../.."), "untitled");
    }

    #[test]
    fn a_slug_can_never_leave_its_directory() {
        for name in [
            "../../etc/passwd",
            "a/b",
            "a\\b",
            ".hidden",
            "..",
            "C:\\x",
            "a\0b",
        ] {
            let slug = slugify(name);
            assert!(
                slug.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "{name:?} -> {slug:?}"
            );
            assert!(!slug.starts_with('-') && !slug.ends_with('-'), "{slug:?}");
        }
    }

    #[test]
    fn reserved_names_are_avoided() {
        assert_eq!(slugify("CON"), "con-item");
        assert_eq!(slugify("nul"), "nul-item");
        assert_eq!(slugify("Folder"), "folder-item");
        assert_eq!(slugify("collection"), "collection-item");
        assert_eq!(slugify("console"), "console");
    }

    #[test]
    fn long_names_are_cut_at_a_whole_character() {
        let slug = slugify(&"word ".repeat(40));
        assert!(slug.len() <= MAX_LENGTH, "{}", slug.len());
        assert!(!slug.ends_with('-'));
        assert!(slug.starts_with("word-word"));
    }

    #[test]
    fn collisions_get_a_numeric_suffix() {
        let taken = vec!["pets".to_owned(), "pets-2".to_owned(), "other".to_owned()];
        assert_eq!(unique_stem("pets", &taken), "pets-3");
        assert_eq!(unique_stem("new", &taken), "new");
    }
}
