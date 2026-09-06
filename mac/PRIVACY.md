# Privacy, Chordmap for Mac

Chordmap for Mac hears what the Mac is playing through a Core Audio tap (the same mechanism as the system's own audio routing), or an input device you choose, and hands the sound to the chordmap engine inside the app. The sound stays in memory for the length of a listen, at 24 kHz, ten minutes at most, and is never written to disk. macOS asks once for System Audio Recording, or once for the Microphone if you pick an input; the app records nothing with either.

The history keeps each listen's title, time, key, tempo, chords and sections in `~/Library/Application Support/Chordmap for Mac/listens.json`. Delete the file and it is gone. No audio is stored.

Nothing is sent anywhere. The only connection the app opens is the optional daily update check, which asks GitHub for the version number of the latest release and sends nothing about you. No analytics, no crash reporting, no account.
