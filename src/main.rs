/*!
 * Keyboard Bridge for Raspberry Pi - Main
 * Created by sheepy0125 on 2023-07-22 under the MIT license
**/

/***** Setup *****/
use anyhow::{Context, Result};
use chrono::Local;
use env_logger::Builder;
use evdev::{Device, EventStream, EventType, InputEvent};
use log::{info, trace, warn};
use std::{cell::Cell, fs::OpenOptions, io::Write, os::unix::prelude::OpenOptionsExt};
pub mod key;
use key::*;
pub mod chord;
use chord::*;
// Config constants
const KEYBOARD_DEVICE_PATH: &str = "/dev/input/event5";
const USB_GADGET_DEVICE_PATH: &str = "/dev/hidg0";
// Constants
const NO_BLOCK: i32 = 2048_i32;
const MAX_ATTEMPTS: usize = 256_usize;

/***** Enums *****/

/***** Structs *****/
/// USB key event
struct USBKeyEvent<'b> {
    modifiers: &'b [ModifierKey],
    keys: &'b [RegularKey],
}
impl<'b> USBKeyEvent<'b> {
    pub fn to_report(&self) -> [u8; 8] {
        // [mod, <empty>, key 1, key n..., key 6]
        let mut report = [0_u8; 8];

        // Modifier keys
        for modifier_key in self.modifiers {
            report[0] |= *modifier_key as u8;
        }

        // Regular keys
        for (idx, key) in self.keys.iter().enumerate() {
            if idx >= 6 {
                warn!("6 keys pressed at once, some are getting dropped!");
                break;
            }
            report[2 + idx] = *key as u8;
        }

        trace!("USB report: {report:?}");
        report
    }
}

/// Keyboard handler
struct Keyboard<'a> {
    event_stream: EventStream,
    keys: Vec<RegularKey>,
    modifiers: Vec<ModifierKey>,
    /// Sentinel value is KeyCode::Unknown
    chord_buffer: Cell<KeyCode>,
    chord_length: u8,
    possible_chords: Vec<&'a [KeyCode]>,
}
impl<'a> Keyboard<'a> {
    pub fn new(device_path: &str) -> Result<Self> {
        let mut device = Device::open(device_path).context("Open device path")?;
        device.grab().context("Grab device")?; // We are the only listener to the device events.
        let event_stream = device.into_event_stream().context("Get event stream")?;
        Ok(Self {
            event_stream,
            keys: Vec::new(),
            modifiers: Vec::new(),
            possible_chords: Vec::new(),
            chord_length: 0_u8,
            chord_buffer: Cell::new(KeyCode::Unknown),
        })
    }

    /// Process key events and update the vecs holding what keys are pressed
    pub fn process_key_events(&mut self, event: InputEvent, key_code: KeyCode) {
        let key_event_enum_variant = event.value().try_into().unwrap_or(Release as u8);
        use KeyEvent::*;
        match key_event_enum_variant {
            // Released key
            _r if _r == Release as u8 => {
                // Remove key from vecs
                if let KeyCode::Regular(released_key) = key_code {
                    if let Some(idx) = self.keys.iter().position(|k| k == &released_key) {
                        self.keys.remove(idx);
                    }
                }
                if let KeyCode::Modifier(released_key) = key_code {
                    if let Some(idx) = self.modifiers.iter().position(|k| k == &released_key) {
                        self.modifiers.remove(idx);
                    }
                }
                // Remove key from chord buffer
                self.chord_buffer.set(KeyCode::Unknown);
            }
            // Pressed key
            _p if _p == Press as u8 => {
                // Push key to vecs
                if let KeyCode::Regular(pressed_key) = key_code {
                    self.keys.push(pressed_key)
                }
                if let KeyCode::Modifier(pressed_key) = key_code {
                    self.modifiers.push(pressed_key)
                }
                // Update chord buffer
                self.chord_buffer.set(key_code);
            }
            // Repeated key
            _h if _h == Repeat as u8 => {
                // Assume the press event already pushed the key into the vec
            }
            _ => unreachable!(),
        }
    }

