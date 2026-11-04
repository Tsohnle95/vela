//! W3C HTML/CSS validation with the reference's size and diagnostic rules.
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub const HTML_VALIDATOR_URL: &str = "https://validator.w3.org/nu/?out=json";
pub const CSS_VALIDATOR_URL: &str = "https://jigsaw.w3.org/css-validator/validator";
pub const MAX_HTML_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_CSS_BYTES: usize = 200 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub message: String,
    pub severity: String,
    pub source: String,
}

fn number(value: Option<&serde_json::Value>, fallback: u32) -> u32 {
    value
        .and_then(serde_json::Value::as_u64)
        .filter(|value| *value > 0 && *value <= u32::MAX as u64)
        .map(|value| value as u32)
        .unwrap_or(fallback)
}

pub fn parse_html(body: &str) -> Vec<Diagnostic> {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(messages) = parsed.get("messages").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    messages
        .iter()
        .filter_map(|message| {
            let text = message.get("message")?.as_str()?;
            let sub_type = message.get("subType").and_then(serde_json::Value::as_str);
            if message.get("type").and_then(serde_json::Value::as_str) == Some("info")
                && sub_type != Some("warning")
            {
                return None;
            }
            let line = number(message.get("firstLine"), 1);
            let column = number(message.get("firstColumn"), 1);
            Some(Diagnostic {
                line,
                column,
                end_line: number(message.get("lastLine"), line),
                end_column: number(message.get("lastColumn"), column + 1),
                message: text.to_owned(),
                severity: if sub_type == Some("warning") {
                    "warning".into()
                } else {
                    "error".into()
                },
                source: "w3c-html".into(),
            })
        })
        .collect()
}

fn decode_xml(value: &str) -> String {
    let mut decoded = value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    for (needle, hex) in [("&#x", true), ("&#", false)] {
        while let Some(start) = decoded.find(needle) {
            let Some(end) = decoded[start..].find(';').map(|index| start + index) else {
                break;
            };
            let digits = &decoded[start + needle.len()..end];
            let value = if hex {
                u32::from_str_radix(digits, 16).ok()
            } else {
                digits.parse::<u32>().ok()
            };
            let Some(character) = value.and_then(char::from_u32) else {
                break;
            };
            decoded.replace_range(start..=end, &character.to_string());
        }
    }
    decoded
}

fn tag_text(xml: &str, name: &str) -> String {
    let open = format!("<{name}");
    let close = format!("</{name}");
    let Some(start) = xml.find(&open) else {
        return String::new();
    };
    let Some(content_start) = xml[start..].find('>').map(|index| start + index + 1) else {
        return String::new();
    };
    let Some(end) = xml[content_start..].find(&close).map(|index| content_start + index) else {
        return String::new();
    };
    decode_xml(
        xml[content_start..end]
            .split('<')
            .next()
            .unwrap_or_default()
            .trim(),
    )
}

fn css_items(body: &str, tag: &str, severity: &str) -> Vec<Diagnostic> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut diagnostics = Vec::new();
    let mut remaining = body;
    while let Some(start) = remaining.find(&open) {
        let Some(content_start) = remaining[start..].find('>').map(|index| start + index + 1) else {
            break;
        };
        let Some(end) = remaining[content_start..].find(&close).map(|index| content_start + index) else {
            break;
        };
        let block = &remaining[content_start..end];
        let line = tag_text(block, "line").parse::<u32>().ok().filter(|line| *line > 0).unwrap_or(1);
        diagnostics.push(Diagnostic {
            line,
            column: 1,
            end_line: line,
            end_column: 2,
            message: {
                let message = tag_text(block, "message");
                if message.is_empty() {
                    let kind = tag_text(block, "errortype");
                    if kind.is_empty() {
                        format!("CSS validation {severity}")
                    } else {
                        kind
                    }
                } else {
                    message
                }
            },
            severity: severity.into(),
            source: "w3c-css".into(),
        });
        remaining = &remaining[end + close.len()..];
    }
    diagnostics
}

pub fn parse_css(body: &str) -> Vec<Diagnostic> {
    if body.to_lowercase().contains("cssvalidationresponse") {
        let mut diagnostics = css_items(body, "error", "error");
        diagnostics.extend(css_items(body, "warning", "warning"));
        if diagnostics.is_empty() && body.to_lowercase().contains("<validity>false") {
            diagnostics.push(Diagnostic {
                line: 1,
                column: 1,
                end_line: 1,
                end_column: 2,
                message: "The CSS validator reported errors but did not return their details."
                    .into(),
                severity: "error".into(),
                source: "w3c-css".into(),
            });
        }
        return diagnostics;
    }
    body.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, ':');
            let _prefix = parts.next()?;
            let line_number = parts.next()?.parse::<u32>().ok()?;
            let rest = parts.next()?.trim();
            if rest.is_empty() {
                return None;
            }
            Some(Diagnostic {
                line: line_number,
                column: 1,
                end_line: line_number,
                end_column: 2,
                message: rest.to_owned(),
                severity: if rest.to_lowercase().contains("warning") {
                    "warning".into()
                } else {
                    "error".into()
                },
                source: "w3c-css".into(),
            })
        })
        .collect()
}

