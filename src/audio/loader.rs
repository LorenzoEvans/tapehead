use std::fs::File;
use std::path::Path;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};
use rubato::{Resampler, Fft, FixedSync};
use audioadapter_buffers::direct::SequentialSliceOfVecs;
use anyhow::{Context, Result};
use crate::audio::AudioSource;

pub fn load_audio<P: AsRef<Path>>(path: P) -> Result<AudioSource> {
    let file = File::open(path.as_ref()).context("Failed to open file")?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.as_ref().extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }

    let probed = get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .context("Failed to probe audio format")?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No supported audio tracks found")?;

    let mut decoder = get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;
    let track_id = track.id;

    let mut samples = Vec::new();
    let mut source_sample_rate = 0;
    let mut channels = 0;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(ref err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(err) => return Err(err.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                if source_sample_rate == 0 {
                    source_sample_rate = audio_buf.spec().rate;
                    channels = audio_buf.spec().channels.count();
                }

                let mut temp_buf = symphonia::core::audio::SampleBuffer::<f32>::new(
                    audio_buf.capacity() as u64,
                    *audio_buf.spec(),
                );
                temp_buf.copy_interleaved_ref(audio_buf);
                samples.extend_from_slice(temp_buf.samples());
            }
            Err(Error::DecodeError(_)) => continue,
            Err(err) => return Err(err.into()),
        }
    }

    if source_sample_rate == 0 {
        return Err(anyhow::anyhow!("No audio data decoded"));
    }

    let target_sample_rate = 44100;
    if source_sample_rate != target_sample_rate {
        let mut resampler = Fft::<f32>::new(
            source_sample_rate as usize,
            target_sample_rate as usize,
            1024,
            1,
            channels,
            FixedSync::Both,
        ).map_err(|e| anyhow::anyhow!("Failed to create resampler: {:?}", e))?;

        let num_frames = samples.len() / channels;
        let mut input_vecs = vec![vec![0.0f32; num_frames]; channels];
        for (i, &sample) in samples.iter().enumerate() {
            input_vecs[i % channels][i / channels] = sample;
        }

        let input_adapter = SequentialSliceOfVecs::new(&input_vecs, channels, num_frames)
            .map_err(|e| anyhow::anyhow!("Failed to create input adapter: {:?}", e))?;
        let out_len = resampler.process_all_needed_output_len(num_frames);
        let mut output_vecs = vec![vec![0.0f32; out_len]; channels];
        let mut output_adapter = SequentialSliceOfVecs::new_mut(&mut output_vecs, channels, out_len)
            .map_err(|e| anyhow::anyhow!("Failed to create output adapter: {:?}", e))?;

        let (_, out_frames) = resampler
            .process_all_into_buffer(&input_adapter, &mut output_adapter, num_frames, None)
            .map_err(|e| anyhow::anyhow!("Resampling failed: {:?}", e))?;
        
        let mut interleaved = Vec::with_capacity(out_frames * channels);
        for i in 0..out_frames {
            for j in 0..channels {
                interleaved.push(output_vecs[j][i]);
            }
        }
        samples = interleaved;
    }

    Ok(AudioSource {
        samples,
        sample_rate: target_sample_rate as u32,
        channels,
    })
}
