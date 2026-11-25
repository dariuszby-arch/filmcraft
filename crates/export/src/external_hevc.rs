//! Optional native-desktop HEVC export through an external FFmpeg executable.
//!
//! Unlike an H.264 intermediate/transcode, the rendered frames are streamed
//! directly to FFmpeg (RGBA8 for Main, RGB48 for Main 10). FilmCraft's audio
//! exporter produces a separate m4a that is copied into the final container.
//! No FFmpeg dependency is linked into the Rust binary.

use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use filmcraft_project::{ItemId, Project};
use filmcraft_render::SourceProvider;

use crate::pipeline::Pipeline;
use crate::{Arc, BitrateMode, ExportError, ExportSettings, Format, HevcEncoder, Progress, Report, Result, export_range, frame_span};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Backend {
    IntelQsv,
    Libx265,
}

/// Run FFmpeg without a visible console on Windows. FilmCraft is a GUI app;
/// CREATE_NO_WINDOW suppresses the transient and long-lived console windows
/// for version checks, encoder probes, video encoding, and audio/video remuxing.
/// It does not change pipe handling, logs, progress, or cancellation.
fn ffmpeg_process(path: &Path) -> Command {
    let command = Command::new(path);
    #[cfg(windows)]
    let mut command = command;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Resolve FFmpeg once. An explicit override is useful for portable Windows builds;
/// a sibling `ffmpeg.exe` also works when FFmpeg is not on the system PATH.
fn ffmpeg_executable() -> Option<&'static Path> {
    static BINARY: OnceLock<Option<PathBuf>> = OnceLock::new();
    BINARY
        .get_or_init(|| {
            let mut candidates = Vec::new();
            if let Some(explicit) = std::env::var_os("FILMCRAFT_FFMPEG") {
                candidates.push(PathBuf::from(explicit));
            }
            if let Ok(exe) = std::env::current_exe()
                && let Some(folder) = exe.parent()
            {
                candidates.push(folder.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }));
                candidates.push(folder.join(if cfg!(windows) { "ffmpeg/bin/ffmpeg.exe" } else { "ffmpeg/bin/ffmpeg" }));
            }
            candidates.push(PathBuf::from("ffmpeg"));
            candidates
                .into_iter()
                .find(|path| ffmpeg_process(path).arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success()))
        })
        .as_deref()
}

fn ffmpeg_command() -> Result<Command> {
    let binary = ffmpeg_executable()
        .ok_or_else(|| ExportError::Unsupported("FFmpeg not found. Install it on PATH, place ffmpeg.exe beside FilmCraft, or set FILMCRAFT_FFMPEG".into()))?;
    Ok(ffmpeg_process(binary))
}

fn encoder_list() -> &'static str {
    static ENCODERS: OnceLock<String> = OnceLock::new();
    ENCODERS.get_or_init(|| {
        ffmpeg_executable()
            .and_then(|bin| ffmpeg_process(bin).args(["-hide_banner", "-encoders"]).output().ok())
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default()
    })
}

pub(crate) fn has_x265() -> bool {
    encoder_list().lines().any(|line| line.split_whitespace().nth(1) == Some("libx265"))
}

