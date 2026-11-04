//! Embedded platform web view (WKWebView on macOS, WebView2 on Windows,
//! WebKitGTK on Linux). GPUI owns chrome; only page content is native.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use wry::dpi::{LogicalPosition, LogicalSize, Position, Size};
use wry::{Rect, WebView, WebViewBuilder};

/// Source denies every permission request/check/device query for the guest
/// partition. Wry's native handler covers media capture, but WebKit still
/// exposes geolocation, notifications, clipboard and device enumeration; this
/// document-start script reports those sensitive APIs as unavailable/denied
/// in every frame before page scripts run.
const PERMISSION_ISOLATION_SCRIPT: &str = r#"
(() => {
  const deny = () => Promise.reject(new DOMException("Permission denied", "NotAllowedError"));
  const fail = (error) => {
    if (typeof error === "function") {
      queueMicrotask(() => error({
        code: 1,
        message: "Permission denied",
        PERMISSION_DENIED: 1,
        POSITION_UNAVAILABLE: 2,
        TIMEOUT: 3
      }));
    }
  };
  try {
    const geolocation = {
      getCurrentPosition(_success, error) { fail(error); },
      watchPosition(_success, error) { fail(error); return 0; },
      clearWatch() {}
    };
    try {
      Object.defineProperty(navigator, "geolocation", { configurable: true, value: geolocation });
    } catch {
      if (navigator.geolocation) {
        navigator.geolocation.getCurrentPosition = geolocation.getCurrentPosition;
        navigator.geolocation.watchPosition = geolocation.watchPosition;
        navigator.geolocation.clearWatch = geolocation.clearWatch;
      }
    }
  } catch {}
  try {
    if (typeof Notification !== "undefined") {
      try {
        Object.defineProperty(Notification, "permission", { configurable: true, get: () => "denied" });
      } catch {}
      Notification.requestPermission = () => Promise.resolve("denied");
    }
  } catch {}
  try {
    if (navigator.mediaDevices) {
      navigator.mediaDevices.getUserMedia = deny;
      navigator.mediaDevices.getDisplayMedia = deny;
      navigator.mediaDevices.enumerateDevices = () => Promise.resolve([]);
    }
  } catch {}
  try {
    if (navigator.clipboard) {
      navigator.clipboard.read = deny;
      navigator.clipboard.readText = deny;
      navigator.clipboard.write = deny;
      navigator.clipboard.writeText = deny;
    }
  } catch {}
  try {
    if (navigator.permissions && navigator.permissions.query) {
      navigator.permissions.query = async () => ({
        state: "denied",
        onchange: null,
        addEventListener() {},
        removeEventListener() {},
        dispatchEvent() { return false; }
      });
    }
  } catch {}
  for (const name of ["usb", "bluetooth", "serial", "hid", "midi", "share", "wakeLock"]) {
    try {
      Object.defineProperty(navigator, name, { configurable: true, value: undefined });
    } catch {}
  }
})();
"#;

/// One nonpersistent browser partition per application profile. Tabs share
/// cookies/storage while alive, and never use WKWebView's default disk store.
#[derive(Default)]
pub struct BrowserProfile {
    #[cfg(target_os = "macos")]
    store: std::cell::RefCell<Option<objc2::rc::Retained<objc2_web_kit::WKWebsiteDataStore>>>,
}

impl BrowserProfile {
    fn configure<'a>(&self, builder: WebViewBuilder<'a>) -> anyhow::Result<WebViewBuilder<'a>> {
        #[cfg(target_os = "macos")]
        {
            use objc2_web_kit::{WKWebViewConfiguration, WKWebsiteDataStore};
            use wry::WebViewBuilderExtMacos;
            let main = objc2::MainThreadMarker::new().ok_or_else(|| {
                anyhow::anyhow!("browser profile must be used on the main thread")
            })?;
            let mut store = self.store.borrow_mut();
            // SAFETY: WebKit's generated configuration API requires unsafe.
            // These calls run only after obtaining a MainThreadMarker, create
            // standard retained WebKit objects, and keep the data store alive
            // for the application partition. Each tab has its own configuration
            // and user-content controller; only the ephemeral store is shared.
            let configuration = unsafe {
                let store =
                    store.get_or_insert_with(|| WKWebsiteDataStore::nonPersistentDataStore(main));
                let configuration = WKWebViewConfiguration::new(main);
                configuration.setWebsiteDataStore(store);
                configuration
            };
            Ok(builder.with_webview_configuration(configuration))
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(builder.with_incognito(true))
        }
    }
}

