#![no_std]

use generic_array::{ArrayLength, GenericArray};
use typenum::Unsigned;

#[derive(Debug)]
pub enum Error {
    /// Error caused by querying the state of a pin which was not initialized during the creation of
    /// the PortDebouncer struct
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

pub struct PortDebouncer<N: ArrayLength<u32>, BTNS: ArrayLength<u32>> {
    port_states: GenericArray<u32, N>,
    current_index: usize,
    last_debounbed_state: u32,
    debounced_state: u32,
    changed_to_pressed: u32,
    press_ticks: usize,
    repeat_ticks: usize,
    hold_ticks: usize,
    counter: GenericArray<u32, BTNS>,
}

impl<N, BTNS> PortDebouncer<N, BTNS>
where
    N: ArrayLength<u32> + Unsigned,
    BTNS: ArrayLength<u32> + Unsigned,
{
    /// Returns a PortDeboncer struct
    /// 
    /// # Generic arguments
    /// 
    /// * `N` - Number of ticks before the pin is considered to be pressed, Unsiged type of the
    /// typenum crate
    /// * `BTNS` - Number of buttons which should be initialised for debouncing. The buttons are
    /// considered to be the bits in sequence order (from least to most significance) in the input
    /// from the `update` method
    /// 
    /// # Arguments
    /// 
    /// * `repeat_ticks` - The number of ticks after que hold state at which the button is considered
    /// to be in the repeat state, i.e. in the current implementation the button must be first past
    /// the hold state before reaching the repeat state. This number must be a multiple of the
    /// `press_ticks` for better accuracy
    /// 
    /// * `hold_ticks` - The number of ticks before the pin is considered to be in the hold state
    /// This number must be a multiple of the `press_ticks` for better accuracy
    pub fn new(repeat_ticks: usize, hold_ticks: usize) -> PortDebouncer<N, BTNS> {
        PortDebouncer {
            port_states: GenericArray::default(),
            current_index: 0,
            last_debounbed_state: 0,
            debounced_state: 0,
            changed_to_pressed: 0,
            press_ticks: N::to_usize(),
            repeat_ticks: repeat_ticks / N::to_usize(),
            hold_ticks: hold_ticks / N::to_usize() - 1,
            counter: GenericArray::default(),
        }
    }

    /// This method should be called frequently according to the precision required by the
    /// application. The last N states will be used to debounce the pin, where N is the number
    /// chosen for the `press_ticks`. For example, if the user wants a 40ms deboucing time, one can
    /// set the `press_ticks` to 4 and call this method every 5ms. A higher `press_ticks` results in
    /// better precision but also in a higher memory usage in order to store the previous states
    /// 
    /// # Arguments
    /// 
    /// * `port_value` - The port state in a given time, where is bit represents a pin state. The pins
    /// are considered to be active-high, if one wants to use a active-low port, he can use the
    /// bitwise negator operator `!`.
    pub fn update(&mut self, port_value: u32) {
        self.port_states[self.current_index] = port_value;
        if self.current_index != self.press_ticks - 1 {
            self.current_index += 1;
        } else {
            self.current_index = 0;
            self.debounced_state = 0xFFFFFFFF as u32;
            let states_slice = &self.port_states[..self.press_ticks];
            for state in 0..self.press_ticks {
                self.debounced_state &= states_slice[state];
            }
            self.changed_to_pressed = !self.last_debounbed_state & self.debounced_state;

            for btn in 0..BTNS::to_usize() {
                if (self.last_debounbed_state & self.debounced_state & (1 << btn)) != 0 {
                    if self.counter[btn] < (self.hold_ticks + self.repeat_ticks) as u32 {
                        self.counter[btn] += 1;
                    }
                } else {
                    self.counter[btn] = 0;
                }
            }
            self.last_debounbed_state = self.debounced_state;
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
        if self.changed_to_pressed & (1 << pin) != 0 {
            return Ok(BtnState::ChangedToPressed);
        }
        if pin >= BTNS::to_usize() {
            Err(Error::BtnUninitialized)
        } else {
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
}

pub struct PinDebouncer {
    current_index: u32,
    last_debounbed_state: BtnState,
    debounced_state: BtnState,
    press_ticks: u32,
    repeat_ticks: u32,
    hold_ticks: u32,
    counter: u32,
}

impl PinDebouncer {
    pub fn new(press_ticks: u32, repeat_ticks: u32, hold_ticks: u32) -> PinDebouncer {
        PinDebouncer {
            current_index: 0,
            last_debounbed_state: BtnState::UnPressed,
            debounced_state: BtnState::UnPressed,
            press_ticks: press_ticks - 1,
            repeat_ticks: repeat_ticks,
            hold_ticks: hold_ticks - 1,
            counter: 0,
        }
    }

    pub fn update(&mut self, pin_value: bool) {
        if pin_value {
            if self.counter < self.hold_ticks + self.repeat_ticks {
                self.counter += 1;
            }
        } else {
            self.counter = 0;
        }

        if self.current_index != self.press_ticks {
            self.current_index += 1;
        } else {
            self.current_index = 0;

            if self.counter >= self.press_ticks {
                self.debounced_state = BtnState::Pressed;
            } else {
                self.debounced_state = BtnState::UnPressed;
                return;
            }

            if (self.last_debounbed_state == BtnState::UnPressed)
                && (self.debounced_state == BtnState::Pressed)
            {
                self.debounced_state = BtnState::ChangedToPressed;
            } else if self.counter >= self.hold_ticks + self.repeat_ticks {
                self.debounced_state = BtnState::Repeat;
            } else if self.counter >= self.hold_ticks {
                self.debounced_state = BtnState::Hold;
            }
            self.last_debounbed_state = self.debounced_state;
        }
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
    use typenum::consts::*;

    #[test]
    fn port_btn0_pressed() {
        let presses: [u32; 8] = [0, 1, 0, 1, 1, 1, 1, 1];
        let mut port_debouncer: PortDebouncer<U4, U1> = PortDebouncer::new(20, 100);

        for count in 0..presses.len() / 2 {
            port_debouncer.update(presses[count]);
        }
        assert_eq!(BtnState::UnPressed, port_debouncer.get_state(0).unwrap());

        for count in presses.len() / 2..presses.len() {
            port_debouncer.update(presses[count]);
        }
        assert_eq!(
            BtnState::ChangedToPressed,
            port_debouncer.get_state(0).unwrap()
        );

        let hold_presses = [1u32; 88];

        for press in hold_presses.iter() {
            port_debouncer.update(*press);
        }
        assert_eq!(BtnState::Pressed, port_debouncer.get_state(0).unwrap());

        for count in 0..8 {
            port_debouncer.update(hold_presses[count]);
        }
        assert_eq!(BtnState::Hold, port_debouncer.get_state(0).unwrap());

        let repeat_presses = [1u32; 20];

        for press in repeat_presses.iter() {
            port_debouncer.update(*press);
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

        for count in 0..presses.len() / 2 {
            port_debouncer.update(presses[count]);
        }
        assert_eq!(BtnState::UnPressed, port_debouncer.get_state(1).unwrap());

        for count in presses.len() / 2..presses.len() {
            port_debouncer.update(presses[count]);
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

        for count in 0..8 {
            port_debouncer.update(hold_presses[count]);
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
        let presses: [u32; 8] = [0, 1, 0, 1, 1, 1, 1, 1];
        let mut port_debouncer: PortDebouncer<U4, U1> = PortDebouncer::new(20, 100);

        for press in presses.iter() {
            port_debouncer.update(*press);
        }
        let _ = port_debouncer.get_state(1).unwrap();
    }

    #[test]
    fn pin_pressed() {
        let mut pin_debouncer = PinDebouncer::new(4, 20, 100);
        let presses: [bool; 8] = [false, true, false, true, true, true, true, true];

        for count in 0..presses.len() / 2 {
            pin_debouncer.update(presses[count]);
        }
        assert_eq!(BtnState::UnPressed, pin_debouncer.get_state());

        for count in presses.len() / 2 .. presses.len() {
            pin_debouncer.update(presses[count]);
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