//! Debouncer is a library to provide a way of debouncing hardware buttons. It can debounce a whole
//! port in parallel with `PortDebouncer` or individual pins with `PinDebouncer`.
//!
//! # Parallel Debouncing
//!
//! `PortDebouncer` keeps the `N` lasts states provided by its `update` method, and updates its
//! internal port state in the Nth time the `update` method gets called. The number of states `N`
//! together with the frequency at which the `update` method is called determine the total debouncing
//! period, e.g. with `N = 8` and calling `update` every 5ms we get a debouncing period of 40ms.
//! A greater `N` provides better granularity but uses more memory.
//!
//! To reduce resource usage, the library request the number of buttons that will be debounced, this
//! is provide during the struct initialization:
//! ```rust,ignore
//! let mut port_debouncer: PortDebouncer<N, BTNS> = PortDebouncer::new(20,100);
//! ```
//! where, `N` is the number of states for debouncing and `BTNS` is the number of buttons to be debounced
//! in sequential order, passing a higher value to `get_state` causes it to return an error.
//! **NOTE:** Buttons count starts at zero.
//!
//! ## Example
//! ```rust
//! use debouncer::{PortDebouncer, BtnState};
//! use debouncer::typenum::consts::*;
//!
//! let presses: [u32; 8] = [0, 1, 0, 1, 1, 1, 1, 1];
//! let mut port_debouncer: PortDebouncer<U4, U1> = PortDebouncer::new(20, 100);
//!
//! for count in 0..presses.len() / 2 {
//!     port_debouncer.update(presses[count]);
//! }
//! assert_eq!(BtnState::UnPressed, port_debouncer.get_state(0).unwrap());
//!
//! for count in presses.len() / 2..presses.len() {
//!     port_debouncer.update(presses[count]);
//! }
//! assert_eq!(
//!     BtnState::ChangedToPressed,
//!     port_debouncer.get_state(0).unwrap()
//! );
//!
//! let hold_presses = [1u32; 88];
//!
//! for press in hold_presses.iter() {
//!     port_debouncer.update(*press);
//! }
//! assert_eq!(BtnState::Pressed, port_debouncer.get_state(0).unwrap());
//!
//! for count in 0..8 {
//!     port_debouncer.update(hold_presses[count]);
//! }
//! assert_eq!(BtnState::Hold, port_debouncer.get_state(0).unwrap());
//!
//! let repeat_presses = [1u32; 20];
//!
//! for press in repeat_presses.iter() {
//!     port_debouncer.update(*press);
//! }
//! assert_eq!(BtnState::Repeat, port_debouncer.get_state(0).unwrap());
//!
//! assert_eq!(BtnState::Hold, port_debouncer.get_state(0).unwrap());
//!
//! for _ in 0..4 {
//!     port_debouncer.update(0);
//! }
//! assert_eq!(BtnState::UnPressed, port_debouncer.get_state(0).unwrap());
//! ```

#![no_std]

use generic_array::typenum::Unsigned;
use generic_array::{ArrayLength, GenericArray};

pub use generic_array::typenum;