pub struct BrowserHost {
    web: WebView,
    bounds: Cell<Rect>,
    events: Receiver<BrowserEvent>,
}

#[derive(Clone, Debug)]
pub enum BrowserEvent {
    Title(String),
    Loading { url: String, loading: bool },
    Failed(String),
    External(String),
    Downloaded {
        url: String,
        path: Option<PathBuf>,
        success: bool,
    },
}

impl BrowserHost {
    pub fn new(window: &gpui::Window, url: &str, profile: &BrowserProfile) -> anyhow::Result<Self> {
        let (sender, events) = mpsc::channel();
        let title_sender = sender.clone();
        let external_sender = sender.clone();
        let download_sender = sender.clone();
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let failure_sender = sender.clone();
        let builder = profile
            .configure(WebViewBuilder::new())?
            .with_url(url)
            .with_visible(false)
            .with_focused(false)
            .with_accept_first_mouse(true)
            .with_incognito(true)
            .with_devtools(cfg!(debug_assertions))
            .with_navigation_handler(|url| allowed_url(&url))
            .with_initialization_script_for_main_only(PERMISSION_ISOLATION_SCRIPT, false)
            .with_permission_handler(|_| wry::PermissionResponse::Deny)
            .with_download_started_handler(|url, path| {
                if !download_allowed(&url) {
                    return false;
                }
                let Some(directory) = dirs::download_dir().filter(|path| path.is_absolute()) else {
                    return false;
                };
                let suggested = std::mem::take(path);
                *path = download_destination(&directory, &suggested, &url);
                true
            })
            .with_download_completed_handler(move |url, path, success| {
                let _ = download_sender.send(BrowserEvent::Downloaded { url, path, success });
            })
            .with_new_window_req_handler(move |url, _| {
                if let PopupAction::External(url) = popup_action(&url) {
                    let _ = external_sender.send(BrowserEvent::External(url));
                }
                wry::NewWindowResponse::Deny
            })
            .with_document_title_changed_handler(move |title| {
                let _ = title_sender.send(BrowserEvent::Title(title));
            })
            .with_on_page_load_handler(move |event, url| {
                let _ = sender.send(BrowserEvent::Loading {
                    url,
                    loading: matches!(event, wry::PageLoadEvent::Started),
                });
            });
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let builder = {
            use wry::WebViewBuilderExtDarwin;
            builder.with_on_web_content_process_terminate_handler(move || {
                let _ = failure_sender.send(BrowserEvent::Failed(
                    "The browser process stopped unexpectedly.".into(),
                ));
            })
        };
        let web = builder.build_as_child(window)?;
        Ok(Self {
            web,
            events,
            bounds: Cell::new(Rect {
                position: Position::Logical(LogicalPosition::new(0.0, 0.0)),
                size: Size::Logical(LogicalSize::new(0.0, 0.0)),
            }),
        })
    }

    pub fn take_events(&self) -> Vec<BrowserEvent> {
        self.events.try_iter().collect()
    }

    pub fn stop(&self) {
        let _ = self.web.evaluate_script("window.stop()");
    }

    pub fn url(&self) -> Option<String> {
        self.web.url().ok()
    }

