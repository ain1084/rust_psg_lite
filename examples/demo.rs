use direct_ring_buffer;
use psg_lite::Output;
use psg_lite::OutputSample;
use psg_lite::SoundGenerator;
use std::slice::Iter;
use std::time::Duration;
use tinyaudio::run_output_device;
use tinyaudio::OutputDeviceParameters;

#[derive(Clone, Copy)]
enum Command {
    Note(usize, f32),
    Vol(usize, u8),
    Mode(usize, Output),
    Noise(u8),
    Wait(Duration),
    Mute,
}

struct Sequencer<'a, T: OutputSample<T>> {
    commands: Iter<'a, Command>,
    sg: SoundGenerator,
    samples: usize,
    producer: direct_ring_buffer::Producer<T>,
}

impl<'a, T: OutputSample<T>> Sequencer<'a, T> {
    fn new(
        commands: &'a [Command],
        clock_rate: u32,
        sample_rate: u32,
        producer: direct_ring_buffer::Producer<T>,
    ) -> Self {
        let mut instance = Self {
            commands: commands.iter(),
            sg: SoundGenerator::new(clock_rate, sample_rate),
            producer,
            samples: 0,
        };
        instance.fill_buffer();
        instance
    }

    fn fill_buffer(&mut self) -> bool
    where
        T: OutputSample<T>,
    {
        loop {
            let written = self.producer.write_slices(
                |data, _offset| {
                    data.fill_with(|| self.sg.next_sample());
                    data.len()
                },
                Some(self.samples),
            );
            self.samples -= written;
            if self.samples != 0 {
                return true;
            }
            match self.commands.next() {
                Some(Command::Note(ch, freq)) => {
                    let tune = (self.sg.clock_rate() as f32 / (freq * 16f32)).round() as u16;
                    self.sg.set_period(*ch, tune)
                }
                Some(Command::Mode(ch, mix)) => self.sg.set_mode(*ch, *mix),
                Some(Command::Vol(ch, vol)) => self.sg.set_volume(*ch, *vol),
                Some(Command::Mute) => (0..3).for_each(|ch| self.sg.set_mode(ch, Output::NONE)),
                Some(Command::Noise(freq)) => self.sg.set_noise_period(*freq),
                Some(Command::Wait(duration)) => {
                    self.samples =
                        self.sg.sample_rate() as usize * duration.as_millis() as usize / 1000
                }
                // End of commands.
                None => return false,
            }
        }
    }
}

fn main() {
    const NOTE_C4: f32 = 261.626;
    const NOTE_D4: f32 = 293.665;
    const NOTE_E4: f32 = 329.628;
    const NOTE_F4: f32 = 349.228;

    let wait_1sec = Command::Wait(Duration::from_millis(1000));
    let commands = &[
        // o4c (mono)
        Command::Vol(0, 12),
        Command::Mode(0, Output::TONE),
        Command::Note(0, NOTE_C4),
        wait_1sec,
        // o4d
        Command::Note(0, NOTE_D4),
        wait_1sec,
        // o4e
        Command::Note(0, NOTE_E4),
        wait_1sec,
        // Mute 1sec
        Command::Mute,
        wait_1sec,
        // o4c (detuned)
        Command::Vol(1, 12),
        Command::Mode(0, Output::TONE),
        Command::Mode(1, Output::TONE),
        // o4c
        Command::Note(0, NOTE_C4),
        Command::Note(1, NOTE_C4 + 2.0),
        wait_1sec,
        // o4d
        Command::Note(0, NOTE_D4),
        Command::Note(1, NOTE_D4 + 2.0),
        wait_1sec,
        // o4e
        Command::Note(0, NOTE_E4),
        Command::Note(1, NOTE_E4 + 2.0),
        wait_1sec,
        // Mute 1sec
        Command::Mute,
        wait_1sec,
        // noise 0
        Command::Mode(0, Output::NOISE),
        Command::Vol(0, 12),
        Command::Noise(0),
        wait_1sec,
        // noise 10
        Command::Noise(10),
        wait_1sec,
        // noise 20
        Command::Noise(20),
        wait_1sec,
        // noise 30
        Command::Noise(30),
        wait_1sec,
        // Mute 1sec
        Command::Mute,
        wait_1sec,
        // noise 0 & tone
        Command::Mode(0, Output::NOISE | Output::TONE),
        Command::Note(0, NOTE_C4),
        Command::Vol(0, 12),
        Command::Noise(0),
        wait_1sec,
        // noise 10 & tone
        Command::Noise(10),
        Command::Note(0, NOTE_D4),
        wait_1sec,
        // noise 20 & tone
        Command::Noise(20),
        Command::Note(0, NOTE_E4),
        wait_1sec,
        // noise 30 & tone
        Command::Noise(30),
        Command::Note(0, NOTE_F4),
        wait_1sec,
    ];

    const CLOCK_RATE: u32 = 1789772;
    const SAMPLE_RATE: u32 = 44100;
    const SAMPLE_BUFFER_SIZE: usize = SAMPLE_RATE as usize / 10;

    let (producer, mut consumer) =
        direct_ring_buffer::create_ring_buffer::<f32>(SAMPLE_BUFFER_SIZE);
    let mut sequencer = Sequencer::new(commands, CLOCK_RATE, SAMPLE_RATE, producer);
    let _device = run_output_device(
        OutputDeviceParameters {
            channels_count: 1,
            sample_rate: SAMPLE_RATE as usize,
            channel_sample_count: SAMPLE_BUFFER_SIZE,
        },
        move |buf| {
            let buf_len = buf.len();
            let written = consumer.read_slices(
                |input, offset| {
                    buf[offset..offset + input.len()].copy_from_slice(input);
                    input.len()
                },
                Some(buf_len),
            );
            buf[written..].fill(f32::default());
        },
    )
    .unwrap();
    while sequencer.fill_buffer() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    // wait for drain
    std::thread::sleep(std::time::Duration::from_millis(1500));
}
