# Project Documentation

## Table of contents

- [User guide](#user-guide)
- [File structure](#file-structure)
- [Introduction](#introduction)
- [Cool, but how is music even represented digitally?](#cool-but-how-is-music-even-represented-digitally)
- [So, how does a .wav file work exactly?](#so-how-does-a-wav-file-work-exactly)
- [What about .mp3 files?](#what-about-mp3-files)
- [Change in focus](#change-in-focus)
- [.wav player 2.0](#wav-player-20)
- [Issues under development of .wav player 2.0](#issues-under-development-of-wav-player-20)
- [Use of LLMs](#use-of-llms)
- [Conclusion and what I have learnt](#conclusion-and-what-i-have-learnt)
- [Links](#links)


## User guide

***IMPORTANT!***

This project *only* compiles on Linux. This is because I use a Linux only crate called ALSA. I suggest deploying on a Raspberry Pi if you have one available.

The `main.rs` file that runs my program is located under `.\musicplayer\src\`. This means that to run it, you have to `cd` to `.\musicplayer\`. 

Files to be played should be put in the `.\musicplayer\tracks` folder. I do not want to put them in my repository in fear of copyright infringement. Make sure that the files are `.wav`!

There are several commands that you can use to manipulate the playlist:

> **P** - Toggles Play/Pause  
> **S** - Skips the current track and plays the next one in the playlist. If the current track was the last one, it loops to the first.  
> **V** - Plays the previous track on the playlist similarly to *skip*. If this was the first track on the playlist, it loops to the last.  
> **R** - Rewinds the current track to the beginning.  
> **Q** - Exits the program


## File structure

My project's file structure works the following way:

1. `.\mp3testing`: files for testing reading of `.mp3` files. 
2. `.\musicplayer`: the final product
3. `.\musicplayer_testing`: files for testing reading of `.wav` headers

I chose to split my files this way to make it clear what I  have done or tried to do, while still keeping the working final product seperate. Otherwise, things might become messy quick.

## Introduction
I wanted to make a music player in Rust using the ALSA crate. The reason I even want to do this project, is because I want to get a deeper understanding of how audio files are stored and played back, explore some different file formats and reasearch how they work, and try my best to make a playback mechanism for some formats I find interesting, implementing it as manually and barebones as possible. I will also take this opportunity to get better at Rust.

ALSA is short for Advanced Linux Sound Architecture, and allows for handling PCM (Pulse Code Modulation) audio output by sending buffers with data to the sound card. As the name suggests, this is a **Linux only crate**, so running 'cargo run' on a non-Linux system will most likely not even build.

Fortunately, since I have a Raspberry Pi 4B available, I will use it actively to test my code. However, a major caveat is that it is terrible to write code on as it is very slow. I will instead write code on my laptop instead and use SCP (Secure Copy Protocol) to transfer files to the RPi with a command in tis format:

```bash
scp -r .\musicplayer\ pi@<ip>:<path>
```


## Cool, but how is music even represented digitally?


The answer depends on the file format it is stored as. As an example, `.mp3` uses an entirely different method of storing data than `.ogg` files.  

There are many audio file formats, with some of the most common being `.mp3`, `.ogg`, `.flac` and `.wav`.

In this project i will focus mostly on `.wav` files.


## So, how does a .wav file work exactly?


`.wav` files use a concept called Pulse-Code Modulation (PCM) to approximate analog audio signals digitally. 

The concept is relatively simple: since it is physically impossible to represent the entire signal without some loss of data, we instead sample the signal at regular intervals. 

PCM streams use two main variables that determines the fidelity/quality of the signal: sampling rate (samples per second) and bit depth (# of bits that can be used to represent a signal. 16 is common).
Source: https://en.wikipedia.org/wiki/Pulse-code_modulation#

To correctly play back a `.wav` file, we first have to read the header of the file. The header contains some metadata about how the file should be played back, with some additional optional data about the content. Some examples of additional data that may be stored in the header is *artist*, *album*, *release year*, *production studio* and *genre*. For a full list of RIFF info tags, see https://exiftool.org/TagNames/RIFF.html#Info. For this project I will only include *artist* and *album* as they are the most relevant for a regular user.

Below is a schema of how 
```
    BYTE #                            DESCRIPTION
    0-3             "RIFF" - indicates that file uses RIFF format
    4-7             File size minus 8 bytes (bytes 0-7)
    8-12            File format - "WAVE" if .wav
    
    13-16           "fmt " - indicates start of metadata chunk     
    17-20           Chunk size minus 8 bytes (bytes 13-20)
    21-22           Audio format - most commonly 0x0001 for PCM
    23-24           # of audio channels - 1 = mono, 2 = stereo, etc.
    25-28           Sample rate (hz), how many samples per second 
    29-32           Bytes to read per second (sample_rate * byte_per_block)
    33-34           Bytes per sample frame (channels * bit_depth / 8)
    35-36           Bit depth, amount of bits used to represent a single sample

    37-40           Usually "data", otherwise "INFO" - start of data/INFO chunk
    41-44           Size of data section

    title: String       // Bonus: Title pulled from INFO tag
    artist: String,     // Bonus: Artist(s) pulled from INFO tag
    album: String       // Bonus: Album pulled from INFO tag

```

## What about .mp3 files?

`.mp3` files is a whole other story. They use an encoding algorithm that lets file sizes be between 75-95% smaller than raw `.wav` files.

This is achieved with some clever use of Huffman coding, 

This is a flow chart of the decoder pipeline from the ISO/IEC 11172-3 standard that contains most of the information about mp3 codecs:

> GET BIT STREAM, FIND HEADER

The `.mp3` file is read byte by byte to look for the sync word in the header (the first 11 bits). A header in an `.mp3` file is 4 byte, and contains information about the coming data section.

> DECODE SIDE INFORMATION

This section is mostly to direct the decoder through the many huffman tables.


> DECODE SCALE FACTORS

Uses psychoacoustics to split the frequency spectrum into critical bands. The `.mp3` encoding takes advantage of the fact that not all parts of the frequency spectrum is audible or barely audible, which allows us to save space by not representing them. 

> DECODE HUFFMAN DATA

The encoding uses predefined, standardized  Huffman tables that allows us to represent data as variable-length bit codes and in turn save space.

> REQUANTIZE SPECTRUM

The decoder tries to reconstruct the sound frquencies and their magnitude/loudness

> REORDER SPECTRUM IF (window-switching-flag) AND (block type==2)
 
> JOINT STEREO PROCESSING (if applicable)
 
> ALIAS REDUCTION
 
> SYNTHESIZE VIA IMDCT
 
> SYNTHESIZE VIA IMDCT & OVERLAP-ADD

> (IMDCT either 18 or 6,6,6 depending on window-switching-flag and block-type )

> SYNTHESIZE VIA POLYPHASE FILTERBANK

> OUTPUT PCM SAMPLES 


Some of the explanations for the `.mp3` header is left intentionally empty. See the next chapter for an explanation.

The source for all of this is the ISO/IEC 11172-3 standard, which can be found via this link: https://www.iso.org/standard/22412.html

## Change in focus

I have made a decision. 

My original plan when I started this project, was to first get down PCM playback by implementing a `.wav` reader/player, then attempt to tackle the task of diving into `.mp3` reading, even though I knew it was going to be tough. 

As of writing this (14.05), I have made a program that is able to read `.mp3` headers. This was extremely tedious and repetetive, as I had to research and write about each of the 32 bits and then implement it in code, which was mostly either a copy-paste of what I had already done earlier in the header with bitshifts and logical ANDing, or creating a lookup table with nested match statements plus more logical ANDing.

If I were to continue with working with `.mp3`, I know I would have to deal with a lot more lookup tables, which just means more hard coded values and match statements. It is not that programmatically interesting nor fun to implement. The Huffman coding that `.mp3` files use is a 32 page table, some of which have almost 200 entries. 

Instead, I will try my best with the time I have left to make a more fully scaled player. The plan is to read all files in a folder that will be added to a "playlist", be able to take commands from the console by reading a character via io::stdin(), and then doing something to the playlist and play tracks based on what the command is.

## .wav player 2.0

In the last chapter I briefly explained what I wanted to do to expand on the `.wav` player I already have, by adding a bunch of features to make it more like an actual music player. This will also be my main focus until the end of the assignment.

I want to focus on taking commands from the user, then do something about how the song is played based on what the command is. 

This would be easy enough if it weren't for the fact that taking input from the user through the console (io::stdin) blocks the *current thread* and waits for user input before the thread continues.

To solve this, we will use threading to separate playback and and taking commands. The command thread will only block itself, so playback will still be possible! This is also a great opportunity to dive into similar areas we learnt about in operating systems.

For threading, I used `std::sync::Arc`, `std::sync::Mutex` and `std::thread`. Threading in Rust is a bit different from threading in C or C++. In C/C++ we would use a global mutex variable, but Rust does not allow us to have multiple mutable references to the same variable as that could cause data races. Instead, we have these two lines:
```rust
let state_command = Arc::new(Mutex::new(Action::None));
let state_player = Arc::clone(&state_command);
```
This will create a mutex that only one of the threads can use at a time, and it initalized to an `Action` of type `None`. `Action` is an enum I use to track what the current "mode" for the program is, e.g if the track is playing, paused, etc. `Action` may change rapidly. 


### Issues under development of .wav player 2.0

I had a really annoying issue that I unfortunately did not have time to properly come up with a solution for. When the "play" action is active, every time the `player_thread` thread gets its turn, the player reads up to 8196 bytes from the active file, and sends the read data to the sound card via the ALSA library. After this, the thread is done and it gives up its turn.

The problem is that when the data is sent to the sound card for playback, the player thread can keep running and send buffers, while the command thread is patiently waiting for ALSA to finish playing its buffers. It is a kind of "manual starvation problem" where one thread gets the majority of CPU time instead of fighting over resources. I fixed the issue by adding an artificial delay, or in other words, putting the player thread to sleep. By putting the player thread to sleep for 40 ms, the program works perfectly fine. This is because the command thread is now allowed to take the player threads turn while it is sleeping.There is probably a better solution, but it works for now at least.

There wasn't really any other major issues I faced programming wise, but something super annoying happened when I started importing ALSA to my project. Since it is a Linux only library and I use Windows to write code, VSCode decided that my intellisense was no good anymore. Writing code without any help when I get warnings or errors in my code, was a bit tedious, since I had to secure copy my files over to the Raspberry Pi and see errors with `cargo run`. Luckily, there usually wasn't that much to fix when I had issues.

## Use of LLMs

Almost everything in this project is made by hand by me. The code that is responsible for playing sound via ALSA is taken from their documentation, but it is not a direct copy and I have of course adapted it to work with my other code.

What I used LLMs the most for, was to broaden my understanding of very niche information, like what the thirteenth bit in an `.mp3` header is supposed to represent. I also used it to understand some of the code from the ALSA library, as its documentation was lackluster to say the least.

I have also used ChatGPT to help me get started on what information to look up, but I don't blindly trust what it gives me. I do my own research.

## Conclusion and what I have learnt

In this project I have learnt a lot. I know know:

- Multithreading in Rust
- How `.wav` files work and how you can parse them
- A lot about `.mp3` files, despite not managing to create my own player
- How to use and copy files to another machine via SCP and remote ssh
- More about chaining functions together

Overall, it has been a very fun experience, even though I didn't necessarily manage to pull off everything that I had planned to do. That was to be expected though, as I had done some research on `.mp3` files prior to starting this project, so I knew that it would take a lot of effort to decode them. If I knew before I started that i would have to manually implement the Huffman tables, I probably would have shifted my focus towards `.wav` or maybe other formats from the start, as it is not that programatically interesting. But oh well, it was a blast anyways!

## Links:

`.wav` header tags:
https://docs.fileformat.com/audio/wav/#wav-file-header

More about the `.wav` header:
https://en.wikipedia.org/wiki/WAV#WAV_file_header