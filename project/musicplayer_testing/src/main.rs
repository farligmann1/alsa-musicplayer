use std::fs::{self, File};
use std::io::{self, Error, ErrorKind, Read, Seek};

use std::sync::{Arc, Mutex};
use std::thread;


/// 
/// Struct containing the most relevant header data
/// 
/// Sources:
/// - https://en.wikipedia.org/wiki/WAV#WAV_file_header
/// - https://docs.fileformat.com/audio/wav/#wav-file-header
#[derive(Debug)]
struct WavFile {
                                // *** Byte # ***||*** Desc. ***
    riff: String,               //      0-3         "RIFF" - indicates that file uses RIFF format
    file_size: u32,             //      4-7         File size minus 8 bytes (bytes 0-7)
    file_format: String,        //      8-12        File format - "WAVE" if .wav
    
    format_id: String,          //      13-16       "fmt " - indicates start of metadata chunk     
    chunk_size: u32,            //      17-20       Chunk size minus 8 bytes (bytes 13-20)
    fmt: u16,                   //      21-22       Audio format - most commonly 0x0001 for PCM
    channels: u16,              //      23-24       # of audio channels - 1 = mono, 2 = stereo, etc.
    sample_rate: u32,           //      25-28       Sample rate (hz), how many samples per second 
    byte_per_sec: u32,          //      29-32       Bytes to read per second (sample_rate * byte_per_block)
    byte_per_block: u16,        //      33-34       Bytes per sample frame (channels * bit_depth / 8)
    bit_depth: u16,             //      35-36       Bit depth, amount of bits used to represent a single sample

    data_block_id: String,      //      37-40       Usually "data", otherwise "INFO" - start of data/INFO chunk
    data_size: u32,             //      41-44       Size of data section

    artist: String,     // Bonus: Artist(s) pulled from INFO tag
    album: String,      // Bonus: Album pulled from INFO tag

    path: String,
    data_start: u64
}

impl WavFile {

    /// 
    /// WavFile constructor that reads the header of the wave file given at  and fills in the str
    fn new(path: &str) -> io::Result<WavFile> {
        let mut file = File::open(path)?;

        // We are going to read both 2 bytes and 4 bytes at a time, so we declare 2 u8 slices
        // that will hold the data we read
        let mut u16buf: [u8; 2] = [0; 2];  // 2 byte big buffer
        let mut u32buf: [u8; 4] = [0; 4];  // 4 byte big buffer

        // These are all helper functions that reads the content of the buffer and returns the content
        // parsed into their respective data type. I made these to avoid this pattern (excerpt from an
        // early version):
        /*
            file.read_exact(&mut u16buf)?;
            let channels = u16::from_le_bytes(u16buf);
            file.read_exact(&mut u32buf)?;
            let sample_rate = u32::from_le_bytes(u32buf);
            file.read_exact(&mut u16buf)?;
            let byte_per_sec = u32::from_le_bytes(u32buf);
            file.read_exact(&mut u16buf)?;
            let byte_per_block = u16::from_le_bytes(u16buf);
            file.read_exact(&mut u16buf)?;
            let bit_depth = u16::from_le_bytes(u16buf);
        */
        // The string version also removes all \0 from the string
        let buf16_as_uint = |x: &mut [u8]| u16::from_le_bytes([x[0],x[1]]);
        let buf32_as_uint = |x: &mut [u8]| u32::from_le_bytes([x[0],x[1],x[2],x[3]]);
        let buf_as_string = |x: &mut [u8]| String::from_utf8_lossy(x).to_string().replace("\0", "");

        // *********************
        // * MASTER RIFF CHUNK *
        // *********************
        // The first 4 bytes should be contain RIFF. https://en.wikipedia.org/wiki/WAV#RIFF
        let riff = read_into_as(&mut u32buf, &mut file, buf_as_string)?;
        if riff != "RIFF" {
            return Err(Error::new(ErrorKind::Unsupported, "Expected .wav file to have RIFF tag"))
        }
        let file_size = read_into_as(&mut u32buf, &mut file, buf32_as_uint)?;
        let file_format = read_into_as(&mut u32buf, &mut file, buf_as_string)?;
        // We are also working with wav files, so we should also do this 
        if file_format != "WAVE" {
            return Err(Error::new(ErrorKind::Unsupported, "Expected .wav file to have WAVE tag"))
        }

        //  ********************************
        //  * CHUNK DESCRIBING DATA FORMAT *
        //  ********************************
        let format_id = read_into_as(&mut u32buf, &mut file, buf_as_string)?;
        let chunk_size= read_into_as(&mut u32buf, &mut file, buf32_as_uint)?;
        let fmt = read_into_as(&mut u16buf, &mut file, buf16_as_uint)?;
        let channels = read_into_as(&mut u16buf, &mut file,buf16_as_uint)?;
        let sample_rate = read_into_as(&mut u32buf, &mut file, buf32_as_uint)?;
        let byte_per_sec = read_into_as(&mut u32buf, &mut file, buf32_as_uint)?;
        let byte_per_block = read_into_as(&mut u16buf, &mut file, buf16_as_uint)?;
        let bit_depth = read_into_as(&mut u16buf, &mut file, buf16_as_uint)?;

        // *********************************
        // * CHUNK CONTAINING SAMPLED DATA *
        // *********************************
        let data_block_id: String;
        let data_size: u32;

        let mut artist = String::from("Unknown Artist");
        let mut album = String::from("-");

        //let mut subchunk_format = String::new();

        loop {
            let chunk_id = read_into_as(&mut u32buf, &mut file, buf_as_string)?;
            let mut chunk_size = read_into_as(&mut u32buf, &mut file, buf32_as_uint)?;
            
            // RIFF adds an extra padding byte to the chunk if the chunk size is odd.
            // We account for this by reading an extra byte if it is odd.
            chunk_size = chunk_size + (chunk_size % 2);


            match chunk_id.as_str() {
                // If we encounter "data", it means we are on the start of the actual data 
                // part of the track
                "data" => {
                    data_block_id = chunk_id;
                    data_size = chunk_size;
                    break;
                },
                // "LIST" indicates that there will be additional metadata. We only care about
                // it if it follows the "INFO" format. See https://exiftool.org/TagNames/RIFF.html#Info
                // for a list of all tags
                "LIST" => {

                    let subchunk_id = read_into_as(&mut u32buf, &mut file, buf_as_string)?;

                    match subchunk_id.as_str() {
                        // "LIST" will be immediately followed by "INFO" (or other formats). INFO
                        // does not have a chunk size associated with it, so we just do nothing if
                        // we encounter it.
                        "INFO" => {
                            // If we wanted to support the "exif" format, we would store
                            // the subchunk format as well. We only care about "INFO"
                            /* subchunk_format = subchunk_id; */
                        },
                        // If there is an unsupported format, skip the entire "LIST" chunk as it 
                        // will just contain garbage. We have to account for the fact that we just
                        // read 4 bytes to get this tag, so we subtract 4 from the chunk size.
                        _ => {
                            /* subchunk_format = subchunk_id; */
                            file.seek(io::SeekFrom::Current(chunk_size as i64 - 4))?;
                        }
                    }
                },
                // IART is the artist(s)
                "IART" => {
                    let mut varbuf = vec![0u8; chunk_size as usize];
                    artist = read_into_as(&mut varbuf, &mut file, buf_as_string)?;
                },
                // IPRD is the "Product" tag name, most commonly the album
                "IPRD" => {
                    let mut varbuf = vec![0u8; chunk_size as usize];
                    album = read_into_as(&mut varbuf, &mut file, buf_as_string)?;

                },
                // There are more tags that we could add support for, but 
                // artist and album are the only ones we care about for now. This
                // piece of code skips the chunk if it is an unknown field without
                // reading any buffers
                _ => {
                    file.seek(io::SeekFrom::Current(chunk_size as i64))?;
                }
            }
        }

        // We need to store where the start of the data section is so we can start the song from there,
        // because otherwise - if we started from the beginning of the file - we may sometimes be able 
        // to hear "random data" because we are then treating the header as audio data. We can do this 
        // by seeking 0 bytes from the current position in the file and storing the result since it will
        // then just be the current position. (kinda annoying that this is the best equivalent of C++'s file.tellg())
        let data_start = file.seek(io::SeekFrom::Current(0))?;
        
        Ok(Self {
            riff,
            file_size,
            file_format,
            format_id,
            chunk_size,
            fmt,
            channels,
            sample_rate,
            byte_per_sec,
            byte_per_block,
            bit_depth,
            data_block_id,
            data_size,
            artist,
            album,
            path: String::from(path),
            data_start
        })
    }

//    fn play(&self, start_pos: u64) -> u64 {
//    }
}