async fn validate_at(
    path: &str,
    content: &str,
    html_url: &str,
    css_url: &str,
) -> Result<Vec<Diagnostic>, String> {
    let lower = path.to_lowercase();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())?;
    if lower.ends_with(".html") || lower.ends_with(".htm") {
        if content.len() > MAX_HTML_BYTES {
            return Err("HTML validation is limited to 4 MiB.".into());
        }
        let response = client
            .post(html_url)
            .header("accept", "application/json, text/plain")
            .header("content-type", "text/html; charset=utf-8")
            .body(content.to_owned())
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("W3C HTML validator returned {}", response.status()));
        }
        return Ok(parse_html(&response.text().await.map_err(|error| error.to_string())?));
    }
    if lower.ends_with(".css") {
        if content.len() > MAX_CSS_BYTES {
            return Err("CSS validation is limited to 200 KiB.".into());
        }
        let body = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("output", "soap12")
            .append_pair("profile", "css3")
            .append_pair("warning", "2")
            .append_pair("lang", "en")
            .append_pair("text", content)
            .finish();
        let response = client
            .post(css_url)
            .header("accept", "application/soap+xml, text/plain")
            .header(
                "content-type",
                "application/x-www-form-urlencoded; charset=utf-8",
            )
            .body(body)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if !response.status().is_success() {
            return Err(format!("W3C CSS validator returned {}", response.status()));
        }
        return Ok(parse_css(&response.text().await.map_err(|error| error.to_string())?));
    }
    Ok(Vec::new())
}

pub async fn validate(
    path: &str,
    content: &str,
    cancel: &CancellationToken,
) -> Result<Vec<Diagnostic>, String> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err("Validation cancelled".into()),
        result = validate_at(path, content, HTML_VALIDATOR_URL, CSS_VALIDATOR_URL) => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::post, Router};
    use serde_json::json;

    #[tokio::test]
    async fn cancelled_validation_returns_without_waiting_for_the_network() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let error = validate("index.html", "<p>x</p>", &cancel)
            .await
            .expect_err("cancelled request");
        assert_eq!(error, "Validation cancelled");
    }

    #[test]
    fn html_and_css_diagnostics_match_reference_filtering() {
        let html = parse_html(
            &json!({"messages":[
                {"type":"error","message":"Bad tag","firstLine":2,"firstColumn":3,"lastLine":2,"lastColumn":9},
                {"type":"info","subType":"warning","message":"Consider this","firstLine":4},
                {"type":"info","message":"ignored"},
                {"type":"error"}
            ]})
            .to_string(),
        );
        assert_eq!(html.len(), 2);
        assert_eq!(html[0].line, 2);
        assert_eq!(html[0].end_column, 9);
        assert_eq!(html[1].severity, "warning");
        assert_eq!(html[1].column, 1);

        let css = parse_css(
            r#"<cssvalidationresponse><result><errors>
            <error><line>3</line><message>Unknown property &amp; value</message></error>
            </errors><warnings><warning><line>5</line><errortype>Deprecated</errortype></warning></warnings></result></cssvalidationresponse>"#,
        );
        assert_eq!(css.len(), 2);
        assert_eq!(css[0].line, 3);
        assert_eq!(css[0].message, "Unknown property & value");
        assert_eq!(css[1].severity, "warning");
        assert_eq!(css[1].line, 5);
        assert_eq!(
            parse_css("file.css:7:3: Bad declaration")[0].line,
            7
        );
    }

    #[tokio::test]
    async fn local_validators_receive_path_appropriate_requests_and_limits() {
        async fn html(body: String) -> String {
            assert!(body.contains("<main>"));
            json!({"messages":[{"type":"error","message":"fixture","firstLine":1}]}).to_string()
        }
        async fn css(body: String) -> String {
            assert!(body.contains("text=body"));
            "<cssvalidationresponse><result><errors><error><line>2</line><message>css fixture</message></error></errors></result></cssvalidationresponse>".into()
        }
        let app = Router::new()
            .route("/html", post(html))
            .route("/css", post(css));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let html = validate_at(
            "index.html",
            "<main>",
            &format!("http://{address}/html"),
            &format!("http://{address}/css"),
        )
        .await
        .unwrap();
        assert_eq!(html[0].message, "fixture");
        let css = validate_at(
            "style.css",
            "body {}",
            &format!("http://{address}/html"),
            &format!("http://{address}/css"),
        )
        .await
        .unwrap();
        assert_eq!(css[0].message, "css fixture");
        assert!(validate_at("x.txt", "x", "http://unused", "http://unused").await.unwrap().is_empty());
    }
}
