mod export_type;
use serde::{Deserialize, Serialize};
mod metadata;
mod multi_file;
use crate::{AudioBuffer, SynthState};
mod midi_note;
use chrono::Datelike;
use chrono::Local;
use common::{Index, Music, Time, U64orF32, DEFAULT_FRAMERATE, PPQ_F};
pub use export_type::*;
use ghakuf::messages::*;
use ghakuf::writer::*;
use hound::*;
use id3::*;
pub use metadata::*;
use midi_note::*;
use mp3lame_encoder::*;
pub use multi_file::*;
use oggvorbismeta::*;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::{Cursor, Write};
use std::path::Path;
use vorbis_encoder::Encoder;

/// A MIDI pulse. This just reminds us what we're trying to accomplish.
const PULSE: u64 = 1;
/// Conversion factor for f32 to i16.
const F32_TO_I16: f32 = 32767.5;
/// An ordered list of MP3 bit rates.
pub const MP3_BIT_RATES: [Bitrate; 16] = [
    Bitrate::Kbps8,
    Bitrate::Kbps16,
    Bitrate::Kbps24,
    Bitrate::Kbps32,
    Bitrate::Kbps40,
    Bitrate::Kbps48,
    Bitrate::Kbps64,
    Bitrate::Kbps80,
    Bitrate::Kbps96,
    Bitrate::Kbps112,
    Bitrate::Kbps128,
    Bitrate::Kbps160,
    Bitrate::Kbps192,
    Bitrate::Kbps224,
    Bitrate::Kbps256,
    Bitrate::Kbps320,
];
/// An ordererd list of mp3 qualities.
pub const MP3_QUALITIES: [Quality; 10] = [
    Quality::Best,
    Quality::SecondBest,
    Quality::NearBest,
    Quality::VeryNice,
    Quality::Nice,
    Quality::Good,
    Quality::Decent,
    Quality::Ok,
    Quality::SecondWorst,
    Quality::Worst,
];

/// This struct contains all export settings, as well as exporter functions.
#[derive(Default, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct Exporter {
    /// The framerate.
    pub framerate: U64orF32,
    pub metadata: Metadata,
    /// If true, write copyright info.
    pub copyright: bool,
    /// The mp3 quality index.
    pub mp3_bit_rate: Index,
    /// The mp3 quality index.
    pub mp3_quality: Index,
    /// The bit rate index.
    pub bit_rate: Index,
    pub multi_file: MultiFile,
    /// The .ogg file quality index.
    pub ogg_quality: Index,
    /// The export type.
    pub export_type: Index,
}

impl Exporter {
    pub fn new() -> Self {
        Self {
            framerate: U64orF32::from(DEFAULT_FRAMERATE),
            export_type: Index::new(0, EXPORT_TYPES.len()),
            mp3_bit_rate: Index::new(8, MP3_BIT_RATES.len()),
            mp3_quality: Index::new(0, MP3_QUALITIES.len()),
            ogg_quality: Index::new(5, 10),
            ..Default::default()
        }
    }