#[derive(Debug)]
pub enum Error {
    /// Error caused by querying the state of a pin which was not initialized during the creation of
    /// the `PortDebouncer` struct
    BtnUninitialized,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum BtnState {
    Pressed = 0,
    UnPressed = 1,
    Repeat = 2,
    Hold = 3,
    ChangedToPressed = 4,
}

pub struct PortDebouncer<N: ArrayLength<u32> + Unsigned, BTNS: ArrayLength<u32> + Unsigned> {
    port_states: GenericArray<u32, N>,
    current_index: usize,
    last_debounced_state: u32,
    debounced_state: u32,
    changed_to_pressed: u32,
    repeat_ticks: usize,
    hold_ticks: usize,
    counter: GenericArray<u32, BTNS>,
}

impl<N, BTNS> PortDebouncer<N, BTNS>
where
    N: ArrayLength<u32> + Unsigned,
    BTNS: ArrayLength<u32> + Unsigned,
{
    /// Returns a PortDebouncer struct
    ///
    /// # Generic arguments
    ///
    /// * `N` - Number of ticks before the pin is considered to be pressed, Unsigned type of the
    /// typenum crate
    /// * `BTNS` - Number of buttons which should be initialized for debouncing. The buttons are
    /// considered to be the bits in sequence order (from least to most significance) in the input
    /// from the `update` method
    ///
    /// # Arguments
    ///
    /// * `repeat_ticks` - The number of ticks after que hold state at which the button is considered
    /// to be in the repeat state, i.e. in the current implementation the button must be first past
    /// the hold state before reaching the repeat state. This number must be a multiple of the
    /// `press_ticks` (`N`) for better accuracy
    ///
    /// * `hold_ticks` - The number of ticks before the pin is considered to be in the hold state
    /// This number must be a multiple of the `press_ticks` for better accuracy
    pub fn new(repeat_ticks: usize, hold_ticks: usize) -> PortDebouncer<N, BTNS> {
        PortDebouncer {
            port_states: GenericArray::default(),
            current_index: 0,
            last_debounced_state: 0,
            debounced_state: 0,
            changed_to_pressed: 0,
            repeat_ticks: repeat_ticks / N::USIZE,
            hold_ticks: hold_ticks / N::USIZE - 1,
            counter: GenericArray::default(),
        }
    }

    /// This method should be called frequently according to the precision required by the
    /// application. The last N states will be used to debounce the pin, where N is the number
    /// chosen for the `press_ticks`. For example, if the user wants a 40ms deboucing time, one can
    /// set the `press_ticks` to 8 and call this method every 5ms. A higher `press_ticks` results in
    /// better precision but also in a higher memory usage in order to store the previous states
    ///
    /// # Arguments
    ///
    /// * `port_value` - The port state in a given time, where its bits represent a pin state. The
    /// pins are considered to be active-high. For an active-low port, the user can use the bitwise
    /// negator operator `!` before passing the value to the method.
    pub fn update(&mut self, port_value: u32) -> bool {
        self.port_states[self.current_index] = port_value;
        if self.current_index != N::USIZE - 1 {
            self.current_index += 1;
            false
        } else {
            self.current_index = 0;
            self.debounced_state = 0xFFFF_FFFF;
            for &state in self.port_states.iter() {
                self.debounced_state &= state;
            }
            self.changed_to_pressed = !self.last_debounced_state & self.debounced_state;

            for (index, btn_counter) in self.counter.iter_mut().enumerate() {
                if (self.last_debounced_state & self.debounced_state & (1 << index)) != 0 {
                    if *btn_counter < (self.hold_ticks + self.repeat_ticks) as u32 {
                        *btn_counter += 1;
                    }
                } else {
                    *btn_counter = 0;
                }
            }
            self.last_debounced_state = self.debounced_state;
            true
        }
    }

    /// Returns the state of the queried pin. It is recommend to call this method each time after
    /// calling the `update` method N times, where N is the chosen `press_ticks`. This is done for
    /// avoiding losing any state change in the port
    ///
    /// # Arguments
    ///
    /// * `pin` - Pin which state must be queried. Where the zeroth pin is considered to be the least
    /// significant bit in the `port_value` used in the `update` method
    pub fn get_state(&mut self, pin: usize) -> Result<BtnState, Error> {
        if pin >= BTNS::USIZE {
            return Err(Error::BtnUninitialized);
        }
        if self.changed_to_pressed & (1 << pin) != 0 {
            return Ok(BtnState::ChangedToPressed);
        }
        if self.counter[pin] >= (self.hold_ticks + self.repeat_ticks) as u32 {
            self.counter[pin] -= self.repeat_ticks as u32;
            Ok(BtnState::Repeat)
        } else if self.counter[pin] >= self.hold_ticks as u32 {
            Ok(BtnState::Hold)
        } else if self.debounced_state & (1 << pin) != 0 {
            Ok(BtnState::Pressed)
        } else {
            Ok(BtnState::UnPressed)
        }
    }
}

pub struct PinDebouncer {
    current_index: u32,
    last_debounced_state: BtnState,
    debounced_state: BtnState,
    press_ticks: u32,
    repeat_ticks: u32,
    hold_ticks: u32,
    counter: u32,
}

impl PinDebouncer {
    pub const fn new(press_ticks: u32, repeat_ticks: u32, hold_ticks: u32) -> PinDebouncer {
        PinDebouncer {
            current_index: 0,
            last_debounced_state: BtnState::UnPressed,
            debounced_state: BtnState::UnPressed,
            press_ticks: press_ticks - 1,
            repeat_ticks,
            hold_ticks: hold_ticks - 1,
            counter: 0,
        }
    }

    pub fn update(&mut self, pin_value: bool) -> bool {
        if pin_value {
            if self.counter < self.hold_ticks + self.repeat_ticks {
                self.counter += 1;
            }
        } else {
            self.counter = 0;
        }

        if self.current_index != self.press_ticks {
            self.current_index += 1;
            return false;
        }

        self.current_index = 0;
        if self.counter >= self.press_ticks {
            self.debounced_state = BtnState::Pressed;
        } else {
            self.debounced_state = BtnState::UnPressed;
        }
        if (self.last_debounced_state == BtnState::UnPressed)
            && (self.debounced_state == BtnState::Pressed)
        {
            self.debounced_state = BtnState::ChangedToPressed;
        } else if self.counter >= self.hold_ticks + self.repeat_ticks {
            self.debounced_state = BtnState::Repeat;
        } else if self.counter >= self.hold_ticks {
            self.debounced_state = BtnState::Hold;
        }
        self.last_debounced_state = self.debounced_state;
        true
    }

