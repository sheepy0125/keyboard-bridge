/*!
 * Keyboard Bridge for Raspberry Pi - Chords
 * Created by sheepy0125 on 2023-07-23 under the MIT license
**/

/***** Setup *****/
use crate::{key::*, Keyboard};
use log::error;
use std::process::exit;
use KeyCode::*;
use ModifierKey::*;
use RegularKey::*;
// Constants
pub type ChordSequence = [KeyCode];

/***** Chord sequences *****/
/* A chord sequence begins with the CHORD_SEQUENCE_START_KEY. Once that key has
 * been pressed, the all chords declared in ALL_CHORDS are listened for. However,
 * the start key should not be included as the first element to the array.
**/
pub const CHORD_SEQUENCE_START_KEY: KeyCode = Regular(Enter);
pub const ALL_CHORDS: &[&ChordSequence] = &[
    QUIT_CHORD_SEQUENCE,
    // Extra chords go here. Example:
    /* HELLO_WORLD_CHORD_SEQUENCE, */
];
pub const QUIT_CHORD_SEQUENCE: &ChordSequence = &[
    Modifier(EitherShift),
    Regular(Grave),
    Regular(Period),
    Regular(Backspace),
    Regular(Backspace),
    Regular(Backspace),
];
// Extra chords go here. Example:
/*
pub const HELLO_WORLD_CHORD_SEQUENCE: &ChordSequence = &[
    Modifier(EitherShift),
    Regular(Grave),
    Regular(Period),
    Regular(H), Regular(E), Regular(L), Regular(L), Regular(O),
    Regular(Space),
    Regular(W), Regular(O), Regular(R), Regular(L), Regular(D),
];
*/

/***** Chord sequence handlers *****/
impl<'a> Keyboard<'a> {
    pub fn handle_chord(&mut self, chord: &ChordSequence) {
        match chord {
            QUIT_CHORD_SEQUENCE => {
                exit(0);
            }
            // Extra chords go here. Example:
            /*
            HELLO_WORLD_CHORD_SEQUENCE => {
                info!("Hello, World!");
            }
            */
            _ => error!("Unhandled chord: {chord:?}"),
        }
    }
}
