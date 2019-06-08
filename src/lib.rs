#![no_std]

use generic_array::{GenericArray, ArrayLength};
use typenum::Unsigned;

#[derive(Debug)]
pub enum Error {
    BtnUninitialized,
    #[doc(hidden)]
    _Extensible,
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

impl<N, BTNS> PortDebouncer<N, BTNS> where 
    N: ArrayLength<u32> + Unsigned,
    BTNS: ArrayLength<u32> + Unsigned {

    pub fn new(repeat_ticks: usize, hold_ticks: usize) -> PortDebouncer<N, BTNS> {

        PortDebouncer {
            port_states: GenericArray::default(),
            current_index: 0,
            last_debounbed_state: 0,
            debounced_state: 0,
            changed_to_pressed: 0,
            press_ticks: N::to_usize(),
            repeat_ticks: repeat_ticks/N::to_usize(),
            hold_ticks: hold_ticks/N::to_usize() - 1,
            counter: GenericArray::default(),
        }
    }

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
                    
                    self.counter[btn] += 1;
                } else {
                    self.counter[btn] = 0;
                }
            }
            self.last_debounbed_state = self.debounced_state;
        }
    }

    pub fn get_state(&mut self, pin: usize) -> Result<BtnState, Error> {
        
        if self.changed_to_pressed & (1 << pin) != 0 {
            return Ok(BtnState::ChangedToPressed)
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
            press_ticks: press_ticks,
            repeat_ticks: repeat_ticks,
            hold_ticks: hold_ticks,
            counter: 0,
        }
    }

    pub fn update(&mut self, pin_value: bool) {
        
        if pin_value {
            if self.counter < self.hold_ticks {
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

            if (self.last_debounbed_state == BtnState::UnPressed) && 
            (self.debounced_state == BtnState::Pressed) {
                self.debounced_state = BtnState::ChangedToPressed;

            } else if self.counter >= self.hold_ticks {
                self.debounced_state = BtnState::Hold;

            } else if self.counter >= self.repeat_ticks {
                self.debounced_state = BtnState::Repeat;  
            }
            self.last_debounbed_state = self.debounced_state;
        }
    }

    pub fn get_state(&mut self) -> BtnState {

        match self.debounced_state {
            BtnState::Hold => {
                self.counter -= self.hold_ticks;
                BtnState::Hold
            },
            BtnState::Repeat => {
                self.counter -= self.repeat_ticks;
                BtnState::Repeat
            },
            other => {
                other
            }
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

        for count in 0..presses.len()/2 {
            port_debouncer.update(presses[count]);
        }
        assert_eq!(BtnState::UnPressed, port_debouncer.get_state(0).unwrap());

        for count in presses.len()/2..presses.len() {
            port_debouncer.update(presses[count]);
        }
        assert_eq!(BtnState::ChangedToPressed, port_debouncer.get_state(0).unwrap());

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

        for count in 0..presses.len()/2 {
            port_debouncer.update(presses[count]);
        }
        assert_eq!(BtnState::UnPressed, port_debouncer.get_state(1).unwrap());

        for count in presses.len()/2..presses.len() {
            port_debouncer.update(presses[count]);
        }
        assert_eq!(BtnState::ChangedToPressed, port_debouncer.get_state(1).unwrap());

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
}
