//! Constraint checks for the source's HTML credential fields. Error reporting
//! is separate from service failures: invalid fields never submit a request.
use serde_json::Value;
use vela_opencode::providers::{FieldKind, ProviderField};

#[derive(Debug, PartialEq)]
pub(crate) enum Invalid {
    Required,
    Number,
    Minimum(f64),
    Maximum(f64),
    Step,
    TooShort(i64, usize),
    Pattern,
    Unavailable,
}
impl Invalid {
    pub fn message(&self) -> String {
        match self {
            Self::Required => "Please fill out this field.".into(),
            Self::Number => "Please enter a number.".into(),
            Self::Minimum(value) => format!("Value must be greater than or equal to {value}."),
            Self::Maximum(value) => format!("Value must be less than or equal to {value}."),
            Self::Step => "Please enter a valid value.".into(),
            Self::TooShort(minimum, length) => format!(
                "Please lengthen this text to {minimum} characters or more (you are currently using {length} characters)."
            ),
            Self::Pattern => "Please match the requested format.".into(),
            Self::Unavailable => "Pattern validation is unavailable on this platform.".into(),
        }
    }
}
/// One native stepper click. HTML number inputs step by one from the minimum
/// (or default/zero) base; integer controls stay integral. Current text that is
/// not a finite number starts from the base, and results clamp to min/max.
pub(crate) fn stepped_text(field: &ProviderField, current: &str, direction: i8) -> String {
    let integer = field.kind == FieldKind::Integer;
    let base = field
        .minimum
        .or_else(|| field.default.as_ref().and_then(Value::as_f64))
        .unwrap_or(0.);
    let parsed = current
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .unwrap_or(base);
    let mut next = if integer {
        base + ((parsed - base).round() + f64::from(direction))
    } else {
        parsed + f64::from(direction)
    };
    if let Some(minimum) = field.minimum {
        next = next.max(minimum);
    }
    if let Some(maximum) = field.maximum
        && field.minimum.is_none_or(|minimum| minimum <= maximum)
    {
        next = next.min(maximum);
    }
    if integer {
        format!("{}", next.round() as i64)
    } else {
        let rounded = (next * 1_000_000.).round() / 1_000_000.;
        let mut text = format!("{rounded}");
        if text.contains('.') {
            while text.ends_with('0') {
                text.pop();
            }
            if text.ends_with('.') {
                text.pop();
            }
        }
        text
    }
}

