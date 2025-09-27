use std::fs::{self, File};
use std::io::{self, Error, ErrorKind, Read, Seek};

use alsa::{Direction, ValueOr};
use alsa::pcm::{Access, Format, HwParams, State, PCM, IO};


use std::time::Duration;
use std::thread::sleep;


use std::sync::{Arc, Mutex};
use std::thread;

mod wavfile;

use crate::wavfile::WavFile;

#[derive(PartialEq)]
enum Action {
    Quit,
    None,
    Play,
    Pause,
    Previous,
    Rewind,
    Skip
}




/// 
/// This is the final version of main. It works, but it has some problems that I address where they arise
/// 
fn main() -> io::Result<()> { 

    let files: Vec<WavFile> = get_wav_files("tracks");
    
                //println!("{:#?}", files);

    // This creates a piece of memory that can be shared between threads, but only updated
    // mutually exclusively
    let state_command = Arc::new(Mutex::new(Action::None));

    // Each threads needs its own copy though, so we clone it to make it personal for each thread
    let state_player = Arc::clone(&state_command);

    // Spawn the threads for taking commands and playing back music. The move keyword will transfer
    // ownership over the closure's variables and move them to thread_command and thread_player
    let thread_command = thread::spawn(move || command_thread(state_command));
    let thread_player = thread::spawn(move || player_thread(state_player, &files));




    // Join the threads when we want to quit the program
    thread_command.join().unwrap();
    thread_player.join().unwrap();




    //    let mut input = String::new();
    //    io::stdin().read_line(&mut input)?;

    //    println!("{input}");

    Ok(())
}

// This is the thread that is reasponsible for handling actions that is relevant to the tracks to be played.
// It first creates an ALSA pcm object, sets the hardware parameters properly, then gives us an IO object
// for playback. 
//
// Playlists are fully functional, with the ability to play the last song, skip tacks and rewind.
// This functionality is achieved by reading the current action that is set in the mutex variable
// and then handled accordingly.
//
// This current version works, but has a major flaw that I do not have time to fix as of writing this.
// The problem is that the buffer that the 
fn player_thread (shared_state: Arc<Mutex<Action>>, tracks: &Vec<WavFile>) {
    
    // NOTE: none of ALSA's functions implements '?' so we need to use unwrap instead.
    let pcm = PCM::new("default", Direction::Playback, false).unwrap();

    let mut index = 0;
    let mut file = File::open(&tracks[index].path).unwrap();

    let io = set_pcm_params(&pcm, &tracks[index]);

    // file.read expects a u8 slice as a buffer, but the actual samples are i16. To get around this,
    // we will convert the data later. The size of the buffer is very arbitrary, 2 to some power is
    // recommended. Too large buffers might result in lag, and too small buffers may result in unnecessarily
    // large CPU loads. 8192 bytes is 4096 samples per buffer, which is a reasonable amount
    let mut buf = [0u8; 8192];

    print_meta_information(&tracks[index]);

    loop {

        // we introduce "fake latency" to give the other thread some room to take control over the mutex
        sleep(Duration::from_millis(40));

        let mut state =  shared_state.lock().unwrap();

        

        match *state {
            Action::Quit => {
                println!("Quitting player...");
                break;
            },
            Action::None => {
                //println!("1");
                // Default value
            },
            Action::Play => {
                // I had some issues that I did not have time to fix.
                // 
                drop(state);
                match file.read(&mut buf) {
                    Ok(n) => {

                        //if n == 0 { break; }
            
                        println!("Read {n} bytes");
                        // initialize to half the length, as 2x u8 = 1x i16
                        let mut output = Vec::with_capacity(buf.len() / 2);
            
                        // read all bytes and push them to the output vec
                        for chunk in buf.chunks(2) {
                            let bytes = [chunk[0], chunk[1]];
                            output.push(i16::from_le_bytes(bytes)); 
                        }
                    
                        // Write only the bytes we were able to fill the buffer with.
                        match io.writei(&output[..n / 2]){
                            Ok(_) => {},
                            Err(e) => {
                                eprintln!("ALSA write error: {:?}", e);
                            }
                        }

                    },
                    Err(error) => {
                        println!("error on read file: {error}");
                    }
                }
                
            },
            Action::Pause => {
                //println!("Pausing");
                // We don't have to do anything when we pause as this will
                // simply just not play or advance the files position.
            },
            Action::Rewind => {
                // SET SONG POS TO HEADER'S DATA START
                match file.seek(io::SeekFrom::Start(tracks[index].data_start)) {
                    Ok(_) => {
                        println!("rewind successful");
                        *state = Action::Play;
                    },
                    Err(_) => {
                        println!("Rewind failed");
                        break;
                    }
                }
            },
            Action::Previous => {
                // SET TRACK INDEX -
                // Wraps around to the last track if we tried to do skip back before the first one
                index = (tracks.len() + index - 1) % tracks.len();
                match File::open(&tracks[index].path) {
                    Ok(f) => {
                        file = f;
                        *state = Action::Play;
                        //println!("previous successful");
                        print_meta_information(&tracks[index]);
                    },
                    Err(_) => {
                        println!("Previous failed");
                        break;
                    }
                }
            },
            Action::Skip => {
                // SET TRACK INDEX +1
                // Wraps around to the first track if it tried to skip beyond the last one
                index = (index + 1) % tracks.len();
                match File::open(&tracks[index].path) {
                    Ok(f) => {
                        file = f;
                        *state = Action::Play;
                        //println!("skip successful")
                        print_meta_information(&tracks[index]);
                    },
                    Err(_) => {
                        println!("Skip failed");
                        break;
                    }
                }
            }
        }
    }
    // Wait for the stream to finish playback.
    pcm.drain().unwrap();
}



