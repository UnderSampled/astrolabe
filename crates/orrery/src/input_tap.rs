//! Passive key observation on Orrery's private Xwayland server. RECORD reports
//! server-delivered events; it does not prove Hype consumed an update.
use anyhow::{Context, Result, ensure};
use std::{
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use x11rb::{
    connection::{Connection, RequestConnection},
    protocol::{
        record::{self, ConnectionExt as _},
        xproto,
    },
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
    x11_utils::TryParse,
};

#[derive(Debug)]
pub struct ObservedKey {
    pub elapsed_micros: u128,
    pub code: u8,
    pub edge: &'static str,
    pub server_time_ms: u32,
    pub modifiers: u16,
}

pub struct InputTap {
    control: RustConnection,
    context: u32,
    receiver: Receiver<Result<ObservedKey>>,
    worker: Option<JoinHandle<Result<()>>>,
}

impl InputTap {
    pub fn start(display: &str, host_display: &str, window: u32, origin: Instant) -> Result<Self> {
        ensure!(
            display.starts_with(':') && display != host_display,
            "refusing host display input tap"
        );
        let (control, _) = x11rb::connect(Some(display))?;
        ensure!(
            control
                .extension_information(record::X11_EXTENSION_NAME)?
                .is_some(),
            "private Xwayland lacks XRecord; passive input coverage unavailable"
        );
        let version = control
            .record_query_version(
                record::X11_XML_VERSION.0.try_into()?,
                record::X11_XML_VERSION.1.try_into()?,
            )?
            .reply()?;
        ensure!(version.major_version >= 1, "unsupported XRecord version");
        let (data, _) = x11rb::connect(Some(display))?;
        let context = control.generate_id()?;
        let none = record::Range8 { first: 0, last: 0 };
        let empty_extension = record::ExtRange {
            major: none,
            minor: record::Range16 { first: 0, last: 0 },
        };
        let range = record::Range {
            core_requests: none,
            core_replies: none,
            ext_requests: empty_extension,
            ext_replies: empty_extension,
            delivered_events: record::Range8 {
                first: xproto::KEY_PRESS_EVENT,
                last: xproto::KEY_RELEASE_EVENT,
            },
            device_events: none,
            errors: none,
            client_started: false,
            client_died: false,
        };
        control
            .record_create_context(context, 0, &[record::CS::ALL_CLIENTS.into()], &[range])?
            .check()?;
        let (sender, receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || -> Result<()> {
            for reply in data.record_enable_context(context)? {
                let reply = reply?;
                if reply.category == 4 {
                    let _ = ready_sender.send(());
                    continue;
                }
                if reply.category != 0 {
                    continue;
                }
                ensure!(!reply.client_swapped, "byte-swapped XRecord client");
                ensure!(
                    reply.data.len().is_multiple_of(32),
                    "unaligned XRecord key event"
                );
                for bytes in reply.data.as_chunks::<32>().0 {
                    let edge = match bytes[0] & 0x7f {
                        xproto::KEY_PRESS_EVENT => "press",
                        xproto::KEY_RELEASE_EVENT => "release",
                        _ => continue,
                    };
                    let (key, _) = xproto::KeyPressEvent::try_parse(bytes)?;
                    if key.event != window {
                        continue;
                    }
                    if sender
                        .send(Ok(ObservedKey {
                            elapsed_micros: origin.elapsed().as_micros(),
                            code: key.detail,
                            edge,
                            server_time_ms: key.time,
                            modifiers: key.state.into(),
                        }))
                        .is_err()
                    {
                        return Ok(());
                    }
                }
            }
            Ok(())
        });
        let tap = Self {
            control,
            context,
            receiver,
            worker: Some(worker),
        };
        ready_receiver
            .recv_timeout(Duration::from_secs(2))
            .context("XRecord did not arm before capture")?;
        Ok(tap)
    }

    pub fn drain(&self) -> Result<Vec<ObservedKey>> {
        let mut keys = Vec::new();
        while let Ok(row) = self.receiver.try_recv() {
            keys.push(row?);
        }
        Ok(keys)
    }

    pub fn stop(mut self) -> Result<Vec<ObservedKey>> {
        self.control.record_disable_context(self.context)?.check()?;
        self.control.sync()?;
        let worker = self.worker.take().context("input tap worker missing")?;
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("input tap thread panicked"))??;
        self.control.record_free_context(self.context)?.check()?;
        self.drain()
    }
}

impl Drop for InputTap {
    fn drop(&mut self) {
        if let Ok(cookie) = self.control.record_disable_context(self.context) {
            let _ = cookie.check();
        }
        if let Ok(cookie) = self.control.record_free_context(self.context) {
            let _ = cookie.check();
        }
    }
}