    /// Export to a .mid file.
    /// - `path` Output to this path.
    /// - `music` This is what we're saving.
    /// - `synth_state` We need this for its present names.
    /// - `text` This is is used for metadata.
    /// - `export_settings` .mid export settings.
    pub fn mid(&self, path: &Path, music: &Music, time: &Time, synth_state: &SynthState) {
        // Gather all notes.
        let mut notes: Vec<MidiNote> = vec![];
        for track in music.midi_tracks.iter() {
            notes.extend(track.notes.iter().map(|n| MidiNote::new(n, track.channel)));
        }
        // End here if there are no notes.
        if notes.is_empty() {
            return;
        }

        // Set the name of the music.
        let mut messages = vec![Message::MetaEvent {
            delta_time: 0,
            event: MetaEvent::TextEvent,
            data: self.metadata.title.as_bytes().to_vec(),
        }];
        // Send copyright.
        if self.copyright {
            messages.push(Message::MetaEvent {
                delta_time: 0,
                event: MetaEvent::CopyrightNotice,
                data: Local::now().year().to_le_bytes().to_vec(),
            });
        }
        // Set the instrument names.
        for program in synth_state.programs.values() {
            messages.push(Message::MetaEvent {
                delta_time: 0,
                event: MetaEvent::InstrumentName,
                data: program.preset_name.as_bytes().to_vec(),
            });
        }
        // Set the tempo.
        let tempo = 60000000 / time.bpm.get_u();
        messages.push(Message::MetaEvent {
            delta_time: 0,
            event: MetaEvent::SetTempo,
            data: [(tempo >> 16) as u8, (tempo >> 8) as u8, tempo as u8].to_vec(),
        });

        // Sort the notes by start time.
        notes.sort_by(|a, b| a.note.start.cmp(&b.note.start));
        // Get the end time.
        let t1 = notes.iter().map(|n| n.note.end).max().unwrap();

        // Get the beat time of one pulse.
        // This is the current time.
        let mut t = 0;

        // The delta-time since the last event.
        let mut dt = 0;

        // Maybe this should be a for loop.
        while t < t1 {
            // Are there any note-on events?
            for note in notes.iter().filter(|n| n.note.start == t) {
                // Note-on.
                messages.push(Message::MidiEvent {
                    delta_time: Self::get_delta_time(&mut dt),
                    event: MidiEvent::NoteOn {
                        ch: note.channel,
                        note: note.note.note,
                        velocity: note.note.velocity,
                    },
                });
            }
            // Are there any note-off events?
            for note in notes.iter().filter(|n| n.note.end == t) {
                // Note-off.
                messages.push(Message::MidiEvent {
                    delta_time: Self::get_delta_time(&mut dt),
                    event: MidiEvent::NoteOff {
                        ch: note.channel,
                        note: note.note.note,
                        velocity: note.note.velocity,
                    },
                });
            }
            // Increment the time and the delta-time.
            t += PULSE;
            dt += PULSE;
        }
        // Track end.
        messages.push(Message::MetaEvent {
            delta_time: 0,
            event: MetaEvent::EndOfTrack,
            data: vec![],
        });
        // Write.
        let mut writer = Writer::new();
        writer.running_status(true);
        for message in &messages {
            writer.push(message);
        }
        if let Err(error) = writer.write(path) {
            panic!("Error writing {:?} {:?}", path, error);
        }
    }