/// A name in `ffmpeg -encoders` alone proves nothing: test whether the driver
/// can actually open a Main 10 session (as the user's Intel Iris Xe does).
pub(crate) fn has_intel_qsv() -> bool {
    static PROBE: OnceLock<bool> = OnceLock::new();
    *PROBE.get_or_init(|| {
        if !encoder_list().lines().any(|line| line.split_whitespace().nth(1) == Some("hevc_qsv")) {
            return false;
        }
        ffmpeg_executable().is_some_and(|bin| {
            ffmpeg_process(bin)
                .args([
                    "-hide_banner",
                    "-loglevel",
                    "error",
                    "-nostdin",
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc2=size=640x360:rate=30",
                    "-frames:v",
                    "1",
                    "-vf",
                    "format=p010le",
                    "-c:v",
                    "hevc_qsv",
                    "-profile:v",
                    "main10",
                    "-f",
                    "null",
                    "-",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        })
    })
}

pub(crate) fn available() -> bool {
    has_x265() || has_intel_qsv()
}

/// `None` means to keep using FilmCraft's existing platform encoder.
/// Never silently switch an explicitly chosen backend to another one.
pub(crate) fn select(settings: &ExportSettings) -> Result<Option<Backend>> {
    let selected = match settings.hevc_encoder {
        HevcEncoder::Platform => return Ok(None),
        HevcEncoder::IntelQsv => {
            if !has_intel_qsv() {
                return Err(ExportError::Unsupported("Intel HEVC Quick Sync is not available in the installed FFmpeg or GPU driver".into()));
            }
            Backend::IntelQsv
        }
        HevcEncoder::Libx265 => {
            if !has_x265() {
                return Err(ExportError::Unsupported("FFmpeg with libx265 is required for CPU HEVC export".into()));
            }
            Backend::Libx265
        }
        HevcEncoder::Auto => {
            // Keep macOS's native VideoToolbox export when a caller has not opted into FFmpeg.
            if cfg!(target_os = "macos") {
                return Ok(None);
            }
            if has_intel_qsv() {
                Backend::IntelQsv
            } else if has_x265() {
                Backend::Libx265
            } else {
                return Ok(None);
            }
        }
    };
    Ok(Some(selected))
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let path = std::env::temp_dir().join(format!("filmcraft-hevc-{}-{stamp}-{id}", std::process::id()));
        fs::create_dir(&path).map_err(|e| ExportError::Io(format!("{}: {e}", path.display())))?;
        Ok(Self(path))
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// An unfinished FFmpeg process must not survive cancellation or a failed frame render.
struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn arg_string(v: impl ToString) -> String {
    v.to_string()
}

fn encoding_args(settings: &ExportSettings, backend: Backend, width: u32, height: u32, fps: &str, bitrate: u32, max_bitrate: u32) -> Vec<String> {
    let mut args: Vec<String> = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-y",
        "-f",
        "rawvideo",
        "-pixel_format",
        if settings.hevc_bit_depth == 10 { "rgb48le" } else { "rgba" },
        "-video_size",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    args.push(format!("{width}x{height}"));
    args.extend(["-framerate".into(), fps.into(), "-i".into(), "pipe:0".into()]);
    // Explicit matrix/range: without this FFmpeg may encode BT.601 YUV from the RGBA frames.
    let pixel_format = match (backend, settings.hevc_bit_depth) {
        (Backend::IntelQsv, 10) => "p010le",
        (Backend::IntelQsv, _) => "nv12",
        (Backend::Libx265, 10) => "yuv420p10le",
        (Backend::Libx265, _) => "yuv420p",
    };
    let vf = format!("scale=in_range=pc:out_range=tv:out_color_matrix=bt709:flags=lanczos,format={pixel_format}");
    args.extend(["-vf".into(), vf, "-c:v".into()]);
    args.push(match backend {
        Backend::IntelQsv => "hevc_qsv".into(),
        Backend::Libx265 => "libx265".into(),
    });
    args.extend(["-profile:v".into(), if settings.hevc_bit_depth == 10 { "main10".into() } else { "main".into() }]);
    match settings.bitrate_mode {
        BitrateMode::Crf => {
            args.extend([
                match backend {
                    Backend::IntelQsv => "-global_quality",
                    Backend::Libx265 => "-crf",
                }
                .into(),
                arg_string(settings.crf),
            ]);
        }
        BitrateMode::Cbr | BitrateMode::Vbr1Pass => {
            args.extend(["-b:v".into(), format!("{}k", bitrate)]);
            let ceiling = if settings.bitrate_mode == BitrateMode::Cbr { bitrate } else { max_bitrate };
            args.extend(["-maxrate".into(), format!("{ceiling}k"), "-bufsize".into(), format!("{}k", ceiling.saturating_mul(2))]);
            if settings.bitrate_mode == BitrateMode::Cbr {
                args.extend(["-minrate".into(), format!("{bitrate}k")]);
            }
        }
        BitrateMode::Vbr2Pass => {} // rejected before invoking FFmpeg
    }
    if backend == Backend::Libx265 {
        args.extend(["-preset".into(), "medium".into(), "-x265-params".into(), "colorprim=bt709:transfer=bt709:colormatrix=bt709:range=limited".into()]);
    }
    args.extend([
        "-color_range".into(),
        "tv".into(),
        "-color_primaries".into(),
        "bt709".into(),
        "-color_trc".into(),
        "bt709".into(),
        "-colorspace".into(),
        "bt709".into(),
        // Also write the colour triplet into the coded stream's VUI (not only MP4 colr).
        "-bsf:v".into(),
        "hevc_metadata=colour_primaries=1:transfer_characteristics=1:matrix_coefficients=1:video_full_range_flag=0".into(),
        "-tag:v".into(),
        "hvc1".into(),
        "-an".into(),
        "-movflags".into(),
        "+faststart".into(),
    ]);
    args
}

fn run_command(mut command: Command, detail: &str) -> Result<()> {
    let result = command.output().map_err(|e| ExportError::Io(format!("FFmpeg ({detail}): {e}")))?;
    if !result.status.success() {
        let msg = String::from_utf8_lossy(&result.stderr);
        return Err(ExportError::Encode(format!("FFmpeg ({detail}): {}", msg.trim())));
    }
    Ok(())
}

fn save_output(from: &Path, settings: &ExportSettings) -> Result<u64> {
    let mut source = File::open(from).map_err(|e| ExportError::Io(e.to_string()))?;
    let mut dest = if let Some(file) = &settings.output_file {
        file.try_clone().map_err(|e| ExportError::Io(e.to_string()))?
    } else {
        File::create(&settings.path).map_err(|e| ExportError::Io(format!("{}: {e}", settings.path)))?
    };
    dest.set_len(0).map_err(|e| ExportError::Io(e.to_string()))?;
    dest.seek(SeekFrom::Start(0)).map_err(|e| ExportError::Io(e.to_string()))?;
    let size = std::io::copy(&mut source, &mut dest).map_err(|e| ExportError::Io(e.to_string()))?;
    dest.flush().map_err(|e| ExportError::Io(e.to_string()))?;
    Ok(size)
}

pub(crate) fn export(
    project: &Arc<Project>,
    seq: ItemId,
    settings: &ExportSettings,
    sources: &dyn SourceProvider,
    progress: &Progress,
    backend: Backend,
) -> Result<Report> {
    if settings.has_audio() && settings.audio_codec() != crate::AudioCodec::Aac {
        return Err(ExportError::Unsupported("FFmpeg HEVC export currently supports AAC audio only; choose AAC in Audio settings".into()));
    }
    if settings.sink.is_some() {
        return Err(ExportError::Unsupported("external FFmpeg HEVC export cannot write to the in-memory output sink".into()));
    }
    if settings.bitrate_mode == BitrateMode::Vbr2Pass {
        return Err(ExportError::Unsupported("external HEVC currently supports CRF, CBR and single-pass VBR, not two-pass VBR".into()));
    }
    if settings.pixel_aspect.is_some_and(|(n, d)| n != d) {
        return Err(ExportError::Unsupported("external HEVC requires square pixels".into()));
    }
    // Main 10 uses RGB48 from the float compositor; 8-bit Main keeps the original RGBA8 path.
    // Precision previously lost when decoding an 8-bit source is not recoverable.
    if project.sequence(seq).is_some_and(|q| q.settings.color.working.is_hdr()) && !settings.sdr {
        return Err(ExportError::Unsupported("FFmpeg HEVC currently exports SDR BT.709 only; enable SDR export for this HDR sequence".into()));
    }
    let started = std::time::Instant::now();
    let range = export_range(project, seq, settings)?;
    let pipeline = Pipeline::new(project.clone(), seq, settings, false)?;
    if pipeline.w % 2 != 0 || pipeline.h % 2 != 0 {
        return Err(ExportError::Unsupported("HEVC 4:2:0 requires even frame dimensions".into()));
    }
    let (f0, f1) = frame_span(pipeline.rate, range);
    let frames = u64::try_from(f1 - f0).unwrap_or(0);
    let resolved = project.sequence(seq).ok_or(ExportError::NoSequence)?;
    let bitrate = settings.resolve(resolved.settings.width, resolved.settings.height, resolved.settings.frame_rate, resolved.settings.sample_rate);
    let fps = format!("{}/{}", pipeline.rate.num, pipeline.rate.den);
    let dir = TempDir::new()?;
    let video = dir.path("video.mp4");
    let video_log = dir.path("video.log");
    let mut command = ffmpeg_command()?;
    command.args(encoding_args(settings, backend, pipeline.w, pipeline.h, &fps, bitrate.target_kbps, bitrate.max_kbps));
    command.arg(&video);
    command.stdin(Stdio::piped()).stdout(Stdio::null());
    command.stderr(Stdio::from(File::create(&video_log).map_err(|e| ExportError::Io(e.to_string()))?));
    let mut process = ChildGuard(command.spawn().map_err(|e| ExportError::Io(format!("could not start FFmpeg: {e}")))?);
    let mut input = process.0.stdin.take().ok_or_else(|| ExportError::Io("FFmpeg stdin is unavailable".into()))?;
    if !settings.part_of_batch {
        progress.total.store(frames.saturating_add(u64::from(settings.include_audio)), Ordering::Relaxed);
        progress.set_status(format!("Encoding {} frames (HEVC {}-bit / FFmpeg)", frames, settings.hevc_bit_depth));
    }
    for f in f0..f1 {
        if progress.cancel.load(Ordering::Relaxed) {
            return Err(ExportError::Cancelled);
        }
        let rgba = if settings.hevc_bit_depth == 10 { pipeline.frame_rgb48(f, sources) } else { pipeline.frame(f, sources).0 };
        if filmcraft_media::pending::take() {
            return Err(ExportError::Io("source media is pending; external FFmpeg requires a local ready source".into()));
        }
        input.write_all(&rgba).map_err(|e| ExportError::Encode(format!("FFmpeg input pipe: {e}")))?;
        progress.done.fetch_add(1, Ordering::Relaxed);
    }
    drop(input);
    let status = process.0.wait().map_err(|e| ExportError::Io(e.to_string()))?;
    if !status.success() {
        let why = fs::read_to_string(&video_log).unwrap_or_else(|_| "no encoder error details".into());
        return Err(ExportError::Encode(format!("FFmpeg HEVC: {}", why.trim())));
    }
    // The FilmCraft audio path performs the same sequence mix, loudness processing and AAC
    // encode as its regular MP4 export. Remuxing is stream-copy: no second lossy encode.
    let completed = if settings.include_audio {
        progress.set_status("Encoding audio for HEVC");
        let audio = dir.path("audio.m4a");
        let mut audio_settings = settings.clone();
        audio_settings.format = Format::Aac;
        audio_settings.path = audio.to_string_lossy().into_owned();
        audio_settings.sink = None;
        audio_settings.output_file = None;
        let audio_progress = Progress::default();
        super::run::export_here(project, seq, &audio_settings, sources, &audio_progress)?;
        if progress.cancel.load(Ordering::Relaxed) {
            return Err(ExportError::Cancelled);
        }
        progress.set_status("Multiplexing HEVC video and AAC audio");
        let joined = dir.path(if settings.extension() == "mov" { "output.mov" } else { "output.mp4" });
        let mut mux = ffmpeg_command()?;
        mux.args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"])
            .arg(&video)
            .arg("-i")
            .arg(&audio)
            .args(["-map", "0:v:0", "-map", "1:a:0", "-c", "copy", "-movflags", "+faststart"])
            .arg(&joined);
        run_command(mux, "audio/video mux")?;
        joined
    } else {
        video
    };
    if progress.cancel.load(Ordering::Relaxed) {
        return Err(ExportError::Cancelled);
    }
    let bytes = save_output(&completed, settings)?;
    let seconds = started.elapsed().as_secs_f64();
    if !settings.part_of_batch {
        progress.done.store(progress.total.load(Ordering::Relaxed), Ordering::Relaxed);
        progress.finished.store(true, Ordering::Relaxed);
        progress.set_status(format!("HEVC export completed in {seconds:.1}s"));
    }
    Ok(Report {
        path: settings.path.clone(),
        frames,
        seconds,
        bytes,
        render_fps: frames as f64 / seconds.max(1e-6),
        extra_files: Vec::new(),
        encoder: Some(
            match backend {
                Backend::IntelQsv => "Intel Quick Sync (FFmpeg)",
                Backend::Libx265 => "libx265 (FFmpeg)",
            }
            .into(),
        ),
        warnings: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ffmpeg_command_respects_hevc_depth_and_mode() {
        let mut s = ExportSettings { format: Format::Hevc, ..Default::default() };
        s.hevc_bit_depth = 10;
        s.bitrate_mode = BitrateMode::Crf;
        let args = encoding_args(&s, Backend::IntelQsv, 3840, 2160, "24000/1001", 20000, 30000);
        assert!(args.windows(2).any(|w| w[0] == "-c:v" && w[1] == "hevc_qsv"));
        assert!(args.windows(2).any(|w| w[0] == "-profile:v" && w[1] == "main10"));
        assert!(args.iter().any(|a| a.contains("format=p010le")));
        assert!(args.windows(2).any(|w| w[0] == "-pixel_format" && w[1] == "rgb48le"));
        assert!(args.windows(2).any(|w| w[0] == "-global_quality" && w[1] == "23"));
        assert!(!args.iter().any(|a| a == "-b:v"));
        s.hevc_bit_depth = 8;
        s.bitrate_mode = BitrateMode::Vbr1Pass;
        let args = encoding_args(&s, Backend::Libx265, 1920, 1080, "25/1", 5000, 8000);
        assert!(args.iter().any(|a| a.contains("format=yuv420p")));
        assert!(args.windows(2).any(|w| w[0] == "-profile:v" && w[1] == "main"));
        assert!(args.windows(2).any(|w| w[0] == "-pixel_format" && w[1] == "rgba"));
        assert!(args.windows(2).any(|w| w[0] == "-b:v" && w[1] == "5000k"));
    }
}