    pub fn get_state(&mut self) -> BtnState {
        match self.debounced_state {
            BtnState::Repeat => {
                self.counter -= self.repeat_ticks;
                self.debounced_state = BtnState::Hold;
                BtnState::Repeat
            }
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use generic_array::typenum::consts::*;

    #[test]
    fn port_btn0_pressed() {
        let presses: [u32; 8] = [0, 1, 0, 1, 1, 1, 1, 1];
        let mut port_debouncer: PortDebouncer<U4, U1> = PortDebouncer::new(20, 100);

        for &value in presses.iter().take(presses.len() / 2) {
            port_debouncer.update(value);
        }
        assert_eq!(BtnState::UnPressed, port_debouncer.get_state(0).unwrap());

        for &value in presses.iter().skip(presses.len() / 2) {
            port_debouncer.update(value);
        }
        assert_eq!(
            BtnState::ChangedToPressed,
            port_debouncer.get_state(0).unwrap()
        );

        let hold_presses = [1u32; 88];

        for &value in hold_presses.iter() {
            port_debouncer.update(value);
        }
        assert_eq!(BtnState::Pressed, port_debouncer.get_state(0).unwrap());

        for &value in hold_presses.iter().take(8) {
            port_debouncer.update(value);
        }
        assert_eq!(BtnState::Hold, port_debouncer.get_state(0).unwrap());

        let repeat_presses = [1u32; 20];

        for &value in repeat_presses.iter() {
            port_debouncer.update(value);
        }
        assert_eq!(BtnState::Repeat, port_debouncer.get_state(0).unwrap());

        assert_eq!(BtnState::Hold, port_debouncer.get_state(0).unwrap());

        for _ in 0..4 {
            port_debouncer.update(0);
        }
        assert_eq!(BtnState::UnPressed, port_debouncer.get_state(0).unwrap());
    }

    #[test]
    fn port_btn1_pressed() {
        let presses: [u32; 8] = [0, 1, 0, 1, 2, 2, 2, 2];
        let mut port_debouncer: PortDebouncer<U4, U2> = PortDebouncer::new(20, 100);

        for &value in presses.iter().take(presses.len() / 2) {
            port_debouncer.update(value);
        }
        assert_eq!(BtnState::UnPressed, port_debouncer.get_state(1).unwrap());

        for &value in presses.iter().skip(presses.len() / 2) {
            port_debouncer.update(value);
        }
        assert_eq!(
            BtnState::ChangedToPressed,
            port_debouncer.get_state(1).unwrap()
        );

        let hold_presses = [2u32; 88];

        for press in hold_presses.iter() {
            port_debouncer.update(*press);
        }
        assert_eq!(BtnState::Pressed, port_debouncer.get_state(1).unwrap());

        for &value in hold_presses.iter().take(8) {
            port_debouncer.update(value);
        }
        assert_eq!(BtnState::Hold, port_debouncer.get_state(1).unwrap());

        let repeat_presses = [2u32; 20];

        for press in repeat_presses.iter() {
            port_debouncer.update(*press);
        }
        assert_eq!(BtnState::Repeat, port_debouncer.get_state(1).unwrap());

        assert_eq!(BtnState::Hold, port_debouncer.get_state(1).unwrap());

        for _ in 0..4 {
            port_debouncer.update(1);
        }
        assert_eq!(BtnState::UnPressed, port_debouncer.get_state(1).unwrap());
    }

    #[test]
    #[should_panic]
    fn port_out_of_bound_btn() {
        let presses: [u32; 8] = [1, 1, 1, 1, 2, 2, 2, 2];
        let mut port_debouncer: PortDebouncer<U4, U1> = PortDebouncer::new(20, 100);

        for &value in presses.iter() {
            port_debouncer.update(value);
        }
        let _ = port_debouncer.get_state(1).unwrap();
    }

    #[test]
    fn pin_pressed() {
        let mut pin_debouncer = PinDebouncer::new(4, 20, 100);
        let presses: [bool; 8] = [false, true, false, true, true, true, true, true];

        for &value in presses.iter().take(presses.len() / 2) {
            pin_debouncer.update(value);
        }
        assert_eq!(BtnState::UnPressed, pin_debouncer.get_state());

        for &value in presses.iter().skip(presses.len() / 2) {
            pin_debouncer.update(value);
        }
        assert_eq!(BtnState::ChangedToPressed, pin_debouncer.get_state());

        for _ in 0..88 {
            pin_debouncer.update(true);
        }
        assert_eq!(BtnState::Pressed, pin_debouncer.get_state());

        for _ in 0..8 {
            pin_debouncer.update(true);
        }
        assert_eq!(BtnState::Hold, pin_debouncer.get_state());

        for _ in 0..20 {
            pin_debouncer.update(true);
        }
        assert_eq!(BtnState::Repeat, pin_debouncer.get_state());
        assert_eq!(BtnState::Hold, pin_debouncer.get_state());

        for _ in 0..4 {
            pin_debouncer.update(false);
        }
        assert_eq!(BtnState::UnPressed, pin_debouncer.get_state());
    }
}
