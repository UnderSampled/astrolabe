//! Input and screenshots on the owned Gamescope Xwayland display only.
use crate::replay::{Edge, Event, Key};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    fs::File,
    path::Path,
    time::{Duration, Instant},
};
use x11rb::{
    CURRENT_TIME,
    connection::Connection,
    protocol::{xproto::*, xtest::ConnectionExt as _},
    rust_connection::RustConnection,
};

#[derive(Debug, Clone, Copy, Serialize)]
pub enum SetupKey {
    Down,
    Return,
}
impl SetupKey {
    fn keysym(self) -> u32 {
        match self {
            Self::Down => 0xff54,
            Self::Return => 0xff0d,
        }
    }
}

fn physical_keysym(key: Key) -> u32 {
    match key {
        Key::ArrowLeft => 0xff51,
        Key::ArrowUp => 0xff52,
        Key::ArrowRight => 0xff53,
        Key::ArrowDown => 0xff54,
        Key::LeftShift => 0xffe1,
    }
}

pub struct Desktop {
    connection: RustConnection,
    root: Window,
    pid: u32,
    pressed: BTreeSet<u8>,
    pub display: String,
}
#[derive(Debug, Serialize)]
pub struct ScreenCapture {
    pub path: String,
    pub width: u16,
    pub height: u16,
    pub window: u32,
    pub request_start_micros: u128,
    pub reply_end_micros: u128,
    pub association: &'static str,
}
impl Desktop {
    /// Called by the controller worker inside Gamescope, whose environment is
    /// supplied at exec. No global DISPLAY/XAUTHORITY mutation or host input.
    pub fn connect(host_display: &str, pid: u32) -> Result<Self> {
        let display = std::env::var("DISPLAY").context("Gamescope did not provide DISPLAY")?;
        Self::connect_display(&display, host_display, pid)
    }
    pub fn connect_display(display: &str, host_display: &str, pid: u32) -> Result<Self> {
        ensure!(
            !display.is_empty() && display != host_display && display.starts_with(':'),
            "refusing input on host/nonlocal display"
        );
        let (connection, screen) = x11rb::connect(Some(display))?;
        let root = connection.setup().roots[screen].root;
        Ok(Self {
            connection,
            root,
            pid,
            pressed: BTreeSet::new(),
            display: display.to_owned(),
        })
    }

    fn named_keysym(name: &str) -> Option<u32> {
        match name {
            "Left" => Some(0xff51),
            "Up" => Some(0xff52),
            "Right" => Some(0xff53),
            "Down" => Some(0xff54),
            "Return" => Some(0xff0d),
            "Escape" => Some(0xff1b),
            "space" => Some(0x20),
            "Tab" => Some(0xff09),
            "Control_L" => Some(0xffe3),
            "Shift_L" => Some(0xffe1),
            value if value.len() == 1 && value.as_bytes()[0].is_ascii_alphanumeric() => {
                Some(value.as_bytes()[0].to_ascii_lowercase().into())
            }
            _ => None,
        }
    }

    pub fn validate_chord(chord: &str) -> Result<Vec<u32>> {
        let names = chord.split('+').collect::<Vec<_>>();
        ensure!(
            !names.is_empty() && names.len() <= 8,
            "empty or oversized key chord"
        );
        names
            .into_iter()
            .map(|name| {
                Self::named_keysym(name)
                    .with_context(|| format!("unsupported X11 key name: {name}"))
            })
            .collect()
    }

