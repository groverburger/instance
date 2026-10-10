//! Audio buffers: WAV reading/writing and decoding of other formats.

/// Interleaved f32 audio.
#[derive(Clone, Debug)]
pub struct Audio {
    pub rate: u32,
    pub channels: u16,
    /// Interleaved samples in -1..1.
    pub samples: Vec<f32>,
    /// Source format description (e.g. "WAV PCM 16-bit").
    pub format: String,
}

impl Audio {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }
    pub fn duration(&self) -> f64 {
        self.frames() as f64 / self.rate.max(1) as f64
    }
    /// Mono mix-down.
    pub fn mono(&self) -> Vec<f32> {
        let c = self.channels.max(1) as usize;
        if c == 1 {
            return self.samples.clone();
        }
        self.samples.chunks(c).map(|f| f.iter().sum::<f32>() / c as f32).collect()
    }
}

/// Encodes a 16-bit PCM WAV file.
pub fn encode_wav(rate: u32, channels: u16, samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0).round() as i16).to_le_bytes());
    }
    b
}

fn u16le(d: &[u8], p: usize) -> u16 {
    u16::from_le_bytes([d[p], d[p + 1]])
}
fn u32le(d: &[u8], p: usize) -> u32 {
    u32::from_le_bytes([d[p], d[p + 1], d[p + 2], d[p + 3]])
}

/// Decodes a RIFF WAVE file with PCM (8/16/24/32-bit) or float (32/64-bit) data.
pub fn decode_wav(d: &[u8]) -> Result<Audio, String> {
    if d.len() < 12 || &d[0..4] != b"RIFF" || &d[8..12] != b"WAVE" {
        return Err("not a RIFF WAVE file".into());
    }
    let mut p = 12;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while p + 8 <= d.len() {
        let id = &d[p..p + 4];
        let len = u32le(d, p + 4) as usize;
        let body = &d[p + 8..(p + 8 + len).min(d.len())];
        if id == b"fmt " && body.len() >= 16 {
            let mut tag = u16le(body, 0);
            if tag == 0xFFFE && body.len() >= 26 {
                tag = u16le(body, 24); // WAVE_FORMAT_EXTENSIBLE sub-format
            }
            fmt = Some((tag, u16le(body, 2), u32le(body, 4), u16le(body, 14)));
        } else if id == b"data" {
            data = Some(body);
        }
        p += 8 + len + (len & 1);
    }
    let (tag, channels, rate, bits) = fmt.ok_or("WAV without fmt chunk")?;
    let data = data.ok_or("WAV without data chunk")?;
    if channels == 0 || rate == 0 {
        return Err("WAV with 0 channels or rate".into());
    }
    let samples: Vec<f32> = match (tag, bits) {
        (1, 8) => data.iter().map(|b| (*b as f32 - 128.0) / 128.0).collect(),
        (1, 16) => data.as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c) as f32 / 32768.0).collect(),
        (1, 24) => data.as_chunks::<3>().0.iter().map(|c| (i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8) as f32 / 8_388_608.0).collect(),
        (1, 32) => data.as_chunks::<4>().0.iter().map(|c| i32::from_le_bytes(*c) as f32 / 2_147_483_648.0).collect(),
        (3, 32) => data.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect(),
        (3, 64) => data.as_chunks::<8>().0.iter().map(|c| f64::from_le_bytes(*c) as f32).collect(),
        _ => return Err(format!("WAV format tag {tag} with {bits} bits is not supported (compressed WAV)")),
    };
    let kind = if tag == 3 { "float" } else { "PCM" };
    Ok(Audio { rate, channels, samples, format: format!("WAV {kind} {bits}-bit") })
}

/// Decodes MP3/OGG/FLAC (and WAV) through symphonia.
pub fn decode_other(d: &[u8], hint_ext: &str) -> Result<Audio, String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;
    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(d.to_vec())), Default::default());
    let mut hint = Hint::new();
    if !hint_ext.is_empty() {
        hint.with_extension(hint_ext.trim_start_matches('.'));
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("cannot decode: {e}"))?;
    let mut format = probed.format;
    let track = format.default_track().ok_or("no audio track")?.clone();
    let mut dec = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|e| format!("cannot decode: {e}"))?;
    let codec = symphonia::default::get_codecs().get_codec(track.codec_params.codec).map(|c| c.short_name.to_uppercase()).unwrap_or_else(|| "?".into());
    let mut out = Vec::new();
    let mut rate = track.codec_params.sample_rate.unwrap_or(0);
    let mut channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(0);
    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track.id {
            continue;
        }
        match dec.decode(&packet) {
            Ok(buf) => {
                let spec = *buf.spec();
                rate = spec.rate;
                channels = spec.channels.count() as u16;
                let mut sb = SampleBuffer::<f32>::new(buf.capacity() as u64, spec);
                sb.copy_interleaved_ref(buf);
                out.extend_from_slice(sb.samples());
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }
    if rate == 0 || channels == 0 {
        return Err("decoded no audio".into());
    }
    Ok(Audio { rate, channels, samples: out, format: codec })
}

/// Decodes WAV (own reader) or another format via symphonia.
pub fn decode(d: &[u8], ext: &str) -> Result<Audio, String> {
    if d.len() >= 12 && &d[0..4] == b"RIFF" && &d[8..12] == b"WAVE" {
        match decode_wav(d) {
            Ok(a) => Ok(a),
            Err(e) => decode_other(d, "wav").map_err(|_| e),
        }
    } else {
        decode_other(d, ext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_round_trip() {
        let s: Vec<f32> = (0..100).map(|i| ((i as f32) * 0.1).sin() * 0.5).collect();
        let a = decode_wav(&encode_wav(22050, 2, &s)).unwrap();
        assert_eq!((a.rate, a.channels, a.samples.len()), (22050, 2, 100));
        assert!(a.samples.iter().zip(&s).all(|(a, b)| (a - b).abs() < 1e-4));
    }
}
