#define ALSA_PCM_NEW_HW_PARAMS_API // Use the newer ALSA API

#include <alsa/asoundlib.h> // alsa functions
#include <iostream> // cout, cin
#include <fstream> // ifstream
#include <thread> // thread
#include <cstdlib> // system()
#include <vector> // vector
#include <string> // string
#include <mutex> // mutex, lock(), unlock()


using namespace std;

// action macros for readability in action handling
enum Action {
    QUIT         =-1,
    NONE         = 0,
    PAUSE        = 1,
    PLAY         = 2,
    SKIP         = 3,
    PREVIOUS     = 4,
    REWIND       = 5,
    END_OF_TRACK = 6
};





void display();
void displayHeader();
void displayMenu(bool truncated);
void initializeHardware(int &size, snd_pcm_t *&handle, snd_pcm_hw_params_t *&params,
                        int &dir, snd_pcm_uframes_t &frames, char *&buffer, int &err);
void initializeTracklist();
void playNextTrack(const enum Action act);
void readHeader(ifstream &file);


// mutual exclusion variables and the critical sections
mutex buf_mtx; // mutex for buffer
mutex mtx;     // mutex for action handling (main mutex)
bool isPlaying;
Action action;
bool quitFlag;

// threads
void actionHandler();
void getCommand();
void musicPlayer();

vector<string> tracklist;
unsigned int track_num = 0;
ifstream file;


// hardware variables
int size;
snd_pcm_t *handle;
snd_pcm_hw_params_t *params;
int dir;
snd_pcm_uframes_t frames;
char *buffer = nullptr;
int err;



/* WAVE FILE HEADER INNHOLD
   ST�TTER KUN PCM p.d.d    */
char chunk_id[5];

unsigned int chunk_size;
char format[5];

char subchunk1_id[4];
unsigned int subchunk1_size;
unsigned short int audio_format;
unsigned short int num_channels;
unsigned int sample_rate;
unsigned int byte_rate;
unsigned short int block_align;
unsigned short int bits_per_sample;
char subchunk2_id[4];
unsigned int subchunk2_size; // UNUSED / USELESS

char ekstra[4];


// track length variables
unsigned int track_pos, // position in song: how far into a song are we? (used when pausing)
             track_beg,
             track_end,
             track_length;

int main() {
    chunk_id[4] = '\0';
    format[4] = '\0';
    isPlaying = true;
    action = NONE;
    quitFlag = false;


    initializeTracklist();





    thread commandThread(&getCommand);
    commandThread.detach(); // The thread that takes commands should run independantly



    file.open(tracklist[track_num], ios::binary);  // open file




        // READ THE ENTIRE FILE HEADER
    readHeader(file); // reads all header info




    displayHeader();
    displayMenu(false);


    initializeHardware(size, handle, params, dir, frames, buffer, err);


    thread musicThread(&musicPlayer);
    musicThread.join();

    snd_pcm_drain(handle);
    snd_pcm_close(handle);
    free(buffer);
    file.close();

    return 0;
}



// ************************************************************************** //
//
// FUNCTIONS
//
// ************************************************************************** //

void actionHandler() {
    switch (action) {
        case QUIT: quitFlag = true; break;
        case NONE: break;
        case PAUSE: isPlaying = false; break;
        case PLAY: isPlaying = true; break;
        case SKIP: playNextTrack(SKIP); break;
        case PREVIOUS: playNextTrack(PREVIOUS); break;
        case REWIND: file.seekg(track_beg, ios::beg); break;
        case END_OF_TRACK: playNextTrack(SKIP); break;
    }
}



void display() {

}



void displayHeader() {
    cout << "\nchunk_id: " << chunk_id
         << "\nchunk_size: " << chunk_size
         << "\nformat: " << format
         << "\nsubchunk1_id: " << subchunk1_id
         << "\nsubchunk1_size: " << subchunk1_size
         << "\naudio_format: " << audio_format << (audio_format == 1 ? " (PCM)" : "")
         << "\nnum_channels: " << num_channels
         << "\nsample_rate: " << sample_rate
         << "\nbyte_rate: " << byte_rate
         << "\nblock_align: " << block_align
         << "\nbits_per_sample: " << bits_per_sample
                    << "\n\nekstra: " << ekstra << "\n"
         << "\nsubchunk2_id: " << subchunk2_id
         << "\nsubchunk2_size: " << subchunk2_size
         << "\n\ntrack length: " << track_length << "\n\n\n\n\n";
}