    /// Timed interactive input for the MCP controller. Every chord is released
    /// before its gap, and Drop remains a final release guard on failure.
    pub fn timed_keys(
        &mut self,
        chords: &[String],
        hold: Duration,
        gap: Duration,
        wait: impl Fn(Duration) -> Result<()>,
    ) -> Result<()> {
        ensure!(
            (20..=10_000).contains(&hold.as_millis()),
            "hold outside 20..10000 ms"
        );
        ensure!(gap.as_millis() <= 2_000, "gap outside 0..2000 ms");
        ensure!(chords.len() <= 256, "too many key chords");
        ensure!(
            chords.len() as u128 * (hold.as_millis() + gap.as_millis()) <= 25_000,
            "key action exceeds 25 seconds"
        );
        let validated = chords
            .iter()
            .map(|c| Self::validate_chord(c))
            .collect::<Result<Vec<_>>>()?;
        self.focus()?;
        wait(Duration::from_millis(200))?;
        let result = (|| -> Result<()> {
            for symbols in validated {
                ensure!(
                    self.connection.get_input_focus()?.reply()?.focus == self.window()?,
                    "game lost keyboard focus"
                );
                for symbol in symbols {
                    let code = self.keycode(symbol)?;
                    ensure!(self.pressed.insert(code), "duplicate physical key in chord");
                    self.connection
                        .xtest_fake_input(KEY_PRESS_EVENT, code, CURRENT_TIME, self.root, 0, 0, 0)?
                        .check()?;
                }
                self.connection.flush()?;
                wait(hold)?;
                self.release()?;
                wait(gap)?;
            }
            Ok(())
        })();
        let released = self.release();
        result?;
        released
    }
    fn atom(&self, name: &[u8]) -> Result<Atom> {
        Ok(self.connection.intern_atom(false, name)?.reply()?.atom)
    }
    fn property(&self, window: Window, atom: Atom) -> Result<GetPropertyReply> {
        Ok(self
            .connection
            .get_property(false, window, atom, AtomEnum::ANY, 0, 1024)?
            .reply()?)
    }
    fn window(&self) -> Result<Window> {
        let pid_atom = self.atom(b"_NET_WM_PID")?;
        let mut queue = vec![(self.root, 0)];
        let mut matches = vec![];
        while let Some((window, depth)) = queue.pop() {
            if depth > 4 {
                continue;
            }
            let row = (|| -> Result<()> {
                let pid = self
                    .property(window, pid_atom)?
                    .value32()
                    .and_then(|mut v| v.next());
                let class = self.property(window, AtomEnum::WM_CLASS.into())?.value;
                let geometry = self.connection.get_geometry(window)?.reply()?;
                if pid == Some(self.pid)
                    && class
                        .split(|b| *b == 0)
                        .any(|c| c.eq_ignore_ascii_case(b"maidfxvr_bleu.exe"))
                    && geometry.width > 100
                    && geometry.height > 100
                    && self
                        .connection
                        .get_window_attributes(window)?
                        .reply()?
                        .map_state
                        == MapState::VIEWABLE
                {
                    matches.push((
                        u32::from(geometry.width) * u32::from(geometry.height),
                        window,
                    ));
                }
                for child in self.connection.query_tree(window)?.reply()?.children {
                    queue.push((child, depth + 1));
                }
                Ok(())
            })();
            if window == self.root {
                row?;
            }
        }
        matches
            .into_iter()
            .max()
            .map(|(_, w)| w)
            .context("no visible window belonging to the owned Hype process")
    }
    pub fn focus(&self) -> Result<()> {
        let window = self.window()?;
        let event = ClientMessageEvent::new(
            32,
            window,
            self.atom(b"_NET_ACTIVE_WINDOW")?,
            [2, CURRENT_TIME, 0, 0, 0],
        );
        self.connection
            .send_event(
                false,
                self.root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                event,
            )?
            .check()?;
        self.connection
            .configure_window(
                window,
                &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
            )?
            .check()?;
        self.connection
            .set_input_focus(InputFocus::PARENT, window, CURRENT_TIME)?
            .check()?;
        ensure!(
            self.connection.get_input_focus()?.reply()?.focus == window,
            "owned game did not receive private focus"
        );
        Ok(())
    }

    fn keycode(&self, keysym: u32) -> Result<u8> {
        let setup = self.connection.setup();
        let count = setup
            .max_keycode
            .checked_sub(setup.min_keycode)
            .context("invalid keyboard range")?
            .checked_add(1)
            .context("keyboard range overflow")?;
        let mapping = self
            .connection
            .get_keyboard_mapping(setup.min_keycode, count)?
            .reply()?;
        ensure!(mapping.keysyms_per_keycode > 0, "empty keyboard mapping");
        let keys: Vec<_> = mapping
            .keysyms
            .chunks(mapping.keysyms_per_keycode as usize)
            .enumerate()
            .filter(|(_, symbols)| symbols.contains(&keysym))
            .map(|(i, _)| setup.min_keycode + i as u8)
            .collect();
        ensure!(
            keys.len() == 1,
            "physical key has no unique primary X11 mapping"
        );
        Ok(keys[0])
    }

