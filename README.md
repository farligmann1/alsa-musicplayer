# alsa-musicplayer
This is a side project using a lot of programming concepts we learnt at uni. This project was made and intended to run on a Raspberry Pi. It is a music player made in C++ using the ALSA (Linux only) library that gives control over the sound card.

This project started in June 2024 (summer break) with researching, and had its last changes made during christmas break the same year. For a project in one of our courses during the spring semester in 2025, I remade the project in Rust while also looking into playback of ```.mp3``` files.

*Both versions of the project are unfinished, but usable*

The main goal of this project was to get a deeper understanding of how music is played back digitally, and to touch on low level concepts like handling audio samples manually, using multithreading to be able to read console inputs without freezing the main program, and actually getting to understand how an audio file works.

Originally, I wanted to make my own driver to get an even deeper understanding of how many of these concepts works, but I realized quickly that this would be way out of scope for what my goals was at the time.
