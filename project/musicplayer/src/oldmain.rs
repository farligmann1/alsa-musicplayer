use std::fs::File;
use std::io::Read;
use std::io;

use alsa::{Direction, ValueOr};
use alsa::pcm::{Access, Format, HwParams, State, PCM};


mod wavfile;

use crate::wavfile::WavFile;

fn oldmain() -> io::Result<()>{

    let wav = WavFile::new("src/wav_sample.wav")?;

    // I found out that supporting all kinds of formats and bit depths 
    // was a much bigger headache than anticipated, because i would have to handle many 
    // different combinations of them, some of which have special behaviour. Since this
    // diverges from what i intended to spend my time on, i will comment out my half-solution
    // below and just have this check instead.
    if wav.fmt != 1 || wav.bit_depth != 16 {
        panic!("Audio format must be PCM and bit depth must be 16!")
    }

    // NOTE: none of ALSA's functions implements '?' so we need to use unwrap instead.
    let pcm = PCM::new("default", Direction::Playback, false).unwrap();

    // hwp is ALSA's hardware parameters.
    let hwp = HwParams::any(&pcm).unwrap();

    hwp.set_channels(wav.channels.into()).unwrap();
    hwp.set_rate(wav.sample_rate, ValueOr::Nearest).unwrap();

    //    
    //    // Format is the representation of samples. We have to set this depending on what the audio format is 
    //    // and what the bit depth we read from the read from the file is.
    //    let format_setting: Format;
    //    match (wav.fmt, wav.bit_depth) {
    //        (1, 16) => format_setting = Format::s16(),
    //        //(1, 24) => format_setting = Format::s24_3(),
    //        (1, 32) => format_setting = Format::s32(),
    //        (3, 32) => format_setting = Format::float(),
    //        _ => panic!("Invalid format for .wav file")
    //    }
    //    hwp.set_format(format_setting).unwrap();
    //    

    // This is the format, determined by the format and bit depth.
    // Hardcoded to signed 16-bit integer. See explanation above
    hwp.set_format(Format::s16()).unwrap();


    // It is standard for samples to be interleaved when we have multiple channels.
    // This means that for stereo audio (2 channels), samples will be laid ot like this
    // in the data section:  ..12121212121212...  , where 1 and 2 represents the channels.
    // This of course also applies when we have more channels, like 5.1 surround.
    hwp.set_access(Access::RWInterleaved).unwrap();
    pcm.hw_params(&hwp).unwrap();

    //    
    //    //
    //    let io:IO<'_, Any>;// = pcm.io_i16().unwrap();
    //    match wav.bit_depth {
    //        16 => io = pcm.io_i16().unwrap(),
    //        //24 => ,
    //        32 => io = pcm.io_i32().unwrap(),
    //        _ => panic!("Unsupported sample format")
    //    }
    //

    let io = pcm.io_i16().unwrap();

    // Make sure we don't start the stream too early.
    // hwp = hardware parameters
    // swp = software parameters
    let hwp = pcm.hw_params_current().unwrap();
    let swp = pcm.sw_params_current().unwrap();
    swp.set_start_threshold(hwp.get_buffer_size().unwrap()).unwrap();
    pcm.sw_params(&swp).unwrap();


    // file.read expects a u8 slice as a buffer, but the actual samples are i16. To get around this,
    // we will convert the data later. The size of the buffer is very arbitrary, 2 to some power is
    // recommended. Too large buffers might result in lag, and too small buffers may result in unnecessarily
    // large CPU loads. 8192 bytes is 4096 samples per buffer, which is a reasonable amount
    let mut buf = [0u8; 8192];

    // Open the file we want to play, given by the path field in the WavFile struct. We read this 
    // field and not a hard coded value for modularity if we were to implement a playlist feature 
    // in the future
    let mut file = File::open(wav.path)?;

    // Reads the file until EOF
    while let Ok(n) = file.read(&mut buf) {
        // if we couldn't read any more bytes, immediately break
        if n == 0 { break; }

        // initialize to half the length, as 2x u8 = 1x i16
        let mut output = Vec::with_capacity(buf.len() / 2);
        
        // read all bytes and push them to the output vec
        for chunk in buf.chunks(2) {
            let bytes = [chunk[0], chunk[1]];
            output.push(i16::from_le_bytes(bytes)); 
        }

        // Write only the bytes we were able to fill the buffer with.
        io.writei(&output[..n/2]);
    }
    // In case the buffer was larger than 2 seconds, start the stream manually.
    if pcm.state() != State::Running { pcm.start().unwrap() };

    // Wait for the stream to finish playback.
    pcm.drain().unwrap();
    Ok(())
}



































