    pub fn set_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        let rect = Rect {
            position: Position::Logical(LogicalPosition::new(x, y)),
            size: Size::Logical(LogicalSize::new(width.max(1.0), height.max(1.0))),
        };
        if self.bounds.get() != rect {
            self.bounds.set(rect);
            let _ = self.web.set_bounds(rect);
        }
    }

    pub fn set_visible(&self, visible: bool) {
        if !visible && self.has_focus() {
            self.focus_parent();
        }
        let _ = self.web.set_visible(visible);
    }

    pub fn focus_parent(&self) {
        let _ = self.web.focus_parent();
    }

    pub fn navigate(&self, url: &str) -> anyhow::Result<()> {
        self.web.load_url(url)?;
        Ok(())
    }

    pub fn go_back(&self) {
        let _ = self.web.go_back();
    }

    pub fn go_forward(&self) {
        let _ = self.web.go_forward();
    }

    pub fn reload(&self) {
        let _ = self.web.reload();
    }

    pub fn can_go_back(&self) -> bool {
        self.web.can_go_back().unwrap_or(false)
    }
    pub fn can_go_forward(&self) -> bool {
        self.web.can_go_forward().unwrap_or(false)
    }

    pub fn set_focus(&self) {
        let _ = self.web.focus();
    }

    /// GPUI retains its logical text focus when a native child takes Cocoa's
    /// first responder. Check the actual responder before handling shortcuts,
    /// otherwise Cmd-V/C/X/Z edit the address field while the page has focus.
    pub fn has_focus(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            use wry::WebViewExtMacOS;
            self.web
                .ns_window()
                .firstResponder()
                .and_then(|responder| responder.downcast::<objc2_app_kit::NSView>().ok())
                .is_some_and(|view| view.isDescendantOf(&self.web.webview()))
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    /// GPUI's Undo/Redo menu items dispatch GPUI actions rather than Cocoa
    /// selectors. Hand those two actions to the native responder's undo manager
    /// while the guest owns keyboard focus.
    pub fn undo(&self, redo: bool) -> bool {
        if !self.has_focus() {
            return false;
        }
        #[cfg(target_os = "macos")]
        {
            use wry::WebViewExtMacOS;
            if let Some(manager) = self
                .web
                .ns_window()
                .firstResponder()
                .and_then(|responder| responder.undoManager())
            {
                if redo {
                    if manager.canRedo() {
                        manager.redo();
                    }
                } else if manager.canUndo() {
                    manager.undo();
                }
            }
            true
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}

/// New-window requests leave the guest only for credential-free HTTPS URLs,
/// matching Source's shared `safeExternalUrl` policy. HTTP, credentials,
/// custom schemes, local files and missing hosts are denied rather than handed
/// to the operating system.
pub fn safe_external_url(value: &str) -> Option<String> {
    let url = url::Url::parse(value).ok()?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str().is_none_or(str::is_empty)
    {
        return None;
    }
    Some(url.to_string())
}

/// The only two outcomes for a guest new-window request: hand a safe external
/// URL to the OS, or deny it inside the guest. Popups are never opened as
/// native windows, matching Source's `setWindowOpenHandler`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PopupAction {
    External(String),
    Deny,
}

pub fn popup_action(value: &str) -> PopupAction {
    safe_external_url(value).map_or(PopupAction::Deny, PopupAction::External)
}

/// Page navigation is limited to credential-free HTTP(S) and `about:blank`.
/// Local files, data/javascript URLs, custom schemes, and URLs carrying user
/// credentials are never admitted by the native guest.
pub fn allowed_url(value: &str) -> bool {
    if value == "about:blank" {
        return true;
    }
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
}

/// Downloads follow the guest navigation policy: credential-free HTTP/HTTPS
/// only. File/data/javascript/custom schemes and blob URLs stay denied, so a
/// page cannot write a file through the browser's download path.
pub fn download_allowed(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(|host| !host.is_empty())
}

/// Human-readable name for a download URL, used when the platform does not
/// report a saved path (macOS reports `None` even on success).
pub fn download_name(url: &str) -> String {
    download_filename(Path::new(""), url)
}

fn download_filename(suggested: &Path, url: &str) -> String {
    suggested
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(safe_download_filename)
        .or_else(|| {
            url::Url::parse(url)
                .ok()
                .and_then(|url| url.path_segments()?.next_back().map(percent_decode))
                .and_then(|name| safe_download_filename(&name))
        })
        .unwrap_or_else(|| "download".into())
}

fn safe_download_filename(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return None;
    }
    let cleaned = name
        .chars()
        .map(|ch| {
            if matches!(ch, '/' | '\\' | '\0') || ch.is_control() {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (
                (bytes[index + 1] as char).to_digit(16),
                (bytes[index + 2] as char).to_digit(16),
            )
        {
            decoded.push((high * 16 + low) as u8);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn download_destination(directory: &Path, suggested: &Path, url: &str) -> PathBuf {
    let filename = download_filename(suggested, url);
    unique_download_path(directory, &filename)
}

fn unique_download_path(directory: &Path, filename: &str) -> PathBuf {
    let candidate = directory.join(filename);
    if !candidate.exists() {
        return candidate;
    }
    let path = Path::new(filename);
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "download".into());
    let extension = path
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    for counter in 1..10_000 {
        let candidate = directory.join(format!("{stem} ({counter}){extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    directory.join(format!("{stem} (download){extension}"))
}

fn explicit_address_scheme(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if ["about:", "file:", "javascript:", "data:"]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return true;
    }
    let Some(index) = value.find("://") else {
        return false;
    };
    let mut scheme = value[..index].chars();
    scheme.next().is_some_and(|ch| ch.is_ascii_alphabetic())
        && scheme.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '.' | '-'))
}

fn host_like_address(value: &str) -> bool {
    let end = value.find(['/', '?', '#']).unwrap_or(value.len());
    let authority = &value[..end];
    let host = match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.bytes().all(|byte| byte.is_ascii_digit()) => {
            host
        }
        Some(_) => return false,
        None => authority,
    };
    let mut parts = host.split('.');
    if parts.next().is_none_or(str::is_empty) {
        return false;
    }
    let mut dots = 0usize;
    for part in parts {
        if part.is_empty() {
            return false;
        }
        dots += 1;
    }
    dots > 0
        && host.split('.').all(|part| {
            part.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        })
}

/// Same address/search semantics as Vela's BrowserPane, with policy checked
/// before creating or navigating a native guest.
pub fn normalize_address(input: &str) -> Option<String> {
    let value = input.trim();
    if value.is_empty() {
        return None;
    }
    let address = if explicit_address_scheme(value) {
        value.to_string()
    } else if value.starts_with("localhost") || host_like_address(value) {
        format!("https://{value}")
    } else {
        let query = value
            .as_bytes()
            .iter()
            .map(|byte| {
                if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(byte) {
                    (*byte as char).to_string()
                } else {
                    format!("%{byte:02X}")
                }
            })
            .collect::<String>();
        format!("https://duckduckgo.com/?q={query}")
    };
    allowed_url(&address).then_some(address)
}

#[cfg(test)]
mod tests {
    use super::{
        PopupAction, allowed_url, download_allowed, download_destination, download_filename,
        popup_action, safe_external_url, unique_download_path,
    };
    use std::path::Path;

    #[test]
    fn address_field_matches_original_search_host_and_policy_behavior() {
        use super::normalize_address;
        assert_eq!(normalize_address("  "), None);
        assert_eq!(
            normalize_address("example.com/path?a=1").as_deref(),
            Some("https://example.com/path?a=1")
        );
        assert_eq!(
            normalize_address("localhost:5173").as_deref(),
            Some("https://localhost:5173")
        );
        assert_eq!(
            normalize_address("example.com:8443/path").as_deref(),
            Some("https://example.com:8443/path")
        );
        assert_eq!(
            normalize_address("foo_bar.example.com?q=1").as_deref(),
            Some("https://foo_bar.example.com?q=1")
        );
        assert_eq!(
            normalize_address("example.com:abc").as_deref(),
            Some("https://duckduckgo.com/?q=example.com%3Aabc"),
            "a non-numeric port is a search, not an address"
        );
        assert_eq!(
            normalize_address("example.com:").as_deref(),
            Some("https://duckduckgo.com/?q=example.com%3A")
        );
        assert_eq!(
            normalize_address("hello http://example.com").as_deref(),
            Some("https://duckduckgo.com/?q=hello%20http%3A%2F%2Fexample.com"),
            "scheme detection is anchored like Source's browserAddress"
        );
        assert_eq!(
            normalize_address("hello 世界").as_deref(),
            Some("https://duckduckgo.com/?q=hello%20%E4%B8%96%E7%95%8C")
        );
        assert_eq!(
            normalize_address("http://127.0.0.1:5173").as_deref(),
            Some("http://127.0.0.1:5173")
        );
        assert_eq!(
            normalize_address("about:blank").as_deref(),
            Some("about:blank")
        );
        assert_eq!(normalize_address("https://user:password@example.com"), None);
        assert_eq!(normalize_address("foo://bar"), None);
        assert_eq!(
            normalize_address("HTTPS://EXAMPLE.COM/Path").as_deref(),
            Some("HTTPS://EXAMPLE.COM/Path"),
            "explicit schemes preserve the input spelling"
        );
        assert_eq!(normalize_address("javascript:alert(1)"), None);
        assert_eq!(normalize_address("file:///etc/passwd"), None);
    }

    #[test]
    fn external_handoff_is_https_only_and_never_carries_credentials() {
        assert_eq!(
            safe_external_url("https://example.com/docs?q=1#top").as_deref(),
            Some("https://example.com/docs?q=1#top")
        );
        assert_eq!(
            safe_external_url("https://example.com:8443/path").as_deref(),
            Some("https://example.com:8443/path")
        );
        assert_eq!(safe_external_url("http://example.com"), None);
        assert_eq!(safe_external_url("https://user:pass@example.com"), None);
        assert_eq!(safe_external_url("https://user@example.com"), None);
        assert_eq!(safe_external_url("javascript:alert(1)"), None);
        assert_eq!(safe_external_url("data:text/html,hi"), None);
        assert_eq!(safe_external_url("file:///etc/passwd"), None);
        assert_eq!(safe_external_url("about:blank"), None);
        assert_eq!(safe_external_url("mailto:hello@example.com"), None);
        assert_eq!(safe_external_url("not a url"), None);
    }

    #[test]
    fn permission_isolation_script_covers_sensitive_web_apis() {
        for needle in [
            "navigator.geolocation",
            "Notification.requestPermission",
            "navigator.mediaDevices.getUserMedia",
            "getDisplayMedia",
            "enumerateDevices",
            "navigator.clipboard.readText",
            "navigator.clipboard.writeText",
            "navigator.permissions.query",
            "\"usb\"",
            "\"bluetooth\"",
            "\"serial\"",
            "\"hid\"",
            "\"midi\"",
            "\"share\"",
            "\"wakeLock\"",
            "NotAllowedError",
        ] {
            assert!(
                super::PERMISSION_ISOLATION_SCRIPT.contains(needle),
                "permission isolation script is missing {needle}"
            );
        }
        assert!(super::PERMISSION_ISOLATION_SCRIPT.contains("state: \"denied\""));
    }

    #[test]
    fn popup_edge_cases_normalize_only_safe_external_https() {
        for (input, expected) in [
            (
                "https://example.com",
                Some("https://example.com/"),
            ),
            (
                "  https://example.com/docs?q=1#top  ",
                Some("https://example.com/docs?q=1#top"),
            ),
            (
                "HTTPS://EXAMPLE.COM/Path",
                Some("https://example.com/Path"),
            ),
            (
                "https://example.com:8443/path",
                Some("https://example.com:8443/path"),
            ),
            (
                "https://[::1]:8443/path",
                Some("https://[::1]:8443/path"),
            ),
            ("https://@example.com/", Some("https://example.com/")),
            ("http://example.com/", None),
            ("http://localhost:5173/", None),
            ("https://user@example.com/", None),
            ("https://user:pass@example.com/", None),
            ("https://:pass@example.com/", None),
            ("javascript:alert(1)", None),
            ("data:text/html,hi", None),
            ("blob:https://example.com/id", None),
            ("file:///tmp/secret", None),
            ("mailto:hello@example.com", None),
            ("about:blank", None),
            ("vela://session/1", None),
            ("//example.com/path", None),
            ("https:///", None),
            ("https://example.com:99999/", None),
            ("", None),
            ("   ", None),
        ] {
            assert_eq!(
                popup_action(input),
                expected.map_or(PopupAction::Deny, |url| PopupAction::External(url.into())),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn navigation_policy_rejects_privileged_and_credentialed_urls() {
        assert!(allowed_url("https://example.com"));
        assert!(allowed_url("http://127.0.0.1:5173/"));
        assert!(allowed_url("about:blank"));
        assert!(!allowed_url("file:///etc/passwd"));
        assert!(!allowed_url("javascript:alert(1)"));
        assert!(!allowed_url("data:text/html,hi"));
        assert!(!allowed_url("https://user:pass@example.com"));
        assert!(!allowed_url("vela://session/1"));
        assert!(!allowed_url("not a url"));
    }

    #[test]
    fn download_policy_accepts_only_credential_free_http_and_https() {
        assert!(download_allowed("https://example.com/files/report.pdf"));
        assert!(download_allowed("http://127.0.0.1:5173/export.csv"));
        assert!(!download_allowed("https://user:pass@example.com/file"));
        assert!(!download_allowed("https://user@example.com/file"));
        assert!(!download_allowed("file:///etc/passwd"));
        assert!(!download_allowed("javascript:alert(1)"));
        assert!(!download_allowed("data:text/plain,hello"));
        assert!(!download_allowed("blob:https://example.com/id"));
        assert!(!download_allowed("about:blank"));
        assert!(!download_allowed("vela://download"));
        assert!(!download_allowed("not a url"));
    }

    #[test]
    fn download_filenames_prefer_the_suggestion_then_the_url_segment() {
        assert_eq!(
            download_filename(
                Path::new("/Users/qa/Downloads/Quarterly Report.pdf"),
                "https://example.com/other.bin"
            ),
            "Quarterly Report.pdf"
        );
        assert_eq!(
            download_filename(
                Path::new(""),
                "https://example.com/files/%E4%B8%96%E7%95%8C.txt?download=1"
            ),
            "世界.txt"
        );
        assert_eq!(
            download_filename(Path::new(""), "https://example.com/files/"),
            "download"
        );
        assert_eq!(
            download_filename(Path::new("../../etc/passwd"), "https://example.com/x"),
            "passwd"
        );
        assert_eq!(
            download_filename(Path::new("."), "https://example.com/%2E%2E"),
            "download"
        );
    }

    #[test]
    fn download_destinations_use_the_os_downloads_directory_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let existing = temp.path().join("report.pdf");
        std::fs::write(&existing, b"existing").unwrap();
        assert_eq!(
            unique_download_path(temp.path(), "report.pdf"),
            temp.path().join("report (1).pdf")
        );
        std::fs::write(temp.path().join("report (1).pdf"), b"existing").unwrap();
        assert_eq!(
            unique_download_path(temp.path(), "report.pdf"),
            temp.path().join("report (2).pdf")
        );
        assert_eq!(
            download_destination(
                temp.path(),
                Path::new("/Users/qa/Downloads/report.pdf"),
                "https://example.com/report.pdf"
            ),
            temp.path().join("report (2).pdf")
        );
        assert_eq!(
            unique_download_path(temp.path(), "README"),
            temp.path().join("README")
        );
    }
}