void displayMenu(bool truncated) {
    if (truncated) {
        cout << "\nP - Play/Pause   S - Skip   V - Previous   R - Rewind   Q - Quit\n";
    } else {
        cout << "\n AVALAIBLE ACTIONS:"
             << "\n\tP - Play/Pause"
             << "\n\tS - Skip"
             << "\n\tV - Previous"
             << "\n\tR - Rewind"
             << "\n\tQ - Quit"
             << "\n\n";
    }
}

void initializeHardware(int &size, snd_pcm_t *&handle, snd_pcm_hw_params_t *&params,
                        int &dir, snd_pcm_uframes_t &frames, char *&buffer, int &err) {
    /* Open PCM device for playback. */
    if ((err = snd_pcm_open(&handle, "default", SND_PCM_STREAM_PLAYBACK, 0)) < 0) {
        cout << "unable to open pcm device: " << snd_strerror(err) << "\n\n";
        exit(1);
    }


    snd_pcm_hw_params_alloca(&params); /* Allocate a hardware parameters object. */

    snd_pcm_hw_params_any(handle, params); /* Fill it in with default values. */

    /* Set the desired hardware parameters. */
    snd_pcm_hw_params_set_access(handle, params,    /* Interleaved mode */
                    SND_PCM_ACCESS_RW_INTERLEAVED);

    snd_pcm_hw_params_set_format(handle, params,  /* Signed 16-bit little-endian format */
                              SND_PCM_FORMAT_S16_LE);

    snd_pcm_hw_params_set_channels(handle, params, num_channels); /* Two channels (stereo) */

    snd_pcm_hw_params_set_rate_near(handle, params, /* 44100 bits/second sampling rate (CD quality) */
                                  &sample_rate, &dir);


    frames = 32;
    snd_pcm_hw_params_set_period_size_near(handle,  /* Set period size to 32 frames. */
                              params, &frames, &dir);

    err = snd_pcm_hw_params(handle, params); /* Write the parameters to the driver */

    if (err < 0) {
        cout << "unable to set hw parameters: " << snd_strerror(err) << '\n';
        exit(1);
    }

    size = frames * num_channels * (bits_per_sample / 8);
    buffer = (char *) malloc(size);
}

void initializeTracklist() {

    if (system(NULL)) {
        cout << "\n\nAttempting to retrieve .wav files from downloads folder...\n\n";

        system("ls ~/Downloads/*.wav > temp.dat"); // get all .wav files from downloads in a temporary .dat file

        string track; // temp string
        ifstream tracks;
        tracks.open("temp.dat");

        while(!tracks.eof()) {
            getline(tracks, track); // gets a path to a track and puts it in global tracklist
            tracklist.push_back(track);
            cout << '\n' << track;

            while(tracks.peek() == '\n') // removes all trailing \n
                tracks.ignore();
        }

        tracks.close();
        remove("temp.dat"); // delete temporary file
    } else
        cout << "\n\nError! Call to 'system()' failed.\n\n";
}


void playNextTrack(const enum Action act) {
    buf_mtx.lock();
    snd_pcm_drain(handle);
    snd_pcm_close(handle);
    file.close();
    free(buffer);

    if (act == SKIP)
        track_num = (track_num + 1) % tracklist.size();
    else if (act == PREVIOUS)
        track_num = (tracklist.size() + track_num - 1) % tracklist.size();

    file.open(tracklist[track_num]);
    readHeader(file);
    track_beg = file.tellg();

    initializeHardware(size, handle, params, dir, frames, buffer, err);

    buf_mtx.unlock();
}




