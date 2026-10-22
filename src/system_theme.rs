//! The system's light or dark mode, for the "system" theme setting.
//!
//! On Windows and macOS egui gets it from winit. On Linux winit does not know it (X11
//! always says nothing, and Wayland reports the window's own decorations), so it is read
//! from the desktop portal's `org.freedesktop.appearance` `color-scheme` setting, which
//! GNOME, KDE, COSMIC, and others publish, also to Flatpak and snap apps. A background
//! thread reads it and follows its changes, so a slow or missing portal never blocks a
//! frame.
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

const UNKNOWN: u8 = 0;
const DARK: u8 = 1;
const LIGHT: u8 = 2;

pub struct SystemTheme {
    state: Arc<AtomicU8>,
}

impl SystemTheme {
    /// Starts following the system's mode (on Linux, a thread that lives as long as the
    /// app); `ctx` is repainted when it changes.
    pub fn start(ctx: &egui::Context) -> Self {
        let state = Arc::new(AtomicU8::new(UNKNOWN));
        #[cfg(target_os = "linux")]
        {
            let (state, ctx) = (state.clone(), ctx.clone());
            let _ = std::thread::Builder::new()
                .name("system-theme".into())
                .spawn(move || portal::follow(&state, &ctx));
        }
        #[cfg(not(target_os = "linux"))]
        let _ = ctx;
        Self { state }
    }

    /// The system's mode, or `None` when it is not known (yet).
    pub fn get(&self, ctx: &egui::Context) -> Option<egui::Theme> {
        match self.state.load(Ordering::Relaxed) {
            DARK => Some(egui::Theme::Dark),
            LIGHT => Some(egui::Theme::Light),
            _ if cfg!(target_os = "linux") => None,
            _ => ctx.system_theme(),
        }
    }
}

/// The portal's `color-scheme` value: 1 prefers dark, 2 prefers light, 0 has no preference.
#[cfg(any(target_os = "linux", test))]
fn from_color_scheme(value: u32) -> u8 {
    match value {
        1 => DARK,
        2 => LIGHT,
        _ => UNKNOWN,
    }
}

#[cfg(target_os = "linux")]
mod portal {
    use super::{from_color_scheme, AtomicU8, Ordering};
    use futures_lite::StreamExt;
    use zbus::zvariant::{OwnedValue, Value};
    use zbus::{Connection, MatchRule, MessageStream};

    const SERVICE: &str = "org.freedesktop.portal.Desktop";
    const PATH: &str = "/org/freedesktop/portal/desktop";
    const SETTINGS: &str = "org.freedesktop.portal.Settings";
    const NAMESPACE: &str = "org.freedesktop.appearance";
    const KEY: &str = "color-scheme";

    pub fn follow(state: &AtomicU8, ctx: &egui::Context) {
        // Errors (no session bus, no portal) leave the mode unknown, which means light.
        let _ = futures_lite::future::block_on(async {
            let conn = Connection::session().await?;
            // Subscribe first, so a change between the read and the subscription is kept.
            let rule = MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface(SETTINGS)?
                .member("SettingChanged")?
                .path(PATH)?
                .build();
            let mut changes = MessageStream::for_match_rule(rule, &conn, None).await?;
            if let Some(value) = read(&conn).await {
                set(state, ctx, value);
            }
            while let Some(message) = changes.next().await {
                let Ok(message) = message else { continue };
                let body = message.body();
                let Ok((namespace, key, value)) =
                    body.deserialize::<(String, String, OwnedValue)>()
                else {
                    continue;
                };
                if namespace == NAMESPACE && key == KEY {
                    if let Some(value) = number(&value) {
                        set(state, ctx, value);
                    }
                }
            }
            Ok::<(), zbus::Error>(())
        });
    }

    async fn read(conn: &Connection) -> Option<u32> {
        // `ReadOne` (portal version 2) returns the value; older portals only have `Read`,
        // which wraps it in one more variant. `number` unwraps either.
        for method in ["ReadOne", "Read"] {
            let reply = conn
                .call_method(
                    Some(SERVICE),
                    PATH,
                    Some(SETTINGS),
                    method,
                    &(NAMESPACE, KEY),
                )
                .await;
            if let Ok(reply) = reply {
                if let Ok(value) = reply.body().deserialize::<OwnedValue>() {
                    return number(&value);
                }
            }
        }
        None
    }

    fn number(value: &Value) -> Option<u32> {
        match value {
            Value::U32(n) => Some(*n),
            Value::Value(inner) => number(inner),
            _ => None,
        }
    }

    fn set(state: &AtomicU8, ctx: &egui::Context, value: u32) {
        let new = from_color_scheme(value);
        if state.swap(new, Ordering::Relaxed) != new {
            ctx.request_repaint();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_scheme_values() {
        assert_eq!(from_color_scheme(1), DARK);
        assert_eq!(from_color_scheme(2), LIGHT);
        assert_eq!(from_color_scheme(0), UNKNOWN);
        assert_eq!(from_color_scheme(7), UNKNOWN);
    }
}
