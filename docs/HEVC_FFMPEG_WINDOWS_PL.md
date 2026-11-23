# FilmCraft — HEVC Main 10 SDR (Windows / Intel Iris Xe)

Zmiany przygotowane na bazie FilmCraft 0.20.1 (`filmcraft-main.zip`). Jest to zmodyfikowany **kod źródłowy** do skompilowania, **nie gotowy EXE**.

## Funkcje dodane

- Eksport HEVC przez **Intel Quick Sync** (`hevc_qsv`) albo **CPU libx265**. Przy Auto program najpierw sprawdza uruchomienie testowego kodowania QSV Main 10, a gdy nie działa, wybiera `libx265`.
- Oddzielny wybór **Main (8-bit)** i **Main 10 (10-bit)** dla materiału SDR Rec.709. Po ręcznym przełączeniu formatu na HEVC w GUI domyślnie proponowany jest 10-bit.
- Przekazanie klatek RGBA renderowanych przez FilmCraft **bezpośrednio do FFmpeg przez stdin**. Brak drugiego stratnego kodowania obrazu i brak pliku pośredniego H.264.
- Różne tryby sterowania bitrate: **CBR, VBR 1-pass, CRF/constant quality**. Dwuprzebiegowy HEVC jeszcze nie jest zaimplementowany i pozostaje niedostępny.
- Zapis informacji BT.709 zarówno w MP4, jak i strumieniu HEVC (VUI); `hvc1` dla zgodności z LG C1.
- Audio z normalnej ścieżki kodowania FilmCraft jest zapisywane oddzielnie do M4A, a potem łączone metodą `-c copy` (bez ponownej kompresji AAC).
- Kopiowanie do docelowego pliku po pełnym powodzeniu kodowania i remuksowania; pliki tymczasowe są usuwane także po błędzie.

## Wymagania

- Rust w wersji przynajmniej **1.95** (projekt `rust-version = "1.95"`).
- Windows: Visual Studio Build Tools z komponentem **Desktop development with C++** i docelową platformą Windows SDK; FFmpeg z `libx265` i/lub `hevc_qsv`.
- FilmCraft odnajduje FFmpeg w kolejności: `FILMCRAFT_FFMPEG`, `ffmpeg.exe` przy `filmcraft.exe`, `ffmpeg/bin/ffmpeg.exe` pod katalogiem aplikacji, globalny `PATH`. W przypadku paczki przenośnej zaleca się umieszczenie `ffmpeg.exe` przy pliku programu.

## Kompilacja na Windows

Uruchom z CMD w katalogu projektu:

```bat
cargo build --release -p filmcraft
```

Gotowy plik po udanej kompilacji: `target\\release\\filmcraft.exe`. Alternatywnie uruchom `buduj_HEVC_Windows.cmd`.

## Użycie

W FilmCraft wejdź do **Export → Format: H.265 (HEVC) → Video** i wybierz:

- **Profile: Main 10 (10-bit SDR)**
- **Encoder: Intel Quick Sync (FFmpeg)** lub **Auto (Intel QSV / CPU)**
- **Bitrate Encoding: CRF (constant quality)** albo VBR 1-pass
- MP4, SDR / BT.709, oryginalna rozdzielczość i FPS.

Po eksporcie możesz potwierdzić parametry:

```bat
ffprobe -v error -select_streams v:0 -show_entries stream=codec_name,profile,pix_fmt,color_space,color_primaries,color_transfer -of default=nw=1 "wynik.mp4"
```

Oczekiwane dla LG C1: `codec_name=hevc`, `profile=Main 10`, `pix_fmt=yuv420p10le`, `bt709`.

## Istotne ograniczenia pierwszej implementacji

- Programowy renderer SDR FilmCraft przekazuje klatki **RGBA8** — eksport HEVC Main 10 zapisuje 10-bitowy strumień, ale **nie odzyskuje precyzji barw wcześniej sprowadzonej do 8 bitów**. Prawdziwa 10-bitowa ścieżka od renderera do kodera wymaga dalszego rozwinięcia.
- Wewnętrzny proces FFmpeg obsługuje na razie SDR BT.709, progresywne wideo, kwadratowe piksele i audio AAC. Przy sekwencji HDR zaznacz eksport SDR; nie jest to nowa ścieżka HDR10/PQ/HLG.
- HEVC 2-pass, FLAC w HEVC i zapis do pamięci (web/OutputSink) nie są obsługiwane w tej ścieżce. Na komputerach z NVENC i na macOS nadal można wybrać dotychczasowy enkoder platformowy.
- Eksport audio odbywa się w osobnym etapie, więc anulowanie w jego trakcie nie jest jeszcze natychmiastowe. Metadane niestandardowe kontenera nie są w pełni odwzorowane przez zewnętrzne FFmpeg.
- Nie uruchomiono `cargo check`, `cargo fmt` ani testów całego programu w środowisku przygotowania (brak kompilatora Rust i pobierania zależności). Test potwierdził poprawność komendy FFmpeg oraz profilu Main 10, tagu BT.709 i remuksowania AAC, **nie działania skompilowanego programu**.

## Kompilacja automatyczna na GitHub Actions

Dodano `.github/workflows/hevc-windows-build.yml`. Po umieszczeniu tego kodu w **osobnym forku FilmCraft** na gałęzi `hevc-main10-intel-qsv` GitHub Actions sam uruchamia kompilację Windows x64 (`cargo test` HEVC i `cargo build --release`). Gotowy plik `filmcraft.exe` jest publikowany jako **Actions → run → Artifacts → FilmCraft-HEVC-Windows-x64**.

**Uwaga:** workflow może wymagać włączenia Actions dla forkowanego repozytorium. Kompilacja na hostowanej maszynie Windows nie dowodzi działania Intel QSV (runner nie ma Intel Iris Xe); test sprzętowy należy wykonać na komputerze docelowym. Artifact nie zawiera `ffmpeg.exe`, które trzeba zapewnić we własnym zakresie zgodnie z instrukcją wyżej. Kompilacja nie wymaga certyfikatu podpisu cyfrowego.