fn command_thread (shared_state: Arc<Mutex<Action>>) {
    let mut input = String::new();
    
    loop {
        
        input.clear();

        
        
        // We need to handle the case where read_line fails, as just unwrapping will cause a panic 
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                
                let mut state = shared_state.lock().unwrap();
                match input.to_ascii_uppercase().trim_ascii() {
                    "Q" => {*state = Action::Quit},
                    "P" => {if *state == Action::Pause {*state = Action::Play} else {*state = Action::Pause}},
                    "R" => {*state = Action::Rewind},
                    "V" => {*state = Action::Previous},
                    "S" => {*state = Action::Skip},
                    _ => {}
                }
                
                // Exit thread if we want to quit
                if *state == Action::Quit {
                    break;
                }
            },
            Err(_) => {}
        }

        //println!("input: '{input}'");
        //input = String::new();
    }
}


fn get_wav_files(folder: &str) -> Vec<WavFile> {
    let paths: Vec<String> = 
    fs::read_dir(folder)
        // makes directory entries iterable
        .into_iter() 
        // we need to filter out the Err path of Result before we can continue
        .map(|entries| entries.filter_map(Result::ok)) 
        // map wraps the result in another iterator (Iterator<Iterator<ReadDir>>) so we flatten to just
        // operate on the inner iterator (i am aware of flat_map() that does both in a single step but
        // i wanted to show the whole process)
        .flatten()
        // we want to get the paths ...
        .map(|entry| entry.path())
        // ... and filter out all but .wav files:
        .filter(|path| {
            // get the extension. this is an &OsStr wrapped in an option which we need to extract to &str ...
            path.extension()
                // ... so we do just that:
                .and_then(|ext| ext.to_str())
                // .WAV is just as valid as .wav or .wAv, so we ignore casing and compare to "wav"
                .map(|ex| ex.eq_ignore_ascii_case("wav")) == Some(true)
        })
        // finally, convert the PathBuf file paths into String and collect them into a String vec
        .map(|file| file.to_string_lossy().into_owned())
        .collect();

    // My previous version was bad, because i handled errors with WavFile::new().ok.unwrap(),
    // which may cause the program to panic on an error. Instead, i consider the output of 
    // WavFile::new() (either OK or Err), by keeping good files and discarding bad ones. This 
    // allows me to handle errors more gracefully than before, only pushing files that didn't 
    // somehow fail :)
    let mut files: Vec<WavFile> = vec![];
    for path in paths {

        let maybe_file = WavFile::new(path.as_str());
        match maybe_file {
            Ok(file) => files.push(file), // keep good files that didn't fail
            Err(_) => {} // discard bad files that failed
        }
    }
    files
}


fn set_pcm_params<'a> (pcm: &'a PCM, wav: &'a WavFile) -> IO<'a, i16>{
    
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

    io
}


fn print_meta_information(header: &WavFile) {
    
    let track_length = ((header.data_size as u64 - header.data_start) / 
                        (header.channels as u64 * (header.bit_depth as u64 / 8))) / header.sample_rate as u64;

    let mins = track_length / 60;
    let secs = track_length % 60;

    
    println!("\n\n\nTitle:  {}", header.title);
    println!("Artist: {}", header.artist);
    println!("Album:  {}", header.album);

    println!("\n-{}:{}{}-", mins, if secs < 10 {"0"} else {""}, secs);

    println!("\nActions:");
    println!("P - Play/Pause  |  S - Skip  | V - Previous  |  R - Rewind  |  Q - Quit");

}