    /// Export to a .wav file.
    ///
    /// - `path` The output path.
    /// - `buffer` A buffer of wav data.
    pub(crate) fn wav(&self, path: &Path, buffer: &AudioBuffer) {
        // Get the spec.
        let spec = WavSpec {
            channels: 2,
            sample_rate: self.framerate.get_u() as u32,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        // Write.
        let mut writer = WavWriter::create(path, spec).unwrap();
        let mut i16_writer = writer.get_i16_writer(buffer[0].len() as u32 * 2);
        for (l, r) in buffer[0].iter().zip(buffer[1].iter()) {
            i16_writer.write_sample(Self::to_i16(l));
            i16_writer.write_sample(Self::to_i16(r));
        }
        i16_writer.flush().unwrap();
        writer.finalize().unwrap();
    }

    /// Export to a .mp3 file.
    ///
    /// - `path` The output path.
    /// - `buffer` A buffer of wav data.
    pub(crate) fn mp3(&self, path: &Path, buffer: &AudioBuffer) {
        // Create the encoder.
        let mut mp3_encoder = Builder::new().expect("Create LAME builder");
        mp3_encoder.set_num_channels(2).expect("set channels");
        mp3_encoder
            .set_sample_rate(self.framerate.get_u() as u32)
            .expect("set sample rate");
        mp3_encoder
            .set_brate(MP3_BIT_RATES[self.mp3_bit_rate.get()])
            .expect("set bitrate");
        mp3_encoder
            .set_quality(MP3_QUALITIES[self.mp3_quality.get()])
            .expect("set quality");
        // Build the encoder.
        let mut mp3_encoder = mp3_encoder.build().expect("To initialize LAME encoder");
        // Get the input.
        let input = DualPcm {
            left: &buffer[0],
            right: &buffer[1],
        };
        // Get the output buffer.
        let mut mp3_out_buffer = Vec::new();
        mp3_out_buffer.reserve(max_required_buffer_size(buffer[0].len()));
        // Get the size.
        let encoded_size = mp3_encoder
            .encode(input, mp3_out_buffer.spare_capacity_mut())
            .expect("To encode");
        unsafe {
            mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
        }
        let encoded_size = mp3_encoder
            .flush::<FlushNoGap>(mp3_out_buffer.spare_capacity_mut())
            .expect("to flush");
        unsafe {
            mp3_out_buffer.set_len(mp3_out_buffer.len().wrapping_add(encoded_size));
        }
        // Write the file.
        let mut file = OpenOptions::new()
            .write(true)
            .append(false)
            .truncate(true)
            .create(true)
            .open(path)
            .expect("Error opening file {:?}");
        if let Err(error) = file.write(&mp3_out_buffer) {
            panic!("Failed to export mp3 to {:?}: {}", path, error)
        }
        // Write the tag.
        self.write_id3_tag(path);
    }

    /// Export to a .ogg file.
    ///
    /// - `path` The output path.
    /// - `buffer` A buffer of wav data.
    pub(crate) fn ogg(&self, path: &Path, buffer: &AudioBuffer) {
        let mut samples = vec![];
        for (l, r) in buffer[0].iter().zip(buffer[1].iter()) {
            samples.push(Self::to_i16(l));
            samples.push(Self::to_i16(r));
        }
        let mut encoder = Encoder::new(
            2,
            self.framerate.get_u(),
            (self.ogg_quality.get() as f32 / 9.0) * 1.2 - 0.2,
        )
        .expect("Error creating .ogg file encoder.");
        let samples = encoder
            .encode(&samples)
            .expect("Error encoding .ogg samples.");
        let year = Local::now().year();
        // Get a cursor.
        let cursor = Cursor::new(&samples);
        // Write the comments.
        let mut comments = CommentHeader::new();
        comments.set_vendor("Ogg");
        comments.add_tag_single("title", &self.metadata.title);
        comments.add_tag_single("date", &year.to_string());
        if let Some(artist) = &self.metadata.artist {
            comments.add_tag_single("artist", artist);
            if self.copyright {
                comments.add_tag_single("copyright", &format!("Copyright {} {}", year, artist));
            }
        }
        if let Some(album) = &self.metadata.album {
            comments.add_tag_single("album", album);
        }
        if let Some(genre) = &self.metadata.genre {
            comments.add_tag_single("genre", genre);
        }
        if let Some(track_number) = &self.metadata.track_number {
            comments.add_tag_single("tracknumber", &track_number.to_string());
        }
        if let Some(comment) = &self.metadata.genre {
            comments.add_tag_single("description", comment);
        }
        // Write the comments.
        let mut out = vec![];
        replace_comment_header(cursor, comments)
            .read_to_end(&mut out)
            .expect("Error reading cursor.");
        // Write the file.
        let mut file = OpenOptions::new()
            .write(true)
            .append(false)
            .truncate(true)
            .create(true)
            .open(path)
            .expect("Error opening file.");
        file.write_all(&out)
            .expect("Failed to write samples to file.");
    }

    /// Converts a PPQ value into a MIDI time delta and resets `ppq` to zero.
    fn get_delta_time(ppq: &mut u64) -> u32 {
        // Get the dt.
        let dt = (*ppq as f32 / PPQ_F) as u32;
        // Reset the PPQ value.
        *ppq = 0;
        dt
    }

    /// Converts an f32 sample to an i16 sample.
    fn to_i16(sample: &f32) -> i16 {
        (sample * F32_TO_I16).floor() as i16
    }

    /// Write an ID3 tag to a file.
    fn write_id3_tag(&self, path: &Path) {
        let time = Local::now();
        let mut tag = Tag::new();
        tag.set_year(time.year());
        tag.set_title(&self.metadata.title);
        if let Some(artist) = &self.metadata.artist {
            tag.set_artist(artist);
        }
        if let Some(album) = &self.metadata.album {
            tag.set_album(album);
        }
        if let Some(genre) = &self.metadata.genre {
            tag.set_genre(genre);
        }
        if let Some(comment) = &self.metadata.comment {
            tag.set_genre(comment);
        }
        if let Some(track_number) = &self.metadata.track_number {
            tag.set_track(*track_number);
        }
        if let Err(error) = tag.write_to_path(path, Version::Id3v24) {
            panic!("Error writing ID3 tag to {:?}: {}", path, error);
        }
    }
}