/*
use alsa::pcm::{Access, Format, HwParams, PCM};
use std::f32::consts::PI;
use std::{thread, time::Duration};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 44100;
    let freq = 440.0; // A4 note
    let duration_secs = 5;
    let channels = 2;

    // Open default PCM device
    let pcm = PCM::new("default", alsa::Direction::Playback, false)?;

    // Hardware parameters
    let hwp = HwParams::any(&pcm)?;
    hwp.set_channels(channels)?;
    hwp.set_rate(sample_rate, alsa::ValueOr::Nearest)?;
    hwp.set_format(Format::s16())?;
    hwp.set_access(Access::RWInterleaved)?;
    pcm.hw_params(&hwp)?;

    let io = pcm.io_i16()?;
    let num_samples = sample_rate * duration_secs;
    let mut buffer = Vec::with_capacity((num_samples * channels) as usize);

    // Generate sine wave
    for t in 0..num_samples {
        let sample = (i16::MAX as f32 * (2.0 * PI * freq * t as f32 / sample_rate as f32).sin()) as i16;
        for _ in 0..channels {
            buffer.push(sample);
        }
    }

    // Write the buffer in chunks (in case it's too big for one write)
    let mut offset = 0;
    while offset < buffer.len() {
        let written = io.writei(&buffer[offset..])?;
        offset += written;
    }

    pcm.drain()?; // Ensure all samples are played
    Ok(())
}
*/




// use alsa::{Direction, ValueOr};
// use alsa::pcm::{PCM, HwParams, Format, Access, State};
//
// // Open default playback device
// let pcm = PCM::new("default", Direction::Playback, false).unwrap();
//
// // Set hardware parameters: 44100 Hz / Mono / 16 bit
// let hwp = HwParams::any(&pcm).unwrap();
// hwp.set_channels(1).unwrap();
// hwp.set_rate(44100, ValueOr::Nearest).unwrap();
// hwp.set_format(Format::s16()).unwrap();
// hwp.set_access(Access::RWInterleaved).unwrap();
// pcm.hw_params(&hwp).unwrap();
// let io = pcm.io_i16().unwrap();
//
// // Make sure we don't start the stream too early
// let hwp = pcm.hw_params_current().unwrap();
// let swp = pcm.sw_params_current().unwrap();
// swp.set_start_threshold(hwp.get_buffer_size().unwrap()).unwrap();
// pcm.sw_params(&swp).unwrap();
//
// // Make a sine wave
// let mut buf = [0i16; 1024];
// for (i, a) in buf.iter_mut().enumerate() {
//     *a = ((i as f32 * 2.0 * ::std::f32::consts::PI / 128.0).sin() * 8192.0) as i16
// }
//
// // Play it back for 2 seconds.
// for _ in 0..2*44100/1024 {
//     assert_eq!(io.writei(&buf[..]).unwrap(), 1024);
// }
//
// // In case the buffer was larger than 2 seconds, start the stream manually.
// if pcm.state() != State::Running { pcm.start().unwrap() };
// // Wait for the stream to finish playback.
// pcm.drain().unwrap();