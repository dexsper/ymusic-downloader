//! FLAC-in-MP4 (`flac-mp4`) demuxer that produces a valid standalone `.flac` file.
//!
//! After decrypting `encraw`, Yandex delivers a standard unfragmented MP4 container
//! (`ftyp/moov/mdat`) where the FLAC stream is embedded: metadata (including `STREAMINFO`)
//! lives in the `dfLa` box (`FLACSpecificBox`), and audio frames are stored in `mdat`
//! distributed across samples in the `stbl` table (`stco`/`stsc`/`stsz`).
//!
//! This module reassembles a valid `.flac` file manually:
//! magic `fLaC` + metadata blocks from `dfLa` (with the last-block flag set correctly)
//! + concatenated raw samples, each of which is already a self-contained FLAC frame.
//!
//! `mp4parse` is used only for structural parsing (it has no "give me the bytes of sample N"
//! API), so sample data is read directly by `(offset, size)` from the source buffer.

use std::io::Cursor;

use mp4parse::{AudioCodecSpecific, SampleEntry, TrackType};

/// Errors from demuxing a `flac-mp4` container.
#[derive(Debug, thiserror::Error)]
pub enum FlacMp4Error {
    #[error("failed to parse MP4 container: {0:?}")]
    Parse(mp4parse::Error),
    #[error("no audio track with FLAC codec found in the container")]
    NoFlacTrack,
    #[error("FLAC track is missing its sample table (stco/stsc/stsz)")]
    MissingSampleTable,
    #[error("sample table references out-of-bounds data (offset={offset}, size={size}, len={len})")]
    SampleOutOfBounds { offset: u64, size: u64, len: usize },
}

struct SampleRange {
    offset: u64,
    size: u64,
}

/// Demuxes a decrypted `flac-mp4` buffer into the bytes of a valid `.flac` file.
///
/// # Errors
/// Returns an error if the container cannot be parsed, contains no FLAC track, is missing
/// its sample table, or the sample table references out-of-bounds data.
pub fn demux_to_flac(mp4_data: &[u8]) -> Result<Vec<u8>, FlacMp4Error> {
    let context = mp4parse::read_mp4(&mut Cursor::new(mp4_data)).map_err(FlacMp4Error::Parse)?;

    let track = context
        .tracks
        .iter()
        .find(|t| t.track_type == TrackType::Audio && flac_blocks(t).is_some())
        .ok_or(FlacMp4Error::NoFlacTrack)?;

    let flac_meta_blocks = flac_blocks(track).ok_or(FlacMp4Error::NoFlacTrack)?;
    let mut out = Vec::with_capacity(mp4_data.len());
    out.extend_from_slice(b"fLaC");
    write_metadata_blocks(&mut out, flac_meta_blocks);

    let ranges = build_sample_ranges(track)?;
    for range in ranges {
        let start = usize::try_from(range.offset).map_err(|_| FlacMp4Error::SampleOutOfBounds {
            offset: range.offset,
            size: range.size,
            len: mp4_data.len(),
        })?;
        let end = start
            .checked_add(usize::try_from(range.size).unwrap_or(usize::MAX))
            .filter(|&e| e <= mp4_data.len())
            .ok_or(FlacMp4Error::SampleOutOfBounds {
                offset: range.offset,
                size: range.size,
                len: mp4_data.len(),
            })?;
        out.extend_from_slice(&mp4_data[start..end]);
    }

    Ok(out)
}

fn flac_blocks(track: &mp4parse::Track) -> Option<&[mp4parse::FLACMetadataBlock]> {
    let stsd = track.stsd.as_ref()?;
    for entry in stsd.descriptions.iter() {
        if let SampleEntry::Audio(audio) = entry
            && let AudioCodecSpecific::FLACSpecificBox(flac) = &audio.codec_specific
        {
            return Some(&flac.blocks);
        }
    }
    None
}

fn write_metadata_blocks(out: &mut Vec<u8>, blocks: &[mp4parse::FLACMetadataBlock]) {
    let last_index = blocks.len().saturating_sub(1);
    for (i, block) in blocks.iter().enumerate() {
        let is_last = i == last_index;
        // Metadata block header: bit 7 = last-metadata-block flag, bits 0-6 = block type.
        let header = (u8::from(is_last) << 7) | (block.block_type & 0x7f);
        out.push(header);
        let len = block.data.len() as u32;
        out.push((len >> 16) as u8);
        out.push((len >> 8) as u8);
        out.push(len as u8);
        out.extend_from_slice(&block.data);
    }
}

fn build_sample_ranges(track: &mp4parse::Track) -> Result<Vec<SampleRange>, FlacMp4Error> {
    let stco = track
        .stco
        .as_ref()
        .ok_or(FlacMp4Error::MissingSampleTable)?;
    let stsc = track
        .stsc
        .as_ref()
        .ok_or(FlacMp4Error::MissingSampleTable)?;
    let stsz = track
        .stsz
        .as_ref()
        .ok_or(FlacMp4Error::MissingSampleTable)?;

    let chunk_offsets: Vec<u64> = stco.offsets.iter().copied().collect();
    let total_samples = if stsz.sample_size != 0 {
        usize::MAX
    } else {
        stsz.sample_sizes.len()
    };

    let stsc_entries: Vec<(u32, u32)> = stsc
        .samples
        .iter()
        .map(|s| (s.first_chunk, s.samples_per_chunk))
        .collect();

    let sample_size_at = |index: usize| -> u64 {
        if stsz.sample_size != 0 {
            u64::from(stsz.sample_size)
        } else {
            stsz.sample_sizes.get(index).copied().map_or(0, u64::from)
        }
    };

    let mut ranges = Vec::new();
    let mut sample_index: usize = 0;

    for (entry_idx, &(first_chunk, samples_per_chunk)) in stsc_entries.iter().enumerate() {
        let next_first_chunk = stsc_entries
            .get(entry_idx + 1)
            .map_or(chunk_offsets.len() as u32 + 1, |&(fc, _)| fc);

        for chunk_number in first_chunk..next_first_chunk {
            let chunk_idx = (chunk_number - 1) as usize;
            let Some(&chunk_offset) = chunk_offsets.get(chunk_idx) else {
                continue;
            };
            let mut offset = chunk_offset;
            for _ in 0..samples_per_chunk {
                if sample_index >= total_samples {
                    break;
                }
                let size = sample_size_at(sample_index);
                if size == 0 && stsz.sample_size == 0 {
                    return Ok(ranges);
                }
                ranges.push(SampleRange { offset, size });
                offset += size;
                sample_index += 1;
            }
        }
    }

    if ranges.is_empty() {
        return Err(FlacMp4Error::MissingSampleTable);
    }
    Ok(ranges)
}