/// 
/// Generic function that reads to the buffer slice and performs 'func' on it, returning the result from 
/// func wrapped in an io::Result. If reading to the buffer fails, or an unexpected amount of bytes was 
/// read, an error will be returned instead.
/// 
/// Params:
/// - buf: &mut \[u8]            -  The buffer to be filled
/// - file: &mut File            -  The file that should be read to
/// - func: fn(&mut \[u8]) -> T  -  Function that takes a \[u8] and returns a T type
fn read_into_as<T>(buf: &mut [u8], file: &mut File, func: fn(&mut [u8]) -> T) -> io::Result<T> {
    let bytes_read = file.read(buf)?;
    if bytes_read != buf.len() {
        return Err(Error::new(ErrorKind::UnexpectedEof, "Unexpected early EOF"))
    }
    Ok(func(buf))
}



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
/// This is a test file for reading .wav headers into a struct
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
    // ownership over the 
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


fn player_thread (shared_state: Arc<Mutex<Action>>, tracks: &Vec<WavFile>) {
    
    let mut index = 0;
    let mut file = File::open(&tracks[index].path).unwrap();
    

    loop {
        let mut state =  shared_state.lock().unwrap();

        

        match *state {
            Action::Quit => {
                println!("Quitting player...");
                break;
            },
            Action::None => {
                //println!("1");
                // DO NOTHING
                // DEFAULT VALUE
            },
            Action::Play => {
                //println!("Playing");
                // KEEP FEEDING BUFFERS
            },
            Action::Pause => {
                //println!("Pausing");
                // STOP PLAYING TRACK
                // SAVE SONG POSITION
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
                // SET TRACK INDEX -1
                index = (tracks.len() + index - 1) % tracks.len();
                match File::open(&tracks[index].path) {
                    Ok(f) => {
                        file = f;
                        *state = Action::Play;
                        println!("previous successful")
                    },
                    Err(_) => {
                        println!("Previous failed");
                        break;
                    }
                }
            },
            Action::Skip => {
                // SET TRACK INDEX +1
                index = (index + 1) % tracks.len();
                match File::open(&tracks[index].path) {
                    Ok(f) => {
                        file = f;
                        *state = Action::Play;
                        println!("skip successful")
                    },
                    Err(_) => {
                        println!("Skip failed");
                        break;
                    }
                }
            }
        }
    }
}

fn command_thread (shared_state: Arc<Mutex<Action>>) {
    let mut input = String::new();
    
    loop {
        

        

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
        input = String::new();
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