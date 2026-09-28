//! Fixed, synthetic inputs exercise file formats used by the previous release.
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures/compat")
        .join(name)
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("meetinghelper-compat-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn source_hashes_keep_the_standard_sha256_representation() {
    assert_eq!(
        nexq_lib::projects::knowledge::content_hash("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn existing_docx_and_pdf_formats_remain_readable() {
    let docx = nexq_lib::rag::file_processor::extract_docx_text(
        fixture("document.docx").to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(docx, "Pipeline review\nTürkçe kaynak");
    let pdf = nexq_lib::context::pdf_extractor::extract_text_from_pdf(
        fixture("document.pdf").to_str().unwrap(),
    )
    .unwrap();
    assert!(pdf.contains("Project pipeline requires human review."));
    assert!(nexq_lib::rag::file_processor::extract_docx_text(
        fixture("document.pdf").to_str().unwrap()
    )
    .is_err());
}

#[test]
fn model_archive_extracts_and_corrupt_archive_fails() {
    let scratch = Scratch::new();
    let model = nexq_lib::stt::local_engines::downloader::extract_tar_bz2(
        &fixture("model.tar.bz2"),
        &scratch.0,
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(model.join("config.txt")).unwrap(),
        "compatibility fixture, no model weights\n"
    );
    assert!(nexq_lib::stt::local_engines::downloader::extract_tar_bz2(
        &fixture("document.pdf"),
        &scratch.0
    )
    .is_err());
}

#[test]
fn pcm_recording_encodes_to_a_decodable_opus_container() {
    let scratch = Scratch::new();
    let wav = scratch.0.join("input.wav");
    let opus = scratch.0.join("output.opus");
    let mut writer = hound::WavWriter::create(
        &wav,
        hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for n in 0..1920 {
        writer
            .write_sample(((n as f32 * 0.1728).sin() * 4000.0) as i16)
            .unwrap();
    }
    writer.finalize().unwrap();
    assert!(nexq_lib::audio::encoder::encode_wav_to_opus(&wav, &opus).unwrap() > 0);
    let mut reader = ogg::reading::PacketReader::new(std::fs::File::open(opus).unwrap());
    assert!(reader
        .read_packet_expected()
        .unwrap()
        .data
        .starts_with(b"OpusHead"));
    assert!(reader
        .read_packet_expected()
        .unwrap()
        .data
        .starts_with(b"OpusTags"));
    let mut error = 0;
    let decoder = unsafe { libopus_sys::opus_decoder_create(16000, 1, &mut error) };
    assert_eq!(error, 0);
    assert!(!decoder.is_null());
    let mut decoded = [0_i16; 5760];
    let mut frames = 0;
    while let Some(packet) = reader.read_packet().unwrap() {
        let count = unsafe {
            libopus_sys::opus_decode(
                decoder,
                packet.data.as_ptr(),
                packet.data.len() as i32,
                decoded.as_mut_ptr(),
                decoded.len() as i32,
                0,
            )
        };
        assert_eq!(count, 960);
        frames += 1;
    }
    unsafe { libopus_sys::opus_decoder_destroy(decoder) };
    assert_eq!(frames, 2);
}
