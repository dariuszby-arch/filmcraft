# FilmCraft 0.20.1 — HEVC Main 10 dla MX Linux (x86_64)

## Co zawiera kompilacja

- Natywny FilmCraft na Linux x86_64; brak Wine.
- Eksport H.265/HEVC Main 10, SDR BT.709 przez zewnętrzny FFmpeg.
- Enkoder programowy `libx265` (CPU).
- Intel Quick Sync `hevc_qsv` (GPU) tylko gdy sterowniki i FFmpeg potwierdzają działanie enkodera.
- AppImage lub natywne pliki binarne w alternatywnym archiwum TAR.GZ.

### Ważna uwaga o głębi kolorów

Obecna ścieżka renderowania SDR FilmCraft wytwarza klatki RGBA8,
więc HEVC Main 10 przechowuje sygnał 10-bitowy, ale **nie** odzyskuje
dodatkowej precyzji utraconej przed kodowaniem.

## Uruchomienie AppImage na MX Linux 23

Zainstaluj FFmpeg wraz z `libx265`:

```bash
sudo apt update
sudo apt install ffmpeg
ffmpeg -hide_banner -encoders | grep -E 'libx265|hevc_qsv'
```

Pobierz `filmcraft-0.20.1-linux-x86_64.AppImage` z GitHub Actions,
a następnie:

```bash
chmod +x ./filmcraft-0.20.1-linux-x86_64.AppImage
./filmcraft-0.20.1-linux-x86_64.AppImage
```

Jeżeli AppImage nie znajduje FFmpeg, wskaż jego ścieżkę bezpośrednio:

```bash
FILMCRAFT_FFMPEG=/usr/bin/ffmpeg ./filmcraft-0.20.1-linux-x86_64.AppImage
```

Jeżeli uruchomienie zgłosi błąd FUSE, można wypróbować:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./filmcraft-0.20.1-linux-x86_64.AppImage
```

Na starszych bibliotekach Mesa / systemach bez odpowiedniego sterownika QSV
wybierz `CPU libx265 (FFmpeg)`. Samo wyświetlanie w systemie grafiki Intel
nie gwarantuje sprzętowego kodowania HEVC 10-bit.

## Wybór eksportu

W FilmCraft: **Export → H.265 (HEVC) → Profile Main 10 (10-bit SDR)**,
**Encoder: CPU libx265 (FFmpeg)** albo Intel Quick Sync, jeśli próba wykrywania
enkodera powiodła się. Zachowaj BT.709 i MP4.

Po eksporcie sprawdź:

```bash
ffprobe -v error -select_streams v:0 -show_entries stream=codec_name,profile,pix_fmt,color_space,color_primaries,color_transfer -of default=nw=1 "wynik.mp4"
```

Dla LG OLED C1 oczekiwane: `codec_name=hevc`, `profile=Main 10`,
`pix_fmt=yuv420p10le`, `color_space=bt709`.

## Ograniczenia

- Kompilacja GitHub Actions x86_64 nie sprawdza działania GPU Intel HD 630.
- Na Linuxie należy samodzielnie zainstalować FFmpeg (nie jest dołączony do AppImage).
- Zewnętrzny HEVC w tej wersji obsługuje SDR, audio AAC, CRF, CBR i VBR 1-pass.
- HEVC VBR 2-pass i pełny 10-bitowy renderer SDR nadal wymagają rozwinięcia.
- Jest to wersja testowa; wynik GitHub Actions nie zastępuje próby na MX Linux.