pub(crate) fn validate(
    field: &ProviderField,
    value: Option<&Value>,
    text: Option<(&str, bool)>,
) -> Option<Invalid> {
    // The reference doesn't set required/minItems/maxItems on these controls.
    if matches!(
        field.kind,
        FieldKind::External | FieldKind::Boolean | FieldKind::Multiselect
    ) {
        return None;
    }
    let string = text
        .map(|text| text.0)
        .or_else(|| value.and_then(Value::as_str));
    let empty = string == Some("") || value.is_none() || value == Some(&Value::Null);
    if empty {
        return (field.required == Some(true)).then_some(Invalid::Required);
    }
    if field.options.is_some() {
        return None;
    }
    if matches!(field.kind, FieldKind::Integer | FieldKind::Number) {
        let number = value
            .and_then(Value::as_f64)
            .or_else(|| string?.parse::<f64>().ok());
        let Some(number) = number.filter(|number| number.is_finite()) else {
            return Some(Invalid::Number);
        };
        if let Some(minimum) = field.minimum.filter(|minimum| number < *minimum) {
            return Some(Invalid::Minimum(minimum));
        }
        if let Some(maximum) = field.maximum.filter(|maximum| number > *maximum) {
            return Some(Invalid::Maximum(maximum));
        }
        // HTML number's default step is one, even when the field is not labelled
        // integer. The minimum, then initial value, determines the step base.
        let base = field
            .minimum
            .or_else(|| field.default.as_ref().and_then(Value::as_f64))
            .unwrap_or(0.);
        let quotient = number - base;
        let error = (quotient - quotient.round()).abs();
        if error > (f64::EPSILON * number.abs().max(base.abs()).max(1.) * 8.) {
            return Some(Invalid::Step);
        }
        return None;
    }
    let string = string.unwrap_or("");
    let length = string.encode_utf16().count();
    if let Some(minimum) = field.min_length.filter(|minimum| {
        text.is_some_and(|(_, edited)| edited) && *minimum >= 0 && length < *minimum as usize
    }) {
        return Some(Invalid::TooShort(minimum, length));
    }
    if let Some(pattern) = &field.pattern {
        match pattern_matches(pattern, string) {
            Ok(true) => {}
            Ok(false) => return Some(Invalid::Pattern),
            Err(()) => return Some(Invalid::Unavailable),
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn pattern_matches(pattern: &str, text: &str) -> Result<bool, ()> {
    use std::{
        ffi::{CString, c_char, c_void},
        ptr,
    };
    #[link(name = "JavaScriptCore", kind = "framework")]
    unsafe extern "C" {
        fn JSGlobalContextCreate(class: *const c_void) -> *mut c_void;
        fn JSGlobalContextRelease(context: *mut c_void);
        fn JSStringCreateWithUTF8CString(string: *const c_char) -> *mut c_void;
        fn JSStringRelease(string: *mut c_void);
        fn JSEvaluateScript(
            context: *mut c_void,
            script: *mut c_void,
            object: *mut c_void,
            url: *mut c_void,
            line: i32,
            exception: *mut *const c_void,
        ) -> *const c_void;
        fn JSValueToBoolean(context: *mut c_void, value: *const c_void) -> bool;
        fn JSValueIsNull(context: *mut c_void, value: *const c_void) -> bool;
    }
    // Native JavaScriptCore supplies the ECMAScript v regular-expression
    // semantics required by HTML pattern (Unicode sets, backreferences and
    // lookarounds). No DOM, browser view, network or UI code is evaluated.
    // JSON serialization keeps patterns and values data, never executable code.
    let expression = format!(
        "(()=>{{try{{new RegExp('','v')}}catch{{return null}}try{{return new RegExp({},'v').test({})}}catch{{return true}}}})()",
        serde_json::to_string(&format!("^(?:{pattern})$")).expect("string serialization"),
        serde_json::to_string(text).expect("string serialization")
    );
    let expression = CString::new(expression).expect("JSON escapes null characters");
    // SAFETY: These signatures match the public JavaScriptCore C headers. A
    // fresh context owns the evaluated value; strings/context are released in
    // reverse order and no borrowed JS value escapes this call.
    unsafe {
        let context = JSGlobalContextCreate(ptr::null());
        if context.is_null() {
            return Err(());
        }
        let script = JSStringCreateWithUTF8CString(expression.as_ptr());
        let mut exception = ptr::null();
        let value = JSEvaluateScript(
            context,
            script,
            ptr::null_mut(),
            ptr::null_mut(),
            1,
            &mut exception,
        );
        let valid = if exception.is_null() && !value.is_null() && !JSValueIsNull(context, value) {
            Ok(JSValueToBoolean(context, value))
        } else {
            Err(())
        };
        JSStringRelease(script);
        JSGlobalContextRelease(context);
        valid
    }
}
#[cfg(not(target_os = "macos"))]
fn pattern_matches(_: &str, _: &str) -> Result<bool, ()> {
    Err(())
}

#[cfg(test)]
mod tests {
    use super::{Invalid, validate};
    use serde_json::json;
    fn field(value: serde_json::Value) -> vela_opencode::providers::ProviderField {
        serde_json::from_value(value).unwrap()
    }
    #[test]
    fn constraints_use_utf16_browser_step_base_and_do_not_validate_unused_metadata() {
        let number = field(json!({"key":"n","type":"number","minimum":0.5,"maximum":3.5}));
        assert_eq!(validate(&number, Some(&json!(1.5)), None), None);
        assert_eq!(
            validate(&number, Some(&json!(1.0)), None),
            Some(Invalid::Step)
        );
        assert_eq!(
            validate(&number, Some(&json!(0.0)), None),
            Some(Invalid::Minimum(0.5))
        );
        assert_eq!(
            validate(&number, Some(&json!(4.5)), None),
            Some(Invalid::Maximum(3.5))
        );
        assert_eq!(
            validate(&number, Some(&json!("bad")), Some(("bad", true))),
            Some(Invalid::Number)
        );
        let text = field(json!({"key":"t","type":"string","minLength":4}));
        assert_eq!(
            validate(&text, Some(&json!("🌍a")), Some(("🌍a", true))),
            Some(Invalid::TooShort(4, 3))
        );
        assert_eq!(
            validate(&text, Some(&json!("🌍ab")), Some(("🌍ab", true))),
            None
        );
        assert_eq!(validate(&text, Some(&json!("a")), Some(("a", false))), None);
        for kind in ["boolean", "multiselect", "external"] {
            assert_eq!(
                validate(
                    &field(json!({"key":"ignored","type":kind,"required":true,"minItems":9})),
                    None,
                    None
                ),
                None
            );
        }
    }
    #[test]
    fn native_steppers_follow_html_bases_and_clamp_bounds() {
        let integer = field(json!({"key":"i","type":"integer","minimum":2,"maximum":5}));
        assert_eq!(super::stepped_text(&integer, "", 1), "3");
        assert_eq!(super::stepped_text(&integer, "", -1), "2");
        assert_eq!(super::stepped_text(&integer, "4", 1), "5");
        assert_eq!(super::stepped_text(&integer, "5", 1), "5");
        assert_eq!(super::stepped_text(&integer, "2", -1), "2");
        assert_eq!(super::stepped_text(&integer, "nonsense", 1), "3");

        let number = field(json!({"key":"n","type":"number","minimum":0.5,"maximum":3.5}));
        assert_eq!(super::stepped_text(&number, "1.5", 1), "2.5");
        assert_eq!(super::stepped_text(&number, "1.5", -1), "0.5");
        assert_eq!(super::stepped_text(&number, "3.5", 1), "3.5");
        assert_eq!(super::stepped_text(&number, "0.1", 1), "1.1");
        assert_eq!(super::stepped_text(&number, "", 1), "1.5");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn html_patterns_use_unicode_sets_backreferences_and_literal_data() {
        for (pattern, text, expected) in [
            (r"[[a-z]&&[^aeiou]]+", "bcdf", true),
            (r"[[a-z]&&[^aeiou]]+", "be", false),
            (r"(?=.{4}$)([a-z]+)\1", "abab", true),
            (r"(?=.{4}$)([a-z]+)\1", "abac", false),
            (r"\p{Emoji}", "🌍", true),
            ("a", "aa", false),
            ("[", "ignored invalid pattern", true),
            (r".*", "');throw Error('injected')//", true),
        ] {
            assert_eq!(
                super::pattern_matches(pattern, text).unwrap(),
                expected,
                "{pattern} with {text}"
            );
        }
    }
}
