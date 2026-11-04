//! Bounded duplicate-checks for TypeScript interface properties.
//!
//! Method overloads are legal and skipped. Duplicate property signatures are
//! reported by the shared type-member analysis; an automatic removal is offered
//! only for identical declarations.

use crate::type_members::DuplicateProperty;

#[derive(Clone, Debug)]
pub(super) struct DuplicateInterface {
    pub name: String,
    pub offset: usize,
    pub first_line: usize,
    pub removal: Option<std::ops::Range<usize>>,
}

fn matching(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in text[open..].char_indices() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + index);
                }
            }
            _ => {}
        }
    }
    None
}

fn interface_body(text: &str, at: usize) -> Option<(usize, usize)> {
    let before = text[..at].trim_end();
    let declaration = before.is_empty()
        || before.ends_with("export")
        || before.ends_with("declare")
        || before.ends_with("default");
    if !declaration {
        return None;
    }
    let open = text[at + "interface".len()..].find('{')? + at + "interface".len();
    let close = matching(text, open)?;
    let body = &text[open + 1..close];
    if body.len() > 8 * 1024 || body.matches('\n').count() > 200 {
        return None;
    }
    Some((open + 1, close))
}

fn convert(found: DuplicateProperty) -> DuplicateInterface {
    DuplicateInterface {
        name: found.name,
        offset: found.offset,
        first_line: found.first_line,
        removal: found.removal,
    }
}

/// Duplicate property members within each interface body.
pub(super) fn duplicates(text: &str) -> Vec<DuplicateInterface> {
    let masked = crate::semantic::mask_comments(text);
    let structural = crate::object_keys::mask_strings(&masked);
    let mut found = Vec::new();
    let mut search = 0;
    while let Some(relative) = structural[search..].find("interface") {
        let at = search + relative;
        let before_ok = structural[..at]
            .chars()
            .next_back()
            .is_none_or(|ch| !(ch.is_alphanumeric() || matches!(ch, '_' | '$')));
        let after_ok = structural[at + "interface".len()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace);
        if !before_ok || !after_ok {
            search = at + "interface".len();
            continue;
        }
        let Some((body_start, body_end)) = interface_body(&structural, at) else {
            search = at + "interface".len();
            continue;
        };
        found.extend(
            crate::type_members::duplicates_in(&masked, body_start, body_end)
                .into_iter()
                .map(convert),
        );
        search = at + "interface".len();
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_properties_are_flagged_and_removed() {
        let text = "interface Reader {\n  read: string;\n  write: string;\n  read: string;\n}\n";
        let found = duplicates(text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        assert_eq!(found[0].first_line, 2);
        let removal = found[0].removal.clone().unwrap();
        let mut edited = text.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(
            edited,
            "interface Reader {\n  read: string;\n  write: string;\n}\n"
        );

        let shared = "interface A { read: string; write: string; read: string }\n";
        let removal = duplicates(shared)[0].removal.clone().unwrap();
        let mut edited = shared.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "interface A { read: string; write: string }\n");

        let quoted = "interface A { \"read\": string; read: string }\n";
        assert!(duplicates(quoted)[0].removal.is_some());
    }

    #[test]
    fn differing_types_modifiers_and_property_method_conflicts_have_no_action() {
        let differing = "interface A { read: string; read: number; }\n";
        assert_eq!(duplicates(differing).len(), 1);
        assert!(duplicates(differing)[0].removal.is_none());

        let readonly = "interface A { readonly read: string; read: string; }\n";
        assert_eq!(duplicates(readonly).len(), 1);
        assert!(duplicates(readonly)[0].removal.is_none());

        let optional = "interface A { read?: string; read: string; }\n";
        assert_eq!(duplicates(optional).len(), 1);
        assert!(duplicates(optional)[0].removal.is_none());

        let property_method = "interface A { read: string; read(): void; }\n";
        assert_eq!(duplicates(property_method).len(), 1);
        assert!(duplicates(property_method)[0].removal.is_none());

        let method_property = "interface A { read(): void; read: string; }\n";
        assert_eq!(duplicates(method_property).len(), 1);
        assert!(duplicates(method_property)[0].removal.is_none());
    }

    #[test]
    fn method_overloads_and_distinct_names_are_ignored() {
        assert!(duplicates("interface A { read(): void; read(x: string): void; }\n").is_empty());
        assert!(duplicates("interface A { read: string; write: number; }\n").is_empty());
        assert!(duplicates("interface A { [key: string]: string; }\n").is_empty());
        assert!(duplicates("interface A { new (): A; }\n").is_empty());
        assert!(duplicates("interface A { (): void; }\n").is_empty());
    }

    #[test]
    fn multiline_prefixes_comments_strings_and_other_braces_are_bounded() {
        let exported = "export interface A {\n  read: string;\n  read: string;\n}\n";
        let removal = duplicates(exported)[0].removal.clone().unwrap();
        let mut edited = exported.to_owned();
        edited.replace_range(removal, "");
        assert_eq!(edited, "export interface A {\n  read: string;\n}\n");

        assert!(duplicates("declare interface A { read: string; read: string }\n").len() == 1);
        assert!(duplicates("// interface A { read: string; read: string }\n").is_empty());
        assert!(
            duplicates("const text = \"interface A { read: string; read: string }\";\n").is_empty()
        );
        assert!(duplicates("type A = { read: string; read: string };\n").is_empty());
        assert!(
            duplicates("interface A { read: { x: string }; read: { x: string }; }\n").len() == 1
        );

        let nested = "interface A { outer: { read: string; read: string } }\n";
        let found = duplicates(nested);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "read");
        assert!(found[0].removal.is_some());
    }
}
