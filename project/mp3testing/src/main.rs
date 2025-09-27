use std::{fs::File, io::{self, Error, ErrorKind, Read}};

fn main() -> io::Result<()>{
    let mut file = File::open("src/mp3test.mp3")?;
    

    
    let header = Mp3Header::new(&mut file)?;


    println!("{:#?}", header);


    

    Ok(())
}

#[derive(Debug)]
struct Mp3Header {
    version: String,
    layer: String,
    error_protection: bool,
    bit_rate: u32,
    sample_rate: u32,
    padding: bool,
    mode: String,
    mode_extension: u8,
    copyrighted: bool,
    original: bool,
}


impl Mp3Header{
    pub fn new (file: &mut File) -> io::Result<Self> {

        let mut buf = [0u8; 4];
        file.read_exact(&mut buf)?;
        
        let parsed_buf = u32::from_be_bytes(buf);
        
        println!("{:?} {:32b}",buf, parsed_buf);

        // BITS 0-10:
        // The first 11 bits are the sync word, and should ALWAYS be all 1s. We can check this 
        // by performing a binary AND on the first 11 bits and see if they are all 1s.
        let mut temp = parsed_buf >> 21; // first shift bits by 21 to get 11 most significant bits
        if temp & 0b111_11111111 != 0b111_11111111 {
            return Err(Error::new(ErrorKind::InvalidData, "Sync word is invalid."))   
        }



        // BITS 11-12:
        // The next 2 bits determine the version. I have let out MPEG2 and MPEG 2.5,
        // but their binary flags would be 0b10 and 0b00. 0b01 is reserved and invalid.
        temp = parsed_buf >> 19;
        let version = match temp & 0b11 {
            0b11 => String::from("MPEG 1"),
            //0b10 => String::from("MPEG 2"),
            //0b00 => String::from("MPEG 2.5"),
            _ => return Err(Error::new(ErrorKind::InvalidData, "Unsupported format"))
        };

        println!("temp: {}", temp & 0b11);
        println!("version: {}", version);



        // BITS 13-14:
        // The next 2 bits determine layer. Layer III is standard for mp3. mp3 is literally
        // short for MPEG 1 layer III, so if we have a .mp3 file this should always be 0b01
        temp = parsed_buf >> 17;
        let layer = match temp & 0b11 {
            //0b11 => String::from("I"),
            //0b10 => String::from("II"),
            0b01 => String::from("III"),
            _ => return Err(Error::new(ErrorKind::InvalidData, "Unsupported layer type"))
        };
        

        println!("temp: {}", temp & 0b11);
        println!("version: {}", layer);


        // BIT 15:
        // This is a bit flag that indicates if we should have a 2-byte CRC checksum right 
        // after the header. CRC is short for Cyclic Redundancy Check and is used for data
        // redundancy. 0 means there IS CRC, 1 means there is NO CRC.
        temp = parsed_buf >> 16;
        let error_protection = temp & 0b1 == 1;

        println!("temp: {}", temp & 0b1);
        println!("error protection: {}", if error_protection {"no"} else {"yes"});
        
        
        
        // BIT 16-19:
        // This is a lookup table that indicates bitrates, based on the 4 bits, as well 
        // as the version and layer used.
        temp = parsed_buf >> 12;
        let bit_rate: u32 = match (&version as &str, &layer as &str) {
            // ("MPEG 1", "I") => ...,
            // ("MPEG 1", "II") => ...,
            ("MPEG 1", "III") => {
                match temp & 0b1111 {
                    0b0001 => 32_000,
                    0b0010 => 40_000,
                    0b0011 => 48_000,
                    0b0100 => 56_000,
                    0b0101 => 64_000,
                    0b0110 => 80_000,
                    0b0111 => 96_000,
                    0b1000 => 112_000,
                    0b1001 => 128_000,
                    0b1010 => 160_000,
                    0b1011 => 192_000,
                    0b1100 => 224_000,
                    0b1101 => 256_000,
                    0b1110 => 320_000,
                    _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid bitrate"))
                }
            },
            _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid bitrate"))
            // ("MPEG 2", "I), ("MPEG 2.5", "I") => ...,
            // ("MPEG 2", "I), ("MPEG 2.5", "II"), ("MPEG 2", "I), ("MPEG 2.5", "III") => ...,
        };


        // BITS 20-21:
        // Used to dermine sample rate in a similar manner as the bitrate, but this time
        // with just the version as the determinant for what the sample rate should be
        temp = parsed_buf >> 10;
        let sample_rate: u32 = match &version as &str {
            "MPEG 1" => {
                match temp & 0b11 {
                    0b00 => 41_000,
                    0b01 => 48_000,
                    0b10 => 32_000,
                    _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid bitrate"))
                }
            },
            _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid bitrate"))
            // "MPEG 2" => ...,
            // "MPEG 2.5" => ...,
        };

        println!("temp: {:2b}", temp & 0b11);
        println!("sample rate: {}", sample_rate);


        // BIT 22:
        // Padding bit. The length of a frame is determined by 
        // frame_size = (144 × bit_rate) / sample_rate + padding,
        // where we sometimes have to pad when the math for working out size doesn't exactly
        // add up
        temp = parsed_buf >> 9;
        let padding = temp & 0b1 == 1; 


        println!("temp: {}", temp & 0b1);
        println!("padding: {}", padding);


        // BIT 23:
        // Decoders do not care about this bit, as does not affect playback.

        // BIT 24-25:
        // These bits detemines the mode of which the audio is played back.
        // Possible values are:
        // - Stereo:        left and right channels are completely independent
        // - Joint Stereo:  a more optimized version of stereo that "averages" left and right channels
        // - Dual Channel:  2 mono tracks, not common in music but can be used for specialized applications
        // - Mono:          left and right channels play the same audio
        temp = parsed_buf >> 6;
        let mode = match temp & 0b11 {
            0b00 => String::from("Stereo"),
            0b01 => String::from("Joint Stereo"),
            0b10 => String::from("Dual Channel"),
            0b11 => String::from("Mono"),
            _ => return Err(Error::new(ErrorKind::Other, "Something went wrong"))
        };


        println!("temp: {}", temp & 0b11);
        println!("mode: {}", mode);

        // BITS 26-27
        // Only used with joint stereo, therefore not parsed here.
        let mode_extension: u8 = ((parsed_buf >> 4) & 0b11) as u8;

        println!("JS extension: {}", mode_extension);

        // BIT 28
        // Copyright bit, 1 if the track is copyrighted, 0 if not.
        let copyrighted = (parsed_buf >> 3) & 0b1 == 1;

        println!("Copyrighted: {}", copyrighted);

        // BIT 29
        // Original bit, 1 if this is original or 0 if it is a copy/duplicate media
        let original = (parsed_buf >> 2) & 0b1 == 1;

        println!("original: {}", original);

        // BITS 30-31
        // Typcally 00, as this is mostly obsolete on modern players.
        // These are the emphasis bits, which uses a technique to boost high frequencies
        // before encoding and then de-emphasize them during playback.
        let emphasis = parsed_buf & 0b11;
        if emphasis != 0 {
            return Err(Error::new(ErrorKind::Other, "Emphasis is not supported"))
        }

        
        Ok(Self{
            version, 
            layer, 
            error_protection, 
            bit_rate, sample_rate, 
            padding, 
            mode, 
            mode_extension, 
            copyrighted,
            original
        })
    }
}