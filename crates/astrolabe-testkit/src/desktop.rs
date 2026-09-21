//! Input and screenshots on the owned Gamescope Xwayland display only.
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
        ensure!(
            !display.is_empty() && display != host_display && display.starts_with(':'),
            "refusing input on host/nonlocal display"
        );
        let (connection, screen) = x11rb::connect(Some(&display))?;
        let root = connection.setup().roots[screen].root;
        Ok(Self {
            connection,
            root,
            pid,
            pressed: BTreeSet::new(),
            display,
        })
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
    /// Timed OS input is permitted only for declared menu/save setup. Gameplay
    /// replay must use the verified frame-indexed sampler, never this function.
    pub fn setup_key(
        &mut self,
        key: SetupKey,
        wait: impl Fn(Duration) -> Result<()>,
    ) -> Result<()> {
        self.focus()?;
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
            .filter(|(_, symbols)| symbols.first() == Some(&key.keysym()))
            .map(|(i, _)| setup.min_keycode + i as u8)
            .collect();
        ensure!(
            keys.len() == 1,
            "setup key has no unique primary X11 mapping"
        );
        let code = keys[0];
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