void readHeader(ifstream & file) {
    char* album = nullptr,
        * artist = nullptr,
        * title = nullptr;






    file.read(chunk_id, 4);                     // "RIFF"
    file.read((char *) &chunk_size, 4);         // file size
    file.read(format, 4);                       // "WAVE"
    file.read(subchunk1_id, 4);                 // "fmt "
    file.read((char *) &subchunk1_size, 4);     // length of the rest of the header
    file.read((char *) &audio_format, 2);       // audio format - pcm is 1
    file.read((char *) &num_channels, 2);       // # of channels
    file.read((char *) &sample_rate, 4);        // sample rate
    file.read((char *) &byte_rate, 4);          // (Sample Rate * BitsPerSample * Channels) / 8.
    file.read((char *) &block_align, 2);        // (BitsPerSample * Channels) / 8
    file.read((char *) &bits_per_sample, 2);    // bit depth


    // reads extra data blocks if there are any
    file.read(ekstra, 4);
    if (strncmp(ekstra, "data", 4) != 0) {
        unsigned int sub_sub_chunk_size;
        file.read((char *) &sub_sub_chunk_size, 4);
        //cout << "\nSUB " << sub_sub_chunk_size;
        file.read(ekstra, 4); // INFO
        file.read(ekstra, 4);
        while (strncmp(ekstra, "data", 4) != 0) {
            if (strncmp(ekstra, "IART", 4) == 0) {
                unsigned int artist_size;
                file.read((char *) &artist_size, 4);

                artist = (char * ) malloc (sizeof(char) * artist_size);
                file.read(artist, artist_size);

            } else if (strncmp(ekstra, "INAM", 4) == 0) {
                unsigned int title_size;
                file.read((char *) &title_size, 4);

                title = (char *) malloc (sizeof(char) * title_size);
                file.read(title, title_size);

            } else if (strncmp(ekstra, "IPRD", 4) == 0) {
                unsigned int album_size;
                file.read((char *) &album_size, 4);

                album = (char *) malloc (sizeof(char) * album_size);
                file.read(album, album_size);

            } else {
                unsigned int ignore_size;
                file.read((char *) &ignore_size, 4);

                char ignore[ignore_size];
                file.read(ignore, ignore_size);
/*
                char temp[5];
                strcpy(temp, ekstra);
                temp[4] = '\0';

                cout << "\n\tIGNORE (" << temp << "): " << ignore;
*/
            }
            while (file.peek() == '\0')
                file.ignore();
            file.read(ekstra, 4);
        }
        cout << "\n\n";
    }

    file.read((char *) &subchunk2_size, 4);

    // figure out start, end, and length of the track (length in seconds)
    track_beg = file.tellg();
    file.seekg(0, ios::end);
    track_end = file.tellg();
    file.seekg(track_beg, ios::beg);
    track_length = ((track_end - track_beg) / (num_channels * (bits_per_sample / 8))) / sample_rate;

    cout << "\n\n\n\n\n\n\n\n\n\n"
         << "\nTITLE : " << (title ? title : "(unknown)")
         << "\nARTIST: " << (artist ? artist : "(unknown)")
         << (album ? "\nALBUM : " : "") << (album ? album : "")
         << "\n\n\t-" << track_length / 60 << ':' << track_length % 60 << "-\n\n";

    displayMenu(true);
    if (album)  free(album);
    if (artist) free(artist);
    if (title)  free(title);
}


// ******************************************************************'
 // thread functions




// QUEUE MAY BE IMPLEMENTED?




void getCommand() {
    char command;
    while(true) {
        cin >> command;
        command = toupper(command);

        mtx.lock();
        switch (command) {
            case 'P': action = (isPlaying ? PAUSE : PLAY); break;
            case 'S': action = SKIP; break;
            case 'V': action = PREVIOUS; break;
            case 'R': action = REWIND; break;
            case 'Q': action = QUIT; break;
            default:  action = NONE; break;
        }


        actionHandler();
        mtx.unlock();
    }
}

void musicPlayer() { // TODO
    while (!quitFlag) {
        while (isPlaying && !quitFlag) {           // read and play the whole file
            if (!file.eof()) {
                buf_mtx.lock();
                file.read(buffer, size);
                err = snd_pcm_writei(handle, buffer, frames);
                if (err == -EPIPE) {
                    /* EPIPE means underrun */
                    cout << "underrun occurred\n";
                    snd_pcm_prepare(handle);

                } else if (err < 0) {
                    cout << "error from writei: " << snd_strerror(err) << '\n';

                }  else if (err != (int)frames) {
                    cout << "short write, write " << err << " frames\n";
                }

                buf_mtx.unlock();

            } else {
                mtx.lock();
                action = END_OF_TRACK;
                actionHandler();
                mtx.unlock();
            }
        }
    }
}


// **************************************************************** //
//