    /// Deliver literal physical key edges through XTest on the private display.
    /// Wine and Hype retain focus handling, translation, callback order, input
    /// history, bindings and consumption. This never writes game memory.
    pub fn physical_events(&mut self, events: &[Event]) -> Result<()> {
        let window = self.window()?;
        ensure!(
            self.connection.get_input_focus()?.reply()?.focus == window,
            "game lost keyboard focus before physical input"
        );
        for event in events {
            ensure!(event.keyboard == 0, "unsupported physical keyboard");
            let code = self.keycode(physical_keysym(event.key))?;
            match event.edge {
                Edge::Press => {
                    ensure!(self.pressed.insert(code), "physical key is already held");
                    self.connection
                        .xtest_fake_input(KEY_PRESS_EVENT, code, CURRENT_TIME, self.root, 0, 0, 0)?
                        .check()?;
                }
                Edge::Release => {
                    ensure!(self.pressed.remove(&code), "physical key is not held");
                    self.connection
                        .xtest_fake_input(
                            KEY_RELEASE_EVENT,
                            code,
                            CURRENT_TIME,
                            self.root,
                            0,
                            0,
                            0,
                        )?
                        .check()?;
                }
            }
        }
        self.connection.flush()?;
        Ok(())
    }
    /// Timed OS input is permitted only for declared menu/save setup. Gameplay
    /// replay must use the verified frame-indexed sampler, never this function.
    pub fn setup_key(
        &mut self,
        key: SetupKey,
        wait: impl Fn(Duration) -> Result<()>,
    ) -> Result<()> {
        self.focus()?;
        let code = self.keycode(key.keysym())?;
        self.pressed.insert(code);
        let held = (|| -> Result<()> {
            self.connection
                .xtest_fake_input(KEY_PRESS_EVENT, code, CURRENT_TIME, self.root, 0, 0, 0)?
                .check()?;
            wait(Duration::from_millis(55))
        })();
        let released = self.release();
        held?;
        released?;
        wait(Duration::from_millis(800))
    }
    pub fn release(&mut self) -> Result<()> {
        for code in self.pressed.clone() {
            self.connection
                .xtest_fake_input(KEY_RELEASE_EVENT, code, CURRENT_TIME, self.root, 0, 0, 0)?
                .check()?;
            self.pressed.remove(&code);
        }
        self.connection.flush()?;
        Ok(())
    }
    pub fn screenshot(&self, path: &Path, origin: Instant) -> Result<ScreenCapture> {
        let window = self.window()?;
        let geometry = self.connection.get_geometry(window)?.reply()?;
        let start = origin.elapsed().as_micros();
        let image = self
            .connection
            .get_image(
                ImageFormat::Z_PIXMAP,
                window,
                0,
                0,
                geometry.width,
                geometry.height,
                u32::MAX,
            )?
            .reply()?;
        let end = origin.elapsed().as_micros();
        let setup = self.connection.setup();
        let visual = setup
            .roots
            .iter()
            .flat_map(|r| &r.allowed_depths)
            .flat_map(|d| &d.visuals)
            .find(|v| v.visual_id == image.visual)
            .context("unknown screenshot visual")?;
        ensure!(
            setup.image_byte_order == ImageOrder::LSB_FIRST
                && matches!(image.depth, 24 | 32)
                && visual.red_mask == 0xff0000
                && visual.green_mask == 0xff00
                && visual.blue_mask == 0xff,
            "unverified screenshot pixel format"
        );
        ensure!(
            image.data.len() == geometry.width as usize * geometry.height as usize * 4,
            "screenshot size/stride differs"
        );
        let rgb: Vec<u8> = image
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|p| [p[2], p[1], p[0]])
            .collect();
        let file = File::create_new(path)?;
        let mut encoder = png::Encoder::new(file, geometry.width.into(), geometry.height.into());
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&rgb)?;
        Ok(ScreenCapture {
            path: path.display().to_string(),
            width: geometry.width,
            height: geometry.height,
            window,
            request_start_micros: start,
            reply_end_micros: end,
            association: "independent capture; no completed-update render association",
        })
    }
}
impl Drop for Desktop {
    fn drop(&mut self) {
        let _ = self.release();
    }
}
