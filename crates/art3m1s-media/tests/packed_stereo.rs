#![cfg(feature = "ffmpeg")]
use art3m1s_media::{MediaSource, ffmpeg::decode_audio_to_wav};
use std::sync::Arc;

struct Bytes(Vec<u8>);
impl MediaSource for Bytes {
    fn len(&self) -> Result<u64, String> {
        Ok(self.0.len() as u64)
    }
    fn read_at(&self, offset: u64, out: &mut [u8]) -> Result<usize, String> {
        let start = (offset as usize).min(self.0.len());
        let len = out.len().min(self.0.len() - start);
        out[..len].copy_from_slice(&self.0[start..start + len]);
        Ok(len)
    }
}

#[test]
fn packed_stereo_export_preserves_both_channels_and_frame_count() {
    let frames = [
        (1000_i16, -2000_i16),
        (2000, -3000),
        (3000, -4000),
        (4000, -5000),
        (5000, -6000),
        (6000, -7000),
        (7000, -8000),
        (8000, -9000),
    ];
    let samples: Vec<u8> = frames
        .iter()
        .flat_map(|(l, r)| l.to_le_bytes().into_iter().chain(r.to_le_bytes()))
        .collect();
    let size = samples.len() as u32;
    let mut wav = b"RIFF".to_vec();
    wav.extend_from_slice(&(36 + size).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&48_000_u32.to_le_bytes());
    wav.extend_from_slice(&192_000_u32.to_le_bytes());
    wav.extend_from_slice(&4_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&size.to_le_bytes());
    wav.extend_from_slice(&samples);
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let output = std::env::temp_dir().join(format!(
        "art3m1s-stereo-{}-{unique}.wav",
        std::process::id()
    ));
    let decoded = decode_audio_to_wav(
        Arc::new(Bytes(wav)),
        &output,
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
    );
    let actual = std::fs::read(&output);
    let _ = std::fs::remove_file(&output);
    assert_eq!(decoded, Ok(true));
    let actual = actual.unwrap();
    assert_eq!(actual.len(), 44 + samples.len());
    assert_eq!(&actual[40..44], &size.to_le_bytes());
    assert_eq!(&actual[44..], samples);
}
