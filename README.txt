respeaker-record


Records audio when there is voice activity, using ReSpeaker v3.
Periodically converts the segments into text.
Tested on NixOS Version 25.05 with ALSA.
Superuser is needed to access the USB.
Note:
ggml-large-v3.bin : https://huggingface.co/ggerganov/whisper.cpp/blob/main/ggml-large-v3.bin
test if the model is working correctly with respeaker-record -p 10 -d [datadir] -m [model-dir]


./target/debug/respeaker-record --help
Usage: respeaker-record [OPTIONS]

Options:
  -m, --modeldir <MODELDIR>
          Location of ggml-large-v3.bin [default: /home/jongkook90/models]
  -d, --datadir <DATADIR>
          Location of data [default: /home/jongkook90/recordings]
  -l, --lang <LANG>
          Location of data [default: ko]
  -p, --proc-period <PROC_PERIOD>
          Period to run speech recognition [default: 3600]
  -s, --segment-length <SEGMENT_LENGTH>
          Length of silence needed to stop recording [default: 10]
  -h, --help
          Print help
  -V, --version
          Print version