    /// Process any chords, doing the desired action
    pub fn process_chords(&mut self) {
        use KeyCode::*;
        use ModifierKey::*;

        // Listen for a chord
        let chord_buffer = self.chord_buffer.get_mut();
        if chord_buffer == &CHORD_SEQUENCE_START_KEY {
            trace!("Chord sequence start key received. Listening for chords.");
            self.possible_chords = ALL_CHORDS.to_vec();
            self.chord_length = 1;
            return;
        }

        if self.chord_length == 0 || chord_buffer == &mut Unknown {
            return;
        }

        // Handle special chord keys
        if let Some(replaced_modifier) = match chord_buffer {
            Modifier(LeftCtrl) => Some(Modifier(EitherCtrl)),
            Modifier(LeftShift) => Some(Modifier(EitherShift)),
            Modifier(LeftAlt) => Some(Modifier(EitherAlt)),
            Modifier(LeftSuper) => Some(Modifier(EitherSuper)),
            Modifier(RightCtrl) => Some(Modifier(EitherCtrl)),
            Modifier(RightShift) => Some(Modifier(EitherShift)),
            Modifier(RightAlt) => Some(Modifier(EitherAlt)),
            Modifier(RightSuper) => Some(Modifier(EitherSuper)),
            _ => None,
        } {
            trace!("Chord modifier swapped with {replaced_modifier:?}");
            *chord_buffer = replaced_modifier;
        };

        // Iterate through possible chords
        self.possible_chords.retain(|chord| {
            // Chords do not have CHORD_SEQUENCE_START_KEY as their first element,
            // but it still is counted in self.chord_length
            if let Some(next_key_of_this_chord) = chord.get(self.chord_length as usize - 1) {
                if *chord_buffer == *next_key_of_this_chord {
                    trace!("Positive match ({next_key_of_this_chord:?}) for {chord:?}");
                    return true;
                }
                trace!(
                    "Negative match ({:?} vs. {next_key_of_this_chord:?}) for {chord:?}",
                    *chord_buffer
                );
                return false;
            }
            trace!("Out of range for {chord:?}");
            false
        });
        self.chord_length += 1;

        // Check if we have concluded a chord. Assume all chords diverge at some point.
        if self.possible_chords.is_empty() {
            self.chord_length = 0;
        }
        if self.possible_chords.len() != 1 {
            return;
        }
        let chord = &self.possible_chords[0];
        if chord.len() as u8 != self.chord_length {
            return;
        }

        // See chord.rs
        self.handle_chord(chord);
    }

    /// Block to read events from the keyboard, process them, and then return a
    /// USB key event.
    pub async fn read_process(&mut self) -> Result<USBKeyEvent> {
        // Read key events
        let mut event;
        loop {
            event = self
                .event_stream
                .next_event()
                .await
                .context("Fetch next event of keyboard event stream")?;
            if event.event_type() == EventType::KEY {
                break;
            } else if event.event_type() != EventType::SYNCHRONIZATION {
                trace!("Skipped event type {:?} (not sync).", event.event_type());
            }
        }
        let key_code = event.into();

        // Process
        self.process_key_events(event, key_code);
        self.process_chords();

        trace!("Keys pressed: {:?}", self.keys);
        trace!("Modifiers pressed: {:?}", self.modifiers);

        // Send the USB key event
        Ok(USBKeyEvent {
            keys: &self.keys,
            modifiers: &self.modifiers,
        })
    }
}

/***** Auxiliary functions *****/

/// Convert a chord sequence to a readable String
fn chord_sequence_to_string(chord_sequence: &ChordSequence) -> String {
    let mut ret = "Enter".to_string();
    for key in chord_sequence {
        ret.push_str(&match key {
            KeyCode::Modifier(modifier_key) => format!(", {modifier_key:?}"),
            KeyCode::Regular(regular_key) => format!(", {regular_key:?}"),
            KeyCode::Unknown => ", UNKNOWN".into(),
        });
    }
    ret
}

/***** Main *****/

#[tokio::main]
async fn main() -> Result<()> {
    // Setup logger
    Builder::new()
        .parse_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    println!(
        "USB Keyboard Bridge. To exit, type: {}",
        chord_sequence_to_string(QUIT_CHORD_SEQUENCE)
    );

    // Setup keyboard
    let mut keyboard = Keyboard::new(KEYBOARD_DEVICE_PATH)
        .with_context(|| format!("Create keyboard at {KEYBOARD_DEVICE_PATH}"))?;
    info!("Registered keyboard device.");
    // Setup USB
    let mut usb_gadget = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(NO_BLOCK)
        .open(USB_GADGET_DEVICE_PATH)
        .with_context(|| format!("Open USB gadget file at {USB_GADGET_DEVICE_PATH}"))?;
    info!("Connected to USB gadget OTG device.");
    let mut attempt;
    loop {
        // Get USB report. The only time this should be okay to fail is when
        // a keyboard is unplugged.
        // TODO: Allow hot-swappable keyboards
        let usb_key_event = keyboard
            .read_process()
            .await
            .context("Reading and processing USB event from keyboard")?;
        let usb_report = usb_key_event.to_report();
        // Write in MAX_ATTEMPTS attempts. It appears that for whatever reason sometimes
        // writing *always* fails with OS error 9, but doing it some arbitrary number of
        // times (even if all those "fail") will have the characters sent out correctly.
        // FIXME: This is pretty broken.
        attempt = 0_usize;
        loop {
            attempt += 1;
            trace!("Writing USB report, attempt {attempt}");
            if usb_gadget
                .write_all(&usb_report)
                .map_err(|e| {
                    warn!("Writing USB report {usb_report:?} on attempt {attempt} failed: {e}")
                })
                .is_ok()
            {
                break;
            }
            if attempt >= MAX_ATTEMPTS {
                warn!("Failed to write USB report {MAX_ATTEMPTS} times.");
            }
        }
        attempt = 0;
        loop {
            if usb_gadget
                .flush()
                .map_err(|e| warn!("Flushing USB gadget on attempt {attempt} failed: {e}"))
                .is_ok()
            {
                break;
            }
            if attempt >= MAX_ATTEMPTS {
                warn!("Failed to flush USB report {MAX_ATTEMPTS} times.");
            }
        }
    }
}
