# FilmCraft 0.20.1-DW1 – HEVC Main 10 Edition

**Wydanie DW1 przetestowane przez użytkownika na Windows i MX Linux.** Zawiera działającą ścieżkę eksportu H.265/HEVC poprzez FFmpeg. Oparte na FilmCraft 0.20.1; aplikacja może nadal pokazywać bazowy numer 0.20.1 w oknie „O programie”.

## Co nowego

- **HEVC Main (8-bit)** oraz **HEVC Main 10 (10-bit)** dla SDR BT.709.
- **Intel Quick Sync (hevc_qsv)** – kodowanie sprzętowe, gdy Intel GPU, sterownik i FFmpeg je obsługują. Potwierdzone na Windows z Intel Iris Xe.
- **libx265 (CPU)** – programowe kodowanie H.265, alternatywa również na Linux.
- Klatki trafiają do enkodera przez potok FFmpeg bez pośredniego zapisu stratnego H.264.
- Zgodność kontenera MP4: `hvc1`, `faststart` i sygnalizacja kolorów BT.709.
- Dźwięk AAC jest łączony bez dodatkowego przekodowania.
- **Windows:** FFmpeg nie otwiera dodatkowego okna terminala podczas kodowania.
- **Linux x86_64:** natywny program bez Wine w wersji AppImage i w archiwum TAR.GZ.

## Pliki do pobrania

| System | Plik |
| --- | --- |
| Windows x64 | `FilmCraft-0.20.1-DW1-Windows-x64.zip` |
| Linux x86_64, w tym MX Linux | `FilmCraft-0.20.1-DW1-Linux-x86_64.AppImage` |
| Linux x86_64, pliki przenośne | `FilmCraft-0.20.1-DW1-Linux-x86_64.tar.gz` |

Wszystkie pliki mają sumy SHA-256 w `SHA256SUMS.txt`.

## Windows – uruchomienie

Rozpakuj ZIP i uruchom `filmcraft.exe`. Zainstaluj FFmpeg zawierający `hevc_qsv` i/lub `libx265`. Umieść `ffmpeg.exe` obok programu albo dodaj FFmpeg do `PATH`. Program potrafi także skorzystać z `FILMCRAFT_FFMPEG`.

## MX Linux – uruchomienie

Zainstaluj FFmpeg:

```bash
sudo apt update
sudo apt install ffmpeg
```

Następnie pobierz AppImage:

```bash
chmod +x FilmCraft-0.20.1-DW1-Linux-x86_64.AppImage
./FilmCraft-0.20.1-DW1-Linux-x86_64.AppImage
```

Jeśli AppImage nie uruchamia się ze względu na FUSE:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./FilmCraft-0.20.1-DW1-Linux-x86_64.AppImage
```

Jeśli FFmpeg nie jest wykrywany, uruchom z `FILMCRAFT_FFMPEG=/usr/bin/ffmpeg`. Intel QSV na Linux wymaga osobnego potwierdzenia zgodności sterowników; zawsze można wybrać `libx265`.

## Eksport zgodny z LG OLED C1

Wybierz w FilmCraft: **Export → H.265 (HEVC) → Main 10 → Intel Quick Sync (FFmpeg)** (gdy dostępny) albo **CPU libx265 (FFmpeg)**. Ustaw MP4 / SDR BT.709.

Dla sprawdzenia parametrów wyjścia:

```bash
ffprobe -v error -select_streams v:0 -show_entries stream=codec_name,profile,pix_fmt,color_space,color_primaries,color_transfer -of default=nw=1 "wynik.mp4"
```

## Ograniczenia i informacje techniczne

- FFmpeg jest **zewnętrzną zależnością** i nie został dołączony do paczek.
- Eksport HEVC Main 10 produkuje 10-bitowy strumień, jednak obecny renderer SDR FilmCraft dostarcza klatki **RGBA8**; nie jest to jeszcze pełny 10-bitowy tor obróbki.
- Ścieżka FFmpeg obsługuje SDR BT.709, AAC, CRF, CBR oraz VBR 1-pass; nie obsługuje jeszcze HEVC 2-pass ani pełnej ścieżki HDR10.
- Kompilacje Windows i Linux z GitHub Actions zakończyły się sukcesem, a użytkownik potwierdził działanie na obu systemach. Nie wszystkie ogólne kontrole projektu przeszły: otwarte pozostają uwagi dotyczące formatowania Rust i lintowania skryptu Linux.
- Wydanie jest oznaczeniem sprawdzonej gałęzi rozwojowej, nie zmianą oryginalnego upstream `storytold/filmcraft`.

**Kod źródłowy:** `fbc5d6365776e514dd33a1feaf556b5cfb358736` (`hevc-main10-intel-qsv`).

Wersja przygotowana jako rozwinięcie FilmCraft 0.20.1.
