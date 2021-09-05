//! # Looprider: a library for recording and playing back event streams
//!
//! *Chikamichi o kidotte*
//! *Ah loop~rider*
//! *Tomawari no kanata*
//! *Hashiru loop~rider*
//!
//! Looprider wraps event streams and allows recording events going through them and playing them
//! back from recorded "replays".

#![warn(missing_docs)]
#![feature(is_sorted)]

use hv_core::prelude::*;
use serde::{Deserialize, Serialize};
use shrev::{Event, EventChannel, EventIterator, ReaderId};

/// Types usable as events with [`Looprider`].
pub trait LoopriderEvent: Event + Clone {}

/// A replay is an ordered list of events to be played back by a [`Looprider`] in playback mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay<E: LoopriderEvent> {
    records: Vec<Record<E>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record<E: LoopriderEvent> {
    frame: u64,
    events: Vec<E>,
}

/// Represents a subscription to a [`Looprider`]'s event stream.
#[derive(Debug)]
pub struct LoopreaderId<E: LoopriderEvent>(ReaderId<E>);

#[derive(Debug)]
enum LoopriderMode<E: LoopriderEvent> {
    Playback,
    Record { buf: Vec<E> },
}

/// A [`Looprider`] is a single-producer multi-consumer event channel based on the `shrev` crate
/// which operates in either "record" or "playback" mode. In both modes, events are only pushed to
/// readers after a call to [`Looprider::tick`]. "Record" mode buffers events added to the event
/// channel with the [`Looprider::push`] method, and then writes them all to the event channel on a
/// call to `tick` while recording all events buffered that frame to a single "frame record".
/// "Playback" mode ignores pushed events, and instead only pushes events coming from a previously
/// recorded [`Replay`].
#[derive(Debug)]
pub struct Looprider<E: LoopriderEvent> {
    channel: EventChannel<E>,
    mode: LoopriderMode<E>,
    records: Vec<Record<E>>,
    frame: u64,
}

impl<E: LoopriderEvent> Looprider<E> {
    /// Construct a new [`Looprider`] in "record" mode.
    pub fn record() -> Shared<Self> {
        Shared::new(Self {
            channel: EventChannel::new(),
            mode: LoopriderMode::Record { buf: Vec::new() },
            records: Vec::new(),
            frame: 0,
        })
    }

    /// Construct a new [`Looprider`] in "playback" mode.
    pub fn playback(replay: Replay<E>) -> Shared<Self> {
        assert!(
            replay
                .records
                .is_sorted_by(|r1, r2| Some(r1.frame.cmp(&r2.frame).reverse())),
            "invalid replay data (out of order)"
        );

        Shared::new(Self {
            channel: EventChannel::new(),
            mode: LoopriderMode::Playback,
            records: replay.records,
            frame: 0,
        })
    }

    /// Convert this [`Looprider`] and all its buffered events to a [`Replay`] for playback and/or
    /// serialization.
    pub fn to_replay(&self) -> Option<Replay<E>> {
        match self.mode {
            LoopriderMode::Playback => None,
            LoopriderMode::Record { .. } => Some(Replay {
                records: self.records.iter().cloned().rev().collect(),
            }),
        }
    }

    /// Update the [`Looprider`] by flushing its internal buffers and incrementing its frame
    /// counter. Should be called once per frame, and assumes a constant delta-time for most
    /// underlying processes - otherwise replays will quickly and easily desynchronize.
    pub fn tick(&mut self) {
        match &mut self.mode {
            LoopriderMode::Playback => {
                while matches!(self.records.last(), Some(record) if record.frame <= self.frame) {
                    let record = self.records.pop().unwrap();
                    assert_eq!(
                        record.frame, self.frame,
                        "a looprider tick was skipped! replay frame mismatch"
                    );
                    self.channel.iter_write(record.events);
                }
            }
            LoopriderMode::Record { buf } => {
                if !buf.is_empty() {
                    self.records.push(Record {
                        frame: self.frame,
                        events: buf.clone(),
                    });

                    self.channel.drain_vec_write(buf);
                }
            }
        }

        self.frame += 1;
    }

    /// Create a subscription handle to the event stream.
    pub fn register_reader(&mut self) -> LoopreaderId<E> {
        LoopreaderId(self.channel.register_reader())
    }

    /// Iterate over all the most recent events.
    pub fn read(&self, reader_id: &mut LoopreaderId<E>) -> EventIterator<E> {
        self.channel.read(&mut reader_id.0)
    }

    /// Push a new event to the stream.
    pub fn push(&mut self, event: E) {
        match &mut self.mode {
            LoopriderMode::Playback => {
                log::warn!("looprider is in playback mode; event is being discarded");
                drop(event);
            }
            LoopriderMode::Record { buf } => buf.push(event),
        }
    }
}

impl<E> LuaUserData for LoopreaderId<E> where
    E: LoopriderEvent + for<'lua> FromLua<'lua> + for<'lua> ToLua<'lua>
{
}

impl<E> LuaUserData for Looprider<E>
where
    E: LoopriderEvent + for<'lua> FromLua<'lua> + for<'lua> ToLua<'lua>,
{
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("tick", |_, this, ()| {
            this.tick();
            Ok(())
        });

        methods.add_method_mut("register_reader", |_, this, ()| Ok(this.register_reader()));

        methods.add_method("read", |_, this, reader: LuaAnyUserData| {
            let mut reader = reader.borrow_mut::<LoopreaderId<E>>()?;
            Ok(this.read(&mut reader).cloned().collect::<Vec<_>>())
        });

        methods.add_method_mut("push", |_, this, event| {
            this.push(event);
            Ok(())
        });
    }
}
