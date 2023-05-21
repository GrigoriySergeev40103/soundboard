use std::{sync::{Arc, Mutex}, thread, time::Duration, str::FromStr, path::PathBuf};

use livesplit_hotkey::{Hook, Hotkey, KeyCode, Modifiers};
use ringbuf::HeapRb;
use rodio::{
    cpal::{traits::{HostTrait, StreamTrait}, Stream},
    DeviceTrait, OutputStream, Device,
};

mod sound;
mod soundboard;

use sound::Sound;
use soundboard::Soundboard;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Get host and devices
    let host = rodio::cpal::default_host();

    if args.len() < 2 {
        eprintln!("Not enough arguments!");
        return;
    }

    // in mode "bind" expected syntax:
    // [.exe] bind name path_to hotkey
    let mode = args[1].as_str();
    match mode {
        "bind" => {
            if args.len() < 4 {
                eprintln!("Not enough arguments for bind mode!");
                return;
            }

            let new_sound = Sound::new(args[2].clone(), PathBuf::from_str(args[3].as_str()).unwrap());

            let (_stream, stream_handle) = OutputStream::try_from_device(&host.default_output_device().unwrap()).unwrap();
            let mut soundboard = Soundboard::new(stream_handle.clone()).expect("Couldn't instantiate a soundboard");

            if let Err(error) = soundboard.load_from_save("save.json") {
                eprintln!("Couldn't load from file {}", error);
            }

            soundboard.bind_new(new_sound, Hotkey::from_str(args[4].as_str()).expect("Oopsie")).unwrap();
            soundboard.save_to_file().unwrap();
            return;
        },
        "unbind" => {
            let (_stream, stream_handle) = OutputStream::try_from_device(&host.default_output_device().unwrap()).unwrap();
            let mut soundboard = Soundboard::new(stream_handle.clone()).expect("Couldn't instantiate a soundboard");

            if let Err(error) = soundboard.load_from_save("save.json") {
                eprintln!("Couldn't load from file {}", error);
            }

            soundboard.unbind(Hotkey::from_str(args[2].as_str()).expect("Oopsie")).unwrap();
            soundboard.save_to_file().unwrap();
            return;
        },
        "ls" => {
            let (_stream, stream_handle) = OutputStream::try_from_device(&host.default_output_device().unwrap()).unwrap();
            let mut soundboard = Soundboard::new(stream_handle.clone()).expect("Couldn't instantiate a soundboard");

            if let Err(error) = soundboard.load_from_save("save.json") {
                eprintln!("Couldn't load from file {}", error);
            }

            let sound_binds = soundboard.sounds();

            for sound_bind in sound_binds {
                println!("{} -> {}", sound_bind.1.name, sound_bind.0);
            }

            soundboard.save_to_file().unwrap();
            return;
        },
        "run" => {

        }
        _ => {
            return;
        }
    }

    // Play sound into virtual device of vb_cable(https://vb-audio.com/Cable/index.htm)
    let out_v_device = host.output_devices().unwrap().last().unwrap();

    let (_stream, stream_handle) = OutputStream::try_from_device(&out_v_device).unwrap();
    let mut soundboard = Soundboard::new(stream_handle.clone()).expect("Couldn't instantiate a soundboard");
    soundboard.load_from_save("save.json").unwrap();

    // Listen for default microphone and forward it's recording into vb_cable wich in turn replays it from vb_cable micro
    let microphone = host.default_input_device().unwrap();

    let micro_feedback = setup_microphone_feedback(&microphone, &out_v_device);
    micro_feedback.0.play().unwrap();
    micro_feedback.1.play().unwrap();

    let hook = Hook::new().unwrap();
    let modifiers = Modifiers::SHIFT.union(Modifiers::ALT).union(Modifiers::CONTROL);
    let termination_hotkey = Hotkey {
        key_code: KeyCode::KeyC,
        modifiers
    };

    let exit_flag = Arc::new(Mutex::new(false));
    let flag_clone = Arc::clone(&exit_flag);

    let wait_for_exit = move || {
        let mut mutex_bool = flag_clone.lock().unwrap();
        *mutex_bool = true;
    };

    hook.register(termination_hotkey, wait_for_exit).unwrap();

    loop {
        thread::sleep(Duration::from_millis(100));
        let flag = exit_flag.lock().unwrap();

        if *flag == true {
            break;
        }
    }

    soundboard.save_to_file().unwrap();
}

fn err_fn(err: rodio::cpal::StreamError) {
    eprintln!("An error occured on stream: {}", err);
}

/// Builds input and output streams where input stream flows into output creating feedback.
/// Used to create microphone feedback into output device(In this program to feedback micro into vb-cable)
fn setup_microphone_feedback(in_device: &Device, out_device: &Device) -> (Stream, Stream){
    let in_config = in_device.default_input_config().unwrap().config();
    let latency_frames = (150.0 / 1000.0) * in_config.sample_rate.0 as f32;
    let latency_samples = latency_frames as usize * in_config.channels as usize;

    // The buffer to share samples
    let ring = HeapRb::<f32>::new(latency_samples * 2);
    let (mut producer, mut consumer) = ring.split();

    // Fill the samples with 0.0 equal to the length of the delay.
    for _ in 0..latency_samples {
        // The ring buffer has twice as much space as necessary to add latency here,
        // so this should never fail
        producer.push(0.0).unwrap();
    }

    let input_data_fn = move |data: &[f32], _: &rodio::cpal::InputCallbackInfo| {
        let mut output_fell_behind = false;
        for &sample in data {
            if producer.push(sample).is_err() {
                output_fell_behind = true;
            }
        }
        if output_fell_behind {
            eprintln!("output stream fell behind: try increasing latency");
        }
    };

    let output_data_fn = move |data: &mut [f32], _: &rodio::cpal::OutputCallbackInfo| {
        let mut input_fell_behind = false;
        for sample in data {
            *sample = match consumer.pop() {
                Some(s) => s,
                None => {
                    input_fell_behind = true;
                    0.0
                }
            };
        }
        if input_fell_behind {
            eprintln!("input stream fell behind: try increasing latency");
        }
    };

    let out_config = out_device.default_output_config().unwrap();

    let input_stream = in_device
        .build_input_stream(&in_config, input_data_fn, err_fn, Option::None)
        .unwrap();
    let output_stream = out_device
        .build_output_stream(&out_config.config(), output_data_fn, err_fn, None)
        .unwrap();

    (input_stream, output_stream)
}