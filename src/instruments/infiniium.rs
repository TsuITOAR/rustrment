use super::Model;

#[derive(Default)]
pub struct Infiniium;

impl crate::DefaultConfig for Infiniium {
    type DefaultProtocol = crate::protocols::Tcp;
    const DEFAULT_PROTOCOL: crate::protocols::Tcp = crate::protocols::Tcp;
}

impl Model for Infiniium {
    const DESCRIPTION: &'static str = "Infiniium (A/H/Q/S/V/X/Z) Series Oscilloscopes";
    type Command = Command;
    type Query = Query;
    const END_BYTE: u8 = b'\n';
    const TERMINATOR: u8 = b'\n';
}



pub enum Command {
    ///Trigger
    Trig(Trig),
}

pub enum Trig {
    Level(TrigLevel),
    Mode(TrigMode),
}
pub struct TrigLevel {
    channel: u8,
    level: f32,
}
pub enum TrigMode {
    ///Edge trigger mode.
    Edge,
    ///Trigger on a pulse that has a width less than a specified amount of time.
    Glitch,
    ///Pattern triggering lets you trigger the oscilloscope using more than one channel as the trigger source. You can also use pattern triggering to trigger on a pulse of a given width.
    Pattern,
    ///State triggering lets you set the oscilloscope to use several channels as the trigger source, with one of the channels being used as a clock waveform.
    State,
    ///Delay by Events mode lets you view pulses in your waveform that occur a number of events after a specified waveform edge. Delay by Time mode lets you view pulses in your waveform that occur a long time after a specified waveform edge.
    Delay,
    ///Timeout triggering lets you trigger when the waveform remains high too long, low to long, or unchanged too long.
    Timeout,
    ///TV trigger mode lets you trigger the oscilloscope on one of the standard television waveforms. You can also use this mode to trigger on a custom television waveform that you define.
    TV,
    ///COMM mode lets you trigger on a serial pattern of bits in a waveform.
    Comm,
    ///Runt triggering lets you trigger on positive or negative pulses that are smaller in amplitude than other pulses in your waveform.
    Runt,
    ///(Available on 90000A Series, 90000 X-Series, V-Series, 90000 Q-Series, and Z-Series oscilloscopes.) Sequential triggering lets you use multiple events or time/pattern qualifications to define your trigger.
    Sequence,
    ///Setup and Hold triggering let you trigger on Setup or Hold violations in your circuit.
    SHold,
    ///Edge Transition triggering lets you trigger on an edge that violates a rise time or fall time specification.
    Transition,
    ///Window triggering lets you define a window on screen and then trigger when the waveform exits the window, enters it, or stays inside/outside the window for too long/short.
    Window,
    ///Pulse width triggering lets you trigger on a pulse that is greater than or less than a specified width and of a certain polarity.
    PWidth,
    ///Allows backward compatibility access to the DELay, PATTern, STATe, TV, and VIOLation modes. When this mode is selected, use the :TRIGger:ADVanced:MODE command to select the advanced trigger mode.
    Advanced,
    ///Serial triggering on SBUS1, SBUS2, SBUS3, or SBUS4.
    SBus(u8),
}

impl super::Command for Command {
    type R = Box<[u8]>;
    fn to_bytes(self) -> Self::R {
        unimplemented!()
    }
}

pub enum Query {}

impl super::Query for Query {
    type R = &'static str;
    fn to_bytes(self) -> Self::R {
        unimplemented!()
    }
